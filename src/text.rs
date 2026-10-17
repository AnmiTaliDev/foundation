// SPDX-License-Identifier: GPL-3.0-only

use fontdue::{Font, FontSettings};

const DEFAULT_FONT_DATA: &[u8] = include_bytes!("../assets/DejaVuSans.ttf");

pub fn default_font() -> Font {
    Font::from_bytes(DEFAULT_FONT_DATA, FontSettings::default())
        .expect("embedded font must be valid")
}

pub fn text_width(font: &Font, text: &str, font_size: f32) -> f32 {
    let mut width = 0.0;
    for c in text.chars() {
        width += font.metrics(c, font_size).advance_width;
    }
    width
}

pub fn line_height(font: &Font, font_size: f32) -> f32 {
    match font.horizontal_line_metrics(font_size) {
        Some(metrics) => metrics.new_line_size,
        None => font_size * 1.2,
    }
}

pub fn ascent(font: &Font, font_size: f32) -> f32 {
    match font.horizontal_line_metrics(font_size) {
        Some(metrics) => metrics.ascent,
        None => font_size,
    }
}

pub fn descent(font: &Font, font_size: f32) -> f32 {
    match font.horizontal_line_metrics(font_size) {
        Some(metrics) => -metrics.descent,
        None => font_size * 0.2,
    }
}
