// SPDX-License-Identifier: GPL-3.0-only

use crate::color::Rgba;
use crate::style::StrokeStyle;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Edge {
    pub width: f32,
    pub rank: u8,
    pub hidden: bool,
    pub priority: u8,
    pub stroke: StrokeStyle,
    pub color: Option<Rgba>,
}

impl Edge {
    pub const NONE: Edge = Edge {
        width: 0.0,
        rank: 0,
        hidden: false,
        priority: 0,
        stroke: StrokeStyle::Solid,
        color: None,
    };

    pub fn from_source(self, priority: u8) -> Edge {
        Edge { priority, ..self }
    }

    pub fn resolve(first: Edge, second: Edge) -> Edge {
        Edge::resolve_all(&[first, second])
    }

    pub fn resolve_all(edges: &[Edge]) -> Edge {
        if edges.iter().any(|edge| edge.hidden) {
            return Edge::NONE;
        }
        let mut winner = Edge::NONE;
        for edge in edges {
            let key = |edge: &Edge| (edge.width, edge.rank, edge.priority);
            if edge.width > 0.0 && (winner.width == 0.0 || key(edge) > key(&winner)) {
                winner = *edge;
            }
        }
        winner
    }
}
