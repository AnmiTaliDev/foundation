// SPDX-License-Identifier: GPL-3.0-only

use crate::color;
use crate::css::Color;
use crate::media::split_terms;
use crate::transform::{parse_amount, parse_angle, Amount};

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    pub kind: Kind,
    pub stops: Vec<Stop>,
    pub repeating: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    Linear(Direction),
    Radial(Radial),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Direction {
    Angle(f32),
    Corner(i8, i8),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Radial {
    pub circle: bool,
    pub extent: Extent,
    pub size: Option<(Amount, Amount)>,
    pub center: (Amount, Amount),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Extent {
    ClosestSide,
    FarthestSide,
    ClosestCorner,
    FarthestCorner,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Stop {
    pub color: Color,
    pub alpha: u8,
    pub position: Option<Amount>,
}

pub struct Prepared {
    geometry: Geometry,
    stops: Vec<(f32, [f32; 4])>,
    repeating: bool,
}

enum Geometry {
    Linear {
        center: (f32, f32),
        direction: (f32, f32),
        length: f32,
    },
    Radial {
        center: (f32, f32),
        radii: (f32, f32),
    },
}

pub(crate) fn split_top_level(text: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(text[start..index].trim());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(text[start..].trim());
    parts
}

pub fn parse(text: &str) -> Option<Gradient> {
    let lower = text.trim().to_ascii_lowercase();
    let open = lower.find('(')?;
    let inner = lower[open + 1..].strip_suffix(')')?;
    let name = &lower[..open];
    let (radial, repeating) = match name {
        "linear-gradient" => (false, false),
        "repeating-linear-gradient" => (false, true),
        "radial-gradient" => (true, false),
        "repeating-radial-gradient" => (true, true),
        _ => return None,
    };
    let parts = split_top_level(inner);
    let first_is_stop = parse_stop(parts[0]).is_some();
    let (kind, stop_parts) = if first_is_stop {
        let kind = if radial {
            Kind::Radial(default_radial())
        } else {
            Kind::Linear(Direction::Angle(std::f32::consts::PI))
        };
        (kind, &parts[..])
    } else if radial {
        (Kind::Radial(parse_radial(parts[0])?), &parts[1..])
    } else {
        (Kind::Linear(parse_direction(parts[0])?), &parts[1..])
    };
    let mut stops = Vec::new();
    for part in stop_parts {
        stops.extend(parse_stop(part)?);
    }
    if stops.len() < 2 {
        return None;
    }
    Some(Gradient {
        kind,
        stops,
        repeating,
    })
}

fn default_radial() -> Radial {
    Radial {
        circle: false,
        extent: Extent::FarthestCorner,
        size: None,
        center: (Amount::Percent(50.0), Amount::Percent(50.0)),
    }
}

fn parse_direction(text: &str) -> Option<Direction> {
    if let Some(sides) = text.strip_prefix("to ") {
        let (mut horizontal, mut vertical) = (0i8, 0i8);
        for side in sides.split_whitespace() {
            match side {
                "left" if horizontal == 0 => horizontal = -1,
                "right" if horizontal == 0 => horizontal = 1,
                "top" if vertical == 0 => vertical = -1,
                "bottom" if vertical == 0 => vertical = 1,
                _ => return None,
            }
        }
        if horizontal == 0 && vertical == 0 {
            return None;
        }
        return Some(Direction::Corner(horizontal, vertical));
    }
    parse_angle(text).map(Direction::Angle)
}

fn position_keyword(word: &str) -> Option<(Option<char>, Amount)> {
    match word {
        "left" => Some((Some('x'), Amount::Percent(0.0))),
        "right" => Some((Some('x'), Amount::Percent(100.0))),
        "top" => Some((Some('y'), Amount::Percent(0.0))),
        "bottom" => Some((Some('y'), Amount::Percent(100.0))),
        "center" => Some((None, Amount::Percent(50.0))),
        _ => parse_amount(word).map(|amount| (None, amount)),
    }
}

fn parse_radial(text: &str) -> Option<Radial> {
    let mut radial = default_radial();
    let (shape_part, position_part) = match text.find("at ") {
        Some(index) if index == 0 || text.as_bytes()[index - 1] == b' ' => {
            (&text[..index], Some(&text[index + 3..]))
        }
        _ => (text, None),
    };
    let mut sizes = Vec::new();
    for word in shape_part.split_whitespace() {
        match word {
            "circle" => radial.circle = true,
            "ellipse" => radial.circle = false,
            "closest-side" => radial.extent = Extent::ClosestSide,
            "farthest-side" => radial.extent = Extent::FarthestSide,
            "closest-corner" => radial.extent = Extent::ClosestCorner,
            "farthest-corner" => radial.extent = Extent::FarthestCorner,
            _ => sizes.push(parse_amount(word)?),
        }
    }
    radial.size = match (radial.circle, sizes.as_slice()) {
        (_, []) => None,
        (true, [radius]) => Some((*radius, *radius)),
        (false, [x, y]) => Some((*x, *y)),
        _ => return None,
    };
    if let Some(position) = position_part {
        let words: Vec<&str> = position.split_whitespace().collect();
        let (x, y) = match words.as_slice() {
            [one] => {
                let (axis, amount) = position_keyword(one)?;
                let center = Amount::Percent(50.0);
                if axis == Some('y') {
                    (center, amount)
                } else {
                    (amount, center)
                }
            }
            [first, second] => {
                let first = position_keyword(first)?;
                let second = position_keyword(second)?;
                if first.0 == Some('y') || second.0 == Some('x') {
                    (second.1, first.1)
                } else {
                    (first.1, second.1)
                }
            }
            _ => return None,
        };
        radial.center = (x, y);
    }
    Some(radial)
}

fn parse_stop(text: &str) -> Option<Vec<Stop>> {
    let terms = split_terms(text);
    let (first, rest) = terms.split_first()?;
    let rgba = color::parse(first)?;
    let mut positions = Vec::new();
    for term in rest {
        positions.push(parse_amount(term)?);
    }
    let stop = |position| Stop {
        color: rgba.color,
        alpha: rgba.alpha,
        position,
    };
    match positions.as_slice() {
        [] => Some(vec![stop(None)]),
        [one] => Some(vec![stop(Some(*one))]),
        [start, end] => Some(vec![stop(Some(*start)), stop(Some(*end))]),
        _ => None,
    }
}

fn amount_px(amount: Amount, base: f32, font_size: f32) -> f32 {
    match amount {
        Amount::Px(px) => px,
        Amount::Em(em) => em * font_size,
        Amount::Percent(percent) => percent / 100.0 * base,
    }
}

impl Gradient {
    pub fn prepare(&self, width: f32, height: f32, font_size: f32) -> Prepared {
        let (geometry, base) = match &self.kind {
            Kind::Linear(direction) => {
                let (dx, dy) = match *direction {
                    Direction::Angle(angle) => (angle.sin(), -angle.cos()),
                    Direction::Corner(horizontal, vertical) => {
                        let x = f32::from(horizontal);
                        let y = f32::from(vertical);
                        if horizontal != 0 && vertical != 0 {
                            let (px, py) = (x * height, y * width);
                            let norm = px.hypot(py).max(1e-6);
                            (px / norm, py / norm)
                        } else {
                            (x, y)
                        }
                    }
                };
                let length = (width * dx).abs() + (height * dy).abs();
                (
                    Geometry::Linear {
                        center: (width / 2.0, height / 2.0),
                        direction: (dx, dy),
                        length: length.max(1e-3),
                    },
                    length.max(1e-3),
                )
            }
            Kind::Radial(radial) => {
                let cx = amount_px(radial.center.0, width, font_size);
                let cy = amount_px(radial.center.1, height, font_size);
                let near = (cx.min(width - cx).abs(), cy.min(height - cy).abs());
                let far = (cx.max(width - cx).abs(), cy.max(height - cy).abs());
                let root2 = std::f32::consts::SQRT_2;
                let radii = match (radial.size, radial.circle, radial.extent) {
                    (Some((x, y)), _, _) => (
                        amount_px(x, width, font_size),
                        amount_px(y, height, font_size),
                    ),
                    (None, true, Extent::ClosestSide) => {
                        let r = near.0.min(near.1);
                        (r, r)
                    }
                    (None, true, Extent::FarthestSide) => {
                        let r = far.0.max(far.1);
                        (r, r)
                    }
                    (None, true, Extent::ClosestCorner) => {
                        let r = near.0.hypot(near.1);
                        (r, r)
                    }
                    (None, true, Extent::FarthestCorner) => {
                        let r = far.0.hypot(far.1);
                        (r, r)
                    }
                    (None, false, Extent::ClosestSide) => near,
                    (None, false, Extent::FarthestSide) => far,
                    (None, false, Extent::ClosestCorner) => (near.0 * root2, near.1 * root2),
                    (None, false, Extent::FarthestCorner) => (far.0 * root2, far.1 * root2),
                };
                let radii = (radii.0.max(1e-3), radii.1.max(1e-3));
                (
                    Geometry::Radial {
                        center: (cx, cy),
                        radii,
                    },
                    radii.0,
                )
            }
        };
        Prepared {
            geometry,
            stops: self.resolve_stops(base, font_size),
            repeating: self.repeating,
        }
    }

    fn resolve_stops(&self, base: f32, font_size: f32) -> Vec<(f32, [f32; 4])> {
        let count = self.stops.len();
        let mut positions: Vec<Option<f32>> = self
            .stops
            .iter()
            .map(|stop| {
                stop.position
                    .map(|amount| amount_px(amount, base, font_size) / base)
            })
            .collect();
        if positions[0].is_none() {
            positions[0] = Some(0.0);
        }
        if positions[count - 1].is_none() {
            positions[count - 1] = Some(1.0);
        }
        let mut highest = f32::MIN;
        for position in positions.iter_mut().flatten() {
            highest = highest.max(*position);
            *position = highest;
        }
        let mut index = 0;
        while index < count {
            if positions[index].is_some() {
                index += 1;
                continue;
            }
            let before = positions[index - 1].unwrap_or(0.0);
            let mut end = index;
            while positions[end].is_none() {
                end += 1;
            }
            let after = positions[end].unwrap_or(1.0);
            let steps = (end - index + 1) as f32;
            for (offset, slot) in positions[index..end].iter_mut().enumerate() {
                *slot = Some(before + (after - before) * (offset + 1) as f32 / steps);
            }
            index = end;
        }
        self.stops
            .iter()
            .zip(positions)
            .map(|(stop, position)| {
                (
                    position.unwrap_or(0.0),
                    [
                        f32::from(stop.color.r),
                        f32::from(stop.color.g),
                        f32::from(stop.color.b),
                        f32::from(stop.alpha),
                    ],
                )
            })
            .collect()
    }
}

impl Prepared {
    pub fn color_at(&self, x: f32, y: f32) -> (Color, u8) {
        let mut t = match self.geometry {
            Geometry::Linear {
                center,
                direction,
                length,
            } => ((x - center.0) * direction.0 + (y - center.1) * direction.1) / length + 0.5,
            Geometry::Radial { center, radii } => {
                (((x - center.0) / radii.0).powi(2) + ((y - center.1) / radii.1).powi(2)).sqrt()
            }
        };
        let first = self.stops[0].0;
        let last = self.stops[self.stops.len() - 1].0;
        if self.repeating && last - first > 1e-6 {
            t = first + (t - first).rem_euclid(last - first);
        }
        let color = self.lookup(t);
        let alpha = color[3];
        if alpha <= 0.0 {
            return (Color { r: 0, g: 0, b: 0 }, 0);
        }
        let channel = |value: f32| (value / alpha * 255.0).round().clamp(0.0, 255.0) as u8;
        (
            Color {
                r: channel(color[0]),
                g: channel(color[1]),
                b: channel(color[2]),
            },
            alpha.round().clamp(0.0, 255.0) as u8,
        )
    }

    fn lookup(&self, t: f32) -> [f32; 4] {
        let premultiplied = |color: [f32; 4]| {
            let alpha = color[3] / 255.0;
            [
                color[0] * alpha,
                color[1] * alpha,
                color[2] * alpha,
                color[3],
            ]
        };
        let (first_t, first) = self.stops[0];
        if t <= first_t {
            return premultiplied(first);
        }
        for pair in self.stops.windows(2) {
            let (start_t, start) = pair[0];
            let (end_t, end) = pair[1];
            if t < end_t {
                let amount = if end_t > start_t {
                    (t - start_t) / (end_t - start_t)
                } else {
                    1.0
                };
                let (start, end) = (premultiplied(start), premultiplied(end));
                let mut mixed = [0.0; 4];
                for channel in 0..4 {
                    mixed[channel] = start[channel] + (end[channel] - start[channel]) * amount;
                }
                return mixed;
            }
        }
        premultiplied(self.stops[self.stops.len() - 1].1)
    }
}
