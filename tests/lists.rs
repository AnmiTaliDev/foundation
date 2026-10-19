// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, LayoutBox};
use foundation::list::marker_text;
use foundation::{html, style, text};

struct Item {
    text: String,
    x: f32,
    baseline: f32,
}

fn items(body: &str) -> Vec<Item> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    collect(&layout_root, &mut found);
    found
}

fn collect(layout_box: &LayoutBox, found: &mut Vec<Item>) {
    for fragment in &layout_box.fragments {
        found.push(Item {
            text: fragment.text.clone(),
            x: fragment.x,
            baseline: fragment.baseline,
        });
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

fn texts(body: &str) -> Vec<String> {
    items(body).into_iter().map(|item| item.text).collect()
}

fn markers(body: &str) -> Vec<String> {
    texts(body)
        .into_iter()
        .filter(|text| text.ends_with(' '))
        .collect()
}

#[test]
fn marker_formats() {
    assert_eq!(marker_text("disc", 5).as_deref(), Some("\u{2022} "));
    assert_eq!(marker_text("circle", 5).as_deref(), Some("\u{25E6} "));
    assert_eq!(marker_text("square", 5).as_deref(), Some("\u{25AA} "));
    assert_eq!(marker_text("decimal", 12).as_deref(), Some("12. "));
    assert_eq!(marker_text("decimal", -3).as_deref(), Some("-3. "));
    assert_eq!(
        marker_text("decimal-leading-zero", 7).as_deref(),
        Some("07. ")
    );
    assert_eq!(
        marker_text("decimal-leading-zero", 12).as_deref(),
        Some("12. ")
    );
    assert_eq!(marker_text("lower-alpha", 1).as_deref(), Some("a. "));
    assert_eq!(marker_text("lower-latin", 26).as_deref(), Some("z. "));
    assert_eq!(marker_text("upper-alpha", 27).as_deref(), Some("AA. "));
    assert_eq!(marker_text("upper-alpha", 0), None);
    assert_eq!(marker_text("lower-roman", 4).as_deref(), Some("iv. "));
    assert_eq!(
        marker_text("upper-roman", 1994).as_deref(),
        Some("MCMXCIV. ")
    );
    assert_eq!(marker_text("upper-roman", 4000), None);
    assert_eq!(marker_text("none", 1), None);
}

#[test]
fn unordered_items_get_bullets_to_the_left_of_the_text() {
    let found = items("<ul><li>one</li><li>two</li></ul>");
    let texts: Vec<&str> = found.iter().map(|item| item.text.as_str()).collect();
    assert_eq!(texts, ["\u{2022} ", "one", "\u{2022} ", "two"]);
    let fonts = text::default_font();
    let width = text::text_width(&fonts, "\u{2022} ", 16.0);
    assert!((found[0].x + width - found[1].x).abs() < 0.01);
    assert_eq!(found[1].x, 40.0);
    assert!((found[0].baseline - found[1].baseline).abs() < 0.01);
    assert!(found[2].baseline > found[0].baseline);
}

#[test]
fn ordered_items_are_numbered() {
    assert_eq!(
        markers("<ol><li>a</li><li>b</li><li>c</li></ol>"),
        ["1. ", "2. ", "3. "]
    );
    assert_eq!(
        markers("<ol start=\"5\"><li>a</li><li>b</li></ol>"),
        ["5. ", "6. "]
    );
    assert_eq!(
        markers("<ol><li>a</li><li value=\"10\">b</li><li>c</li></ol>"),
        ["1. ", "10. ", "11. "]
    );
}

#[test]
fn non_items_do_not_advance_the_counter() {
    assert_eq!(
        markers("<ol><li>a</li><p>x</p><li>b</li></ol>"),
        ["1. ", "2. "]
    );
}

#[test]
fn nested_lists_have_their_own_counters_and_marker_styles() {
    assert_eq!(
        markers("<ol><li>a<ol><li>x</li><li>y</li></ol></li><li>b</li></ol>"),
        ["1. ", "1. ", "2. ", "2. "]
    );
    assert_eq!(
        markers("<ul><li>a<ul><li>b<ul><li>c</li></ul></li></ul></li></ul>"),
        ["\u{2022} ", "\u{25E6} ", "\u{25AA} "]
    );
    assert_eq!(
        markers("<ol><li>a<ul><li>b</li></ul></li></ol>"),
        ["1. ", "\u{25E6} "]
    );
}

#[test]
fn list_style_type_changes_the_markers() {
    assert_eq!(
        markers("<ol style=\"list-style-type: upper-roman\"><li>a</li><li>b</li></ol>"),
        ["I. ", "II. "]
    );
    assert_eq!(
        markers("<ol style=\"list-style-type: lower-alpha\"><li>a</li><li>b</li></ol>"),
        ["a. ", "b. "]
    );
    assert_eq!(
        markers("<ul style=\"list-style: square inside\"><li>a</li></ul>"),
        ["\u{25AA} "]
    );
    assert_eq!(
        markers("<ul><li style=\"list-style-type: decimal\">a</li><li>b</li></ul>"),
        ["1. ", "\u{2022} "]
    );
}

#[test]
fn none_removes_the_markers() {
    assert!(markers("<ul style=\"list-style: none\"><li>a</li></ul>").is_empty());
    assert!(markers("<ul style=\"list-style-type: none\"><li>a</li></ul>").is_empty());
    assert_eq!(
        texts("<ul style=\"list-style: none\"><li>a</li><li>b</li></ul>"),
        ["a", "b"]
    );
    assert_eq!(
        markers("<ol style=\"list-style-type: none\"><li>a</li><li style=\"list-style-type: decimal\">b</li></ol>"),
        ["2. "]
    );
}

#[test]
fn list_item_display_works_on_any_element() {
    assert_eq!(
        markers("<div style=\"display: list-item\">a</div>"),
        ["\u{2022} "]
    );
    assert_eq!(markers("<li>a</li>"), ["\u{2022} "]);
}

#[test]
fn an_empty_item_still_gets_a_marker() {
    assert_eq!(markers("<ul><li></li></ul>"), ["\u{2022} "]);
}

#[test]
fn item_content_is_indented_by_the_list_padding() {
    let found = items("<ul style=\"padding-left: 20px\"><li>a</li></ul>");
    assert_eq!(found[1].x, 20.0);
    let found = items("<ul style=\"padding-left: 0\"><li>a</li></ul>");
    assert_eq!(found[1].x, 0.0);
    assert!(found[0].x < 0.0);
}

#[test]
fn lists_have_vertical_margins_only_at_the_top_level() {
    let top = items("<ul><li>a</li></ul>");
    assert!(top[1].baseline > 16.0 + 10.0);
    let nested = items("<ul style=\"margin: 0\"><li><ul><li>a</li></ul></li></ul>");
    let flat = items("<ul style=\"margin: 0\"><li>a</li></ul>");
    assert!((nested[2].baseline - flat[1].baseline).abs() < 0.01);
}

#[test]
fn markers_follow_the_item_font() {
    let found = items("<ul style=\"font-size: 32px\"><li>a</li></ul>");
    let fonts = text::default_font();
    let width = text::text_width(&fonts, "\u{2022} ", 32.0);
    assert!((found[0].x + width - found[1].x).abs() < 0.01);
}
