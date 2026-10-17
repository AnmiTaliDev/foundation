// SPDX-License-Identifier: GPL-3.0-only

use crate::layout::Rect;
use crate::style::Clear;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Side {
    Left,
    Right,
}

#[derive(Debug, Default)]
pub struct FloatContext {
    left: Vec<Rect>,
    right: Vec<Rect>,
    top_limit: f32,
}

fn covers(rect: &Rect, y: f32) -> bool {
    rect.y <= y && y < rect.y + rect.height
}

fn overlaps(rect: &Rect, top: f32, bottom: f32) -> bool {
    rect.height > 0.0 && rect.y < bottom && rect.y + rect.height > top
}

impl FloatContext {
    pub fn add(&mut self, side: Side, rect: Rect) {
        self.top_limit = self.top_limit.max(rect.y);
        match side {
            Side::Left => self.left.push(rect),
            Side::Right => self.right.push(rect),
        }
    }

    pub fn top_limit(&self) -> f32 {
        self.top_limit
    }

    pub fn max_bottom(&self) -> f32 {
        let mut bottom = f32::NEG_INFINITY;
        for rect in self.left.iter().chain(self.right.iter()) {
            bottom = bottom.max(rect.y + rect.height);
        }
        bottom
    }

    pub fn clearance(&self, clear: Clear) -> f32 {
        let mut bottom = f32::NEG_INFINITY;
        if clear == Clear::Left || clear == Clear::Both {
            for rect in &self.left {
                bottom = bottom.max(rect.y + rect.height);
            }
        }
        if clear == Clear::Right || clear == Clear::Both {
            for rect in &self.right {
                bottom = bottom.max(rect.y + rect.height);
            }
        }
        bottom
    }

    pub fn bounds_at(&self, y: f32, min_x: f32, max_x: f32) -> (f32, f32) {
        let mut left = min_x;
        let mut right = max_x;
        for rect in self.left.iter().filter(|rect| covers(rect, y)) {
            left = left.max(rect.x + rect.width);
        }
        for rect in self.right.iter().filter(|rect| covers(rect, y)) {
            right = right.min(rect.x);
        }
        (left, right)
    }

    pub fn is_blocked_at(&self, y: f32) -> bool {
        self.left
            .iter()
            .chain(self.right.iter())
            .any(|rect| covers(rect, y))
    }

    pub fn next_clear_after(&self, y: f32) -> f32 {
        let mut bottom = f32::INFINITY;
        for rect in self.left.iter().chain(self.right.iter()) {
            if covers(rect, y) {
                bottom = bottom.min(rect.y + rect.height);
            }
        }
        bottom
    }

    pub fn place(
        &self,
        side: Side,
        width: f32,
        height: f32,
        min_y: f32,
        min_x: f32,
        max_x: f32,
    ) -> (f32, f32) {
        let mut y = min_y;
        loop {
            let bottom = y + height;
            let mut left = min_x;
            let mut right = max_x;
            let mut blocking_bottom = f32::INFINITY;
            let mut any = false;
            for rect in self.left.iter().filter(|rect| overlaps(rect, y, bottom)) {
                left = left.max(rect.x + rect.width);
                blocking_bottom = blocking_bottom.min(rect.y + rect.height);
                any = true;
            }
            for rect in self.right.iter().filter(|rect| overlaps(rect, y, bottom)) {
                right = right.min(rect.x);
                blocking_bottom = blocking_bottom.min(rect.y + rect.height);
                any = true;
            }
            if !any || right - left >= width {
                let x = match side {
                    Side::Left => left,
                    Side::Right => right - width,
                };
                return (x, y);
            }
            y = blocking_bottom;
        }
    }
}
