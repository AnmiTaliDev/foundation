// SPDX-License-Identifier: GPL-3.0-only

pub mod css;
pub mod dom;
pub mod entities;
pub mod html;
pub mod inline;
pub mod layout;
pub mod paint;
pub mod style;
pub mod text;

use fontdue::Font;

pub struct Page {
    document: dom::Node,
    stylesheet: css::Stylesheet,
    font: Font,
}

pub struct Frame {
    pub canvas: paint::Canvas,
    pub content_height: f32,
    pub scroll_y: f32,
}

impl Page {
    pub fn new(source: &str) -> Page {
        let document = html::parse(source);
        let stylesheet = style::stylesheet_for_document(&document);
        Page {
            document,
            stylesheet,
            font: text::default_font(),
        }
    }

    pub fn render(&self, width: usize, height: usize, scroll_y: f32) -> Frame {
        match style::style_tree(&self.document, &self.stylesheet) {
            Some(styled_root) => {
                let layout_root = layout::layout_tree(&styled_root, width as f32, &self.font);
                let content_height = layout_root.dimensions.margin_box().height;
                let max_scroll = (content_height - height as f32).max(0.0);
                let scroll_y = scroll_y.clamp(0.0, max_scroll);
                Frame {
                    canvas: paint::paint(&layout_root, width, height, scroll_y, &self.font),
                    content_height,
                    scroll_y,
                }
            }
            None => Frame {
                canvas: paint::Canvas::new(width, height),
                content_height: 0.0,
                scroll_y: 0.0,
            },
        }
    }
}

pub fn render(source: &str, width: usize, height: usize) -> paint::Canvas {
    Page::new(source).render(width, height, 0.0).canvas
}
