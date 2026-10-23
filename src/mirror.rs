// SPDX-License-Identifier: GPL-3.0-only

use crate::layout::{BoxKind, LayoutBox};

impl LayoutBox<'_> {
    pub(crate) fn mirror_children(&mut self) {
        let content = self.dimensions.content;
        let sum = 2.0 * content.x + content.width;
        for child in &mut self.children {
            if matches!(child.kind, BoxKind::Block(style) if style.is_out_of_flow()) {
                continue;
            }
            let margin = child.dimensions.margin_box();
            child.translate(sum - 2.0 * margin.x - margin.width, 0.0);
        }
    }
}
