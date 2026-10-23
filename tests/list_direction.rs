// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::style::Content;
use foundation::{html, style, text};

fn find_items<'a>(layout_box: &'a LayoutBox<'a>, found: &mut Vec<&'a LayoutBox<'a>>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if matches!(style.content, Content::Element(e) if e.tag_name == "li") {
            found.push(layout_box);
        }
    }
    for child in &layout_box.children {
        find_items(child, found);
    }
}

fn marker_edges(list: &str) -> (f32, f32, f32) {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 300px }}</style></head><body><div id=a>{}</div></body></html>",
        list
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut items = Vec::new();
    find_items(&layout_root, &mut items);
    let item = items[0];
    let fragment = &item.fragments[0];
    let width = fonts.width(fragment.face, &fragment.text, fragment.font_size);
    let content = item.dimensions.content;
    (fragment.x, fragment.x + width, content.x + content.width)
}

#[test]
fn ltr_markers_sit_left_of_the_content() {
    let (_, right, _) = marker_edges("<ul><li>text</li></ul>");
    let content_left = 40.0;
    assert!((right - content_left).abs() < 0.01, "{}", right);
}

#[test]
fn rtl_markers_sit_right_of_the_content() {
    let (left, _, content_right) = marker_edges("<ul style=\"direction: rtl\"><li>text</li></ul>");
    assert!(
        (left - content_right).abs() < 0.01,
        "{} {}",
        left,
        content_right
    );
    assert!((content_right - 260.0).abs() < 0.01);
}

#[test]
fn ordered_list_numbers_move_too() {
    let (left, _, content_right) = marker_edges("<ol style=\"direction: rtl\"><li>text</li></ol>");
    assert!((left - content_right).abs() < 0.01);
}

#[test]
fn a_single_li_in_ltr_is_unchanged_by_the_marker_fix() {
    let (left, right, _) = marker_edges("<ul><li>text</li></ul>");
    assert!(left < right);
    assert!(right <= 40.0 + 0.01);
}
