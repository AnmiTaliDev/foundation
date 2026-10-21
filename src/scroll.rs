// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::layout::{BoxKind, LayoutBox};
use crate::style::{Content, Position};
use crate::text::Fonts;

pub type Offsets = HashMap<usize, (f32, f32)>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scroller {
    pub offset: (f32, f32),
    pub max: (f32, f32),
    pub axes: (bool, bool),
}

impl LayoutBox<'_> {
    pub fn scroll_size(&self, fonts: &Fonts) -> (f32, f32) {
        let padding = self.dimensions.padding_box();
        let content = self.dimensions.content;
        let mut extent = (padding.x + padding.width, content.y + content.height);
        self.extend_overflow(fonts, &mut extent);
        let (right, bottom) = extent;
        (
            right - padding.x,
            (bottom + self.dimensions.padding.bottom - padding.y).max(padding.height),
        )
    }

    pub fn apply_scrolls(&mut self, offsets: &Offsets, fonts: &Fonts) -> Vec<Scroller> {
        let mut scrollers = Vec::new();
        self.scroll_walk(offsets, fonts, &mut scrollers);
        scrollers
    }

    fn scroll_axes(&self) -> Option<(bool, bool)> {
        let BoxKind::Block(style) = self.kind else {
            return None;
        };
        let is_root = matches!(
            style.content,
            Content::Element(element) if element.tag_name == "html" || element.tag_name == "body"
        );
        let axes = style.scroll_axes();
        (!is_root && (axes.0 || axes.1)).then_some(axes)
    }

    fn scroll_walk(&mut self, offsets: &Offsets, fonts: &Fonts, scrollers: &mut Vec<Scroller>) {
        let container = self.scroll_axes().map(|axes| {
            let padding = self.dimensions.padding_box();
            let (width, height) = self.scroll_size(fonts);
            let max = (
                if axes.0 {
                    (width - padding.width).max(0.0)
                } else {
                    0.0
                },
                if axes.1 {
                    (height - padding.height).max(0.0)
                } else {
                    0.0
                },
            );
            let index = scrollers.len();
            let wanted = offsets.get(&index).copied().unwrap_or((0.0, 0.0));
            let offset = (wanted.0.clamp(0.0, max.0), wanted.1.clamp(0.0, max.1));
            scrollers.push(Scroller { offset, max, axes });
            offset
        });
        for child in &mut self.children {
            child.scroll_walk(offsets, fonts, scrollers);
        }
        if let Some((x, y)) = container {
            if x != 0.0 || y != 0.0 {
                self.shift_content(-x, -y);
            }
        }
    }

    fn shift_content(&mut self, dx: f32, dy: f32) {
        let content = self.dimensions.content;
        self.translate(dx, dy);
        self.dimensions.content = content;
    }

    pub fn scrollers_at(&self, x: f32, y: f32) -> Vec<usize> {
        let mut hits = Vec::new();
        let mut index = 0;
        self.hit_walk(x, y, true, &mut index, &mut hits);
        hits
    }

    fn hit_walk(&self, x: f32, y: f32, visible: bool, index: &mut usize, hits: &mut Vec<usize>) {
        let mut visible = visible;
        if self.scroll_axes().is_some() {
            if visible && contains(self, x, y) {
                hits.push(*index);
            } else {
                visible = false;
            }
            *index += 1;
        } else if let BoxKind::Block(style) = self.kind {
            if style.establishes_clip() && !contains(self, x, y) {
                visible = false;
            }
        }
        for child in &self.children {
            child.hit_walk(x, y, visible, index, hits);
        }
    }

    fn extend_overflow(&self, fonts: &Fonts, extent: &mut (f32, f32)) {
        for fragment in &self.fragments {
            let width = fonts.width(fragment.face, &fragment.text, fragment.font_size)
                + fragment.letter_spacing * fragment.text.chars().count() as f32;
            reach(
                extent,
                fragment.x + width,
                fragment.baseline + fonts.descent(fragment.face, fragment.font_size),
            );
        }
        for image in &self.images {
            reach(
                extent,
                image.rect.x + image.rect.width,
                image.rect.y + image.rect.height,
            );
        }
        for inline_box in &self.inline_boxes {
            let rect = inline_box.rect;
            reach(extent, rect.x + rect.width, rect.y + rect.height);
        }
        for child in &self.children {
            let mut clipped = false;
            if let BoxKind::Block(style) = child.kind {
                if style.position() == Position::Fixed {
                    continue;
                }
                clipped = style.establishes_clip();
            }
            let border = child.dimensions.border_box();
            reach(extent, border.x + border.width, border.y + border.height);
            if !clipped {
                child.extend_overflow(fonts, extent);
            }
        }
    }
}

fn reach(extent: &mut (f32, f32), x: f32, y: f32) {
    extent.0 = extent.0.max(x);
    extent.1 = extent.1.max(y);
}

fn contains(layout_box: &LayoutBox, x: f32, y: f32) -> bool {
    let rect = layout_box.dimensions.padding_box();
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
