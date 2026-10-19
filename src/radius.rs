// SPDX-License-Identifier: GPL-3.0-only

use crate::layout::Rect;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Radii {
    pub corners: [(f32, f32); 4],
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Round {
    pub rect: Rect,
    pub radii: Radii,
    pub inverted: bool,
}

impl Radii {
    pub fn is_zero(&self) -> bool {
        self.corners.iter().all(|(x, y)| *x <= 0.0 || *y <= 0.0)
    }

    pub fn fit(self, width: f32, height: f32) -> Radii {
        let [tl, tr, br, bl] = self.corners;
        let sides = [
            (width, tl.0 + tr.0),
            (height, tr.1 + br.1),
            (width, bl.0 + br.0),
            (height, tl.1 + bl.1),
        ];
        let mut factor: f32 = 1.0;
        for (available, needed) in sides {
            if needed > available {
                factor = factor.min(available / needed);
            }
        }
        Radii {
            corners: self.corners.map(|(x, y)| (x * factor, y * factor)),
        }
    }

    pub fn shrink(self, widths: [f32; 4]) -> Radii {
        let [top, right, bottom, left] = widths;
        let [tl, tr, br, bl] = self.corners;
        let reduce = |(x, y): (f32, f32), horizontal: f32, vertical: f32| {
            ((x - horizontal).max(0.0), (y - vertical).max(0.0))
        };
        Radii {
            corners: [
                reduce(tl, left, top),
                reduce(tr, right, top),
                reduce(br, right, bottom),
                reduce(bl, left, bottom),
            ],
        }
    }
}

impl Round {
    pub fn coverage(&self, x: f32, y: f32) -> f32 {
        let inside = (0.5 - self.distance(x, y)).clamp(0.0, 1.0);
        if self.inverted {
            1.0 - inside
        } else {
            inside
        }
    }

    pub fn distance(&self, x: f32, y: f32) -> f32 {
        let rect = &self.rect;
        let right = rect.x + rect.width;
        let bottom = rect.y + rect.height;
        let mut distance = (rect.x - x).max(x - right).max(rect.y - y).max(y - bottom);
        let [tl, tr, br, bl] = self.radii.corners;
        let corners = [
            (tl, rect.x + tl.0, rect.y + tl.1, -1.0, -1.0),
            (tr, right - tr.0, rect.y + tr.1, 1.0, -1.0),
            (br, right - br.0, bottom - br.1, 1.0, 1.0),
            (bl, rect.x + bl.0, bottom - bl.1, -1.0, 1.0),
        ];
        for ((rx, ry), center_x, center_y, side_x, side_y) in corners {
            if rx <= 0.0 || ry <= 0.0 {
                continue;
            }
            let dx = (x - center_x) * side_x;
            let dy = (y - center_y) * side_y;
            if dx > 0.0 && dy > 0.0 {
                let ratio = ((dx / rx).powi(2) + (dy / ry).powi(2)).sqrt();
                distance = (ratio - 1.0) * rx.min(ry);
            }
        }
        distance
    }
}
