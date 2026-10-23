// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text};

fn find(layout_box: &LayoutBox, tag: &str, found: &mut Vec<Rect>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if matches!(style.content, Content::Element(e) if e.tag_name == tag) {
            found.push(layout_box.dimensions.border_box());
        }
    }
    for child in &layout_box.children {
        find(child, tag, found);
    }
}

fn target(body: &str) -> Rect {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 300px; position: relative }} section {{ height: 10px }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, "section", &mut found);
    found[0]
}

fn boxed(container: &str, section: &str) -> Rect {
    target(&format!(
        "<div id=a style=\"{}\"><section style=\"{}\"></section></div>",
        container, section
    ))
}

#[test]
fn ltr_over_constrained_boxes_keep_the_left_margin() {
    let found = boxed("", "width: 100px; margin: 0 10px");
    assert_eq!(found.x, 10.0);
    assert_eq!(found.width, 100.0);
}

#[test]
fn rtl_over_constrained_boxes_keep_the_right_margin() {
    let found = boxed("direction: rtl", "width: 100px; margin: 0 10px");
    assert_eq!(found.x, 190.0);
    assert_eq!(found.width, 100.0);
}

#[test]
fn the_containing_block_decides() {
    let found = boxed("", "direction: rtl; width: 100px; margin: 0 10px");
    assert_eq!(found.x, 10.0);
    let found = boxed(
        "direction: rtl",
        "direction: ltr; width: 100px; margin: 0 10px",
    );
    assert_eq!(found.x, 190.0);
}

#[test]
fn auto_widths_are_unchanged() {
    let found = boxed("direction: rtl", "margin: 0 10px");
    assert_eq!(found.x, 10.0);
    assert_eq!(found.width, 280.0);
}

#[test]
fn auto_margins_still_center() {
    let found = boxed("direction: rtl", "width: 100px; margin: 0 auto");
    assert_eq!(found.x, 100.0);
}

#[test]
fn an_auto_left_margin_pushes_the_box_right() {
    let found = boxed("direction: rtl", "width: 100px; margin-left: auto");
    assert_eq!(found.x, 200.0);
    let found = boxed("", "width: 100px; margin-left: auto");
    assert_eq!(found.x, 200.0);
}

#[test]
fn an_auto_right_margin_keeps_the_box_left() {
    let found = boxed("direction: rtl", "width: 100px; margin-right: auto");
    assert_eq!(found.x, 0.0);
}

#[test]
fn rtl_overflow_goes_to_the_left() {
    let found = boxed("direction: rtl", "min-width: 400px");
    assert_eq!(found.x, -100.0);
    assert_eq!(found.width, 400.0);
    let found = boxed("", "min-width: 400px");
    assert_eq!(found.x, 0.0);
}

#[test]
fn max_width_is_resolved_in_the_direction_too() {
    let found = boxed("direction: rtl", "max-width: 100px; margin: 0 10px");
    assert_eq!(found.x, 190.0);
}

#[test]
fn over_constrained_absolute_boxes_use_the_right_edge() {
    let rtl = boxed(
        "direction: rtl",
        "position: absolute; left: 0; right: 0; width: 50px",
    );
    assert_eq!(rtl.x, 250.0);
    let ltr = boxed("", "position: absolute; left: 0; right: 0; width: 50px");
    assert_eq!(ltr.x, 0.0);
}

#[test]
fn absolute_boxes_with_one_offset_do_not_change() {
    let found = boxed(
        "direction: rtl",
        "position: absolute; left: 20px; width: 50px",
    );
    assert_eq!(found.x, 20.0);
}
