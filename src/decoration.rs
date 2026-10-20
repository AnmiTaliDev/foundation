// SPDX-License-Identifier: GPL-3.0-only

use crate::color::Rgba;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Line {
    Underline,
    Overline,
    LineThrough,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LineStyle {
    Solid,
    Double,
    Dotted,
    Dashed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Decoration {
    pub line: Line,
    pub style: LineStyle,
    pub color: Rgba,
    pub thickness: Option<f32>,
}

pub fn parse_lines(text: &str) -> Option<Vec<Line>> {
    let mut lines = Vec::new();
    for token in text.split_whitespace() {
        match token.to_ascii_lowercase().as_str() {
            "underline" => lines.push(Line::Underline),
            "overline" => lines.push(Line::Overline),
            "line-through" => lines.push(Line::LineThrough),
            "none" if text.split_whitespace().count() == 1 => {}
            _ => return None,
        }
    }
    if lines.is_empty() && text.trim().is_empty() {
        return None;
    }
    Some(lines)
}

pub fn parse_style(text: &str) -> Option<LineStyle> {
    match text.to_ascii_lowercase().as_str() {
        "solid" | "wavy" => Some(LineStyle::Solid),
        "double" => Some(LineStyle::Double),
        "dotted" => Some(LineStyle::Dotted),
        "dashed" => Some(LineStyle::Dashed),
        _ => None,
    }
}
