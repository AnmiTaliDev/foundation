// SPDX-License-Identifier: GPL-3.0-only

use crate::color::Rgba;
use crate::style::StrokeStyle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub width: f32,
    pub rank: u8,
    pub hidden: bool,
    pub stroke: StrokeStyle,
    pub color: Option<Rgba>,
}

impl Edge {
    pub const NONE: Edge = Edge {
        width: 0.0,
        rank: 0,
        hidden: false,
        stroke: StrokeStyle::Solid,
        color: None,
    };

    pub fn resolve(first: Edge, second: Edge) -> Edge {
        if first.hidden || second.hidden {
            return Edge::NONE;
        }
        if second.width > first.width || (second.width == first.width && second.rank > first.rank) {
            second
        } else {
            first
        }
    }
}
