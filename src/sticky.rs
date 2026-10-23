// SPDX-License-Identifier: GPL-3.0-only

use crate::layout::{BoxKind, LayoutBox, Rect};
use crate::style::Position;

impl LayoutBox<'_> {
    pub(crate) fn stick_horizontally(&mut self) {
        let port = (0.0, self.dimensions.margin_box().width);
        self.stick_children(port);
    }

    fn stick_children(&mut self, port: (f32, f32)) {
        let container = self.dimensions.content;
        let port = self.scroll_port().unwrap_or(port);
        for child in &mut self.children {
            let shift = child.sticky_shift(container, port);
            if shift != 0.0 {
                child.translate(shift, 0.0);
            }
            child.stick_children(port);
        }
    }

    fn scroll_port(&self) -> Option<(f32, f32)> {
        let bars = self.scrollbars?;
        let area = self.dimensions.padding_box();
        let bar = if bars.show.1 { bars.width } else { 0.0 };
        let start = if bars.rtl { area.x + bar } else { area.x };
        Some((start, (area.width - bar).max(0.0)))
    }

    fn sticky_shift(&self, container: Rect, port: (f32, f32)) -> f32 {
        let BoxKind::Block(style) = self.kind else {
            return 0.0;
        };
        if style.position() != Position::Sticky {
            return 0.0;
        }
        let border = self.dimensions.border_box();
        let margin_left = style.px("margin-left", port.1);
        let margin_right = style.px("margin-right", port.1);
        let left = style.length_or_auto("left", None, Some(port.1));
        let right = style.length_or_auto("right", None, Some(port.1));
        let from_left = left.map(|left| (port.0 + left - border.x).max(0.0));
        let from_right =
            right.map(|right| (port.0 + port.1 - right - (border.x + border.width)).min(0.0));
        let shift = match (from_left, from_right) {
            (Some(shift), _) if shift > 0.0 => shift,
            (_, Some(shift)) => shift,
            _ => 0.0,
        };
        let farthest = container.x + container.width - (border.x + border.width + margin_right);
        let nearest = container.x - (border.x - margin_left);
        shift.min(farthest).max(nearest)
    }
}
