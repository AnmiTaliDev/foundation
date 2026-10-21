// SPDX-License-Identifier: GPL-3.0-only

use crate::layout::{BoxKind, LayoutBox};
use crate::style::Position;
use crate::text::Fonts;

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
