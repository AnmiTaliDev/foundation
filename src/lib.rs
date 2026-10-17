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

pub fn render(source: &str, width: usize, height: usize) -> paint::Canvas {
    let document = html::parse(source);
    let stylesheet = style::stylesheet_for_document(&document);
    match style::style_tree(&document, &stylesheet) {
        Some(styled_root) => {
            let font = text::default_font();
            let layout_root = layout::layout_tree(&styled_root, width as f32, &font);
            paint::paint(&layout_root, width, height, &font)
        }
        None => paint::Canvas::new(width, height),
    }
}
