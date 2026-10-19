// SPDX-License-Identifier: GPL-3.0-only

use crate::color::{self, Rgba};
use crate::css::split_values;
use crate::gradient;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Dim {
    pub px: f32,
    pub em: f32,
}

impl Dim {
    pub fn resolve(&self, font_size: f32) -> f32 {
        self.px + self.em * font_size
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    pub offset_x: Dim,
    pub offset_y: Dim,
    pub blur: Dim,
    pub spread: Dim,
    pub color: Option<Rgba>,
    pub inset: bool,
}

fn dim(token: &str) -> Option<Dim> {
    if token == "0" {
        return Some(Dim { px: 0.0, em: 0.0 });
    }
    if let Some(number) = token.strip_suffix("px") {
        let px = number.parse::<f32>().ok().filter(|v| v.is_finite())?;
        return Some(Dim { px, em: 0.0 });
    }
    if token.ends_with("rem") {
        return None;
    }
    let number = token.strip_suffix("em")?;
    let em = number.parse::<f32>().ok().filter(|v| v.is_finite())?;
    Some(Dim { px: 0.0, em })
}

fn parse_one(text: &str) -> Option<Shadow> {
    let mut lengths = Vec::new();
    let mut color = None;
    let mut color_set = false;
    let mut inset = false;
    for token in split_values(text) {
        if token.eq_ignore_ascii_case("inset") {
            inset = true;
        } else if token.eq_ignore_ascii_case("currentcolor") {
            color_set = true;
        } else if let Some(length) = dim(token) {
            lengths.push(length);
        } else {
            let parsed = color::parse(token)?;
            if color_set {
                return None;
            }
            color = Some(parsed);
            color_set = true;
        }
    }
    if lengths.len() < 2 || lengths.len() > 4 {
        return None;
    }
    let zero = Dim { px: 0.0, em: 0.0 };
    let blur = lengths.get(2).copied().unwrap_or(zero);
    if blur.px < 0.0 || blur.em < 0.0 {
        return None;
    }
    Some(Shadow {
        offset_x: lengths[0],
        offset_y: lengths[1],
        blur,
        spread: lengths.get(3).copied().unwrap_or(zero),
        color,
        inset,
    })
}

pub fn parse(text: &str) -> Option<Vec<Shadow>> {
    let mut shadows = Vec::new();
    for part in gradient::split_top_level(text) {
        shadows.push(parse_one(part)?);
    }
    Some(shadows)
}

fn erfc(x: f32) -> f32 {
    if x < 0.0 {
        return 2.0 - erfc(-x);
    }
    let t = 1.0 / (1.0 + 0.3275911 * x);
    let polynomial = t
        * (0.254_829_6
            + t * (-0.284_496_72 + t * (1.421_413_8 + t * (-1.453_152_1 + t * 1.061_405_4))));
    polynomial * (-x * x).exp()
}

pub fn blurred_coverage(distance: f32, blur: f32) -> f32 {
    if blur <= 0.0 {
        return (0.5 - distance).clamp(0.0, 1.0);
    }
    let sigma = blur / 2.0;
    0.5 * erfc(distance / (sigma * std::f32::consts::SQRT_2))
}
