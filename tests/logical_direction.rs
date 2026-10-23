// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, Dimensions, LayoutBox};
use foundation::style::Content;
use foundation::{html, style, text};

fn find(layout_box: &LayoutBox, tag: &str, found: &mut Vec<Dimensions>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if matches!(style.content, Content::Element(e) if e.tag_name == tag) {
            found.push(layout_box.dimensions);
        }
    }
    for child in &layout_box.children {
        find(child, tag, found);
    }
}

fn dimensions(head: &str, body: &str, tag: &str) -> Dimensions {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 300px; position: relative }} section {{ height: 10px }} {}</style></head><body>{}</body></html>",
        head, body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, tag, &mut found);
    found[0]
}

fn section(container: &str, style: &str) -> Dimensions {
    dimensions(
        "",
        &format!(
            "<div id=a style=\"{}\"><section style=\"{}\"></section></div>",
            container, style
        ),
        "section",
    )
}

#[test]
fn margin_inline_start_follows_the_container_direction() {
    let ltr = section("", "width: 100px; margin-inline-start: 10px");
    assert_eq!(ltr.border_box().x, 10.0);
    let rtl = section("direction: rtl", "width: 100px; margin-inline-start: 10px");
    assert_eq!(rtl.margin.right, 10.0);
    assert_eq!(rtl.border_box().x, 190.0);
}

#[test]
fn margin_inline_end_is_the_opposite_side() {
    let rtl = section("direction: rtl", "margin-inline-end: 10px");
    assert_eq!(rtl.margin.left, 10.0);
    assert_eq!(rtl.margin.right, 0.0);
    assert_eq!(rtl.border_box().width, 290.0);
    let ltr = section("", "margin-inline-end: 10px");
    assert_eq!(ltr.margin.right, 10.0);
    assert_eq!(ltr.margin.left, 0.0);
}

#[test]
fn the_elements_own_direction_picks_the_physical_side() {
    let rtl = section("", "direction: rtl; padding-inline-start: 10px");
    assert_eq!(rtl.padding.right, 10.0);
    assert_eq!(rtl.padding.left, 0.0);
    let ltr = section("", "padding-inline-start: 10px");
    assert_eq!(ltr.padding.left, 10.0);
    assert_eq!(ltr.padding.right, 0.0);
}

#[test]
fn inherited_direction_applies() {
    let found = dimensions(
        "",
        "<div id=a style=\"direction: rtl\"><div><section style=\"padding-inline-start: 7px\"></section></div></div>",
        "section",
    );
    assert_eq!(found.padding.right, 7.0);
}

#[test]
fn a_later_direction_in_the_same_block_counts() {
    let found = section("", "padding-inline-start: 10px; direction: rtl");
    assert_eq!(found.padding.right, 10.0);
}

#[test]
fn direction_from_another_rule_counts() {
    let found = dimensions(
        "section { direction: rtl }",
        "<div id=a><section style=\"padding-inline-end: 5px\"></section></div>",
        "section",
    );
    assert_eq!(found.padding.left, 5.0);
}

#[test]
fn declaration_order_is_kept_between_logical_and_physical() {
    let logical_last = section("", "padding-left: 30px; padding-inline-start: 10px");
    assert_eq!(logical_last.padding.left, 10.0);
    let physical_last = section("", "padding-inline-start: 10px; padding-left: 30px");
    assert_eq!(physical_last.padding.left, 30.0);
    let rtl_logical_last = section(
        "direction: rtl",
        "padding-right: 30px; padding-inline-start: 10px",
    );
    assert_eq!(rtl_logical_last.padding.right, 10.0);
}

#[test]
fn important_logical_declarations_win() {
    let found = section(
        "",
        "padding-inline-start: 10px !important; padding-left: 30px",
    );
    assert_eq!(found.padding.left, 10.0);
}

#[test]
fn border_widths_follow_the_direction() {
    let rtl = section(
        "",
        "direction: rtl; border-inline-start: 4px solid red; border-inline-end-width: 2px; border-inline-end-style: solid",
    );
    assert_eq!(rtl.border.right, 4.0);
    assert_eq!(rtl.border.left, 2.0);
}

#[test]
fn inset_inline_start_positions_absolute_boxes() {
    let ltr = section(
        "",
        "position: absolute; width: 50px; inset-inline-start: 20px",
    );
    assert_eq!(ltr.border_box().x, 20.0);
    let rtl = section(
        "direction: rtl",
        "position: absolute; width: 50px; inset-inline-start: 20px",
    );
    assert_eq!(rtl.border_box().x, 230.0);
}

#[test]
fn pairs_split_into_start_and_end() {
    let rtl = section("direction: rtl", "padding-inline: 3px 9px");
    assert_eq!(rtl.padding.right, 3.0);
    assert_eq!(rtl.padding.left, 9.0);
    let ltr = section("", "padding-inline: 3px 9px");
    assert_eq!(ltr.padding.left, 3.0);
    assert_eq!(ltr.padding.right, 9.0);
}

#[test]
fn list_padding_moves_to_the_start_side() {
    let ltr = dimensions("", "<div id=a><ul><li>x</li></ul></div>", "ul");
    assert_eq!(ltr.padding.left, 40.0);
    assert_eq!(ltr.padding.right, 0.0);
    let rtl = dimensions(
        "",
        "<div id=a style=\"direction: rtl\"><ul><li>x</li></ul></div>",
        "ul",
    );
    assert_eq!(rtl.padding.right, 40.0);
    assert_eq!(rtl.padding.left, 0.0);
}

#[test]
fn logical_properties_are_supported() {
    assert!(foundation::supports::evaluate("(margin-inline-start: 4px)"));
    assert!(foundation::supports::evaluate("(inset-inline-end: 4px)"));
    assert!(foundation::supports::evaluate(
        "(border-inline-start: 1px solid red)"
    ));
}
