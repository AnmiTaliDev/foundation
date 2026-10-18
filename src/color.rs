// SPDX-License-Identifier: GPL-3.0-only

use crate::css::{self, Color};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rgba {
    pub color: Color,
    pub alpha: u8,
}

pub fn parse(text: &str) -> Option<Rgba> {
    let lower = text.trim().to_ascii_lowercase();
    if let Some(hex) = lower.strip_prefix('#') {
        return parse_hex(hex);
    }
    if lower == "transparent" {
        return Some(Rgba {
            color: Color { r: 0, g: 0, b: 0 },
            alpha: 0,
        });
    }
    for name in ["rgba", "rgb"] {
        if let Some(inner) = function_body(&lower, name) {
            return parse_rgb(inner);
        }
    }
    for name in ["hsla", "hsl"] {
        if let Some(inner) = function_body(&lower, name) {
            return parse_hsl(inner);
        }
    }
    css::named_color(&lower).map(|color| Rgba { color, alpha: 255 })
}

fn function_body<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.strip_prefix(name)?
        .strip_prefix('(')?
        .strip_suffix(')')
}

fn parse_hex(hex: &str) -> Option<Rgba> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let pair = |index: usize| u8::from_str_radix(&hex[index..index + 2], 16).ok();
    let single = |index: usize| {
        u8::from_str_radix(&hex[index..index + 1], 16)
            .ok()
            .map(|v| v * 17)
    };
    let (r, g, b, a) = match hex.len() {
        6 => (pair(0)?, pair(2)?, pair(4)?, 255),
        8 => (pair(0)?, pair(2)?, pair(4)?, pair(6)?),
        3 => (single(0)?, single(1)?, single(2)?, 255),
        4 => (single(0)?, single(1)?, single(2)?, single(3)?),
        _ => return None,
    };
    Some(Rgba {
        color: Color { r, g, b },
        alpha: a,
    })
}

fn arguments(inner: &str) -> Option<(Vec<&str>, Option<&str>)> {
    let (main, alpha) = match inner.split_once('/') {
        Some((main, alpha)) => (main, Some(alpha.trim())),
        None => (inner, None),
    };
    let mut parts: Vec<&str> = if main.contains(',') {
        main.split(',').map(str::trim).collect()
    } else {
        main.split_whitespace().collect()
    };
    let mut alpha = alpha;
    if alpha.is_none() && parts.len() == 4 {
        alpha = parts.pop();
    }
    if parts.len() != 3 || parts.iter().any(|part| part.is_empty()) {
        return None;
    }
    Some((parts, alpha))
}

fn parse_alpha(text: Option<&str>) -> Option<u8> {
    let Some(text) = text else {
        return Some(255);
    };
    let fraction = match text.strip_suffix('%') {
        Some(percent) => percent.trim().parse::<f32>().ok()? / 100.0,
        None => text.parse::<f32>().ok()?,
    };
    if !fraction.is_finite() {
        return None;
    }
    Some((fraction.clamp(0.0, 1.0) * 255.0).round() as u8)
}

fn parse_channel(text: &str) -> Option<u8> {
    let value = match text.strip_suffix('%') {
        Some(percent) => percent.trim().parse::<f32>().ok()? * 2.55,
        None => text.parse::<f32>().ok()?,
    };
    if !value.is_finite() {
        return None;
    }
    Some(value.round().clamp(0.0, 255.0) as u8)
}

fn parse_rgb(inner: &str) -> Option<Rgba> {
    let (parts, alpha) = arguments(inner)?;
    Some(Rgba {
        color: Color {
            r: parse_channel(parts[0])?,
            g: parse_channel(parts[1])?,
            b: parse_channel(parts[2])?,
        },
        alpha: parse_alpha(alpha)?,
    })
}

fn parse_hue(text: &str) -> Option<f32> {
    let (number, factor) = if let Some(number) = text.strip_suffix("deg") {
        (number, 1.0)
    } else if let Some(number) = text.strip_suffix("turn") {
        (number, 360.0)
    } else if let Some(number) = text.strip_suffix("rad") {
        (number, 180.0 / std::f32::consts::PI)
    } else {
        (text, 1.0)
    };
    let degrees = number.trim().parse::<f32>().ok()? * factor;
    degrees.is_finite().then(|| degrees.rem_euclid(360.0))
}

fn parse_fraction(text: &str) -> Option<f32> {
    let percent = text.strip_suffix('%')?.trim().parse::<f32>().ok()?;
    percent
        .is_finite()
        .then(|| (percent / 100.0).clamp(0.0, 1.0))
}

fn parse_hsl(inner: &str) -> Option<Rgba> {
    let (parts, alpha) = arguments(inner)?;
    let hue = parse_hue(parts[0])?;
    let saturation = parse_fraction(parts[1])?;
    let lightness = parse_fraction(parts[2])?;
    let chroma = (1.0 - (2.0 * lightness - 1.0).abs()) * saturation;
    let sector = hue / 60.0;
    let second = chroma * (1.0 - (sector % 2.0 - 1.0).abs());
    let (r, g, b) = match sector as u32 {
        0 => (chroma, second, 0.0),
        1 => (second, chroma, 0.0),
        2 => (0.0, chroma, second),
        3 => (0.0, second, chroma),
        4 => (second, 0.0, chroma),
        _ => (chroma, 0.0, second),
    };
    let offset = lightness - chroma / 2.0;
    let channel = |value: f32| ((value + offset) * 255.0).round().clamp(0.0, 255.0) as u8;
    Some(Rgba {
        color: Color {
            r: channel(r),
            g: channel(g),
            b: channel(b),
        },
        alpha: parse_alpha(alpha)?,
    })
}
