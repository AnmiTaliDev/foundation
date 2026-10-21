// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::style::Content;
use foundation::{html, style, text};

fn find(layout_box: &LayoutBox, id: &str, fonts: &text::Fonts, found: &mut Option<(f32, f32)>) {
    if let BoxKind::Block(node) = layout_box.kind {
        if let Content::Element(element) = node.content {
            if element.attributes.get("id").map(String::as_str) == Some(id) {
                *found = Some(layout_box.scroll_size(fonts));
            }
        }
    }
    for child in &layout_box.children {
        find(child, id, fonts, found);
    }
}

fn scroll_size(css: &str, body: &str) -> (f32, f32) {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 100px; height: 50px; overflow: auto; {} }}</style></head><body>{}</body></html>",
        css, body
    );
    let root = html::parse(&source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let tree = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = None;
    find(&tree, "a", &fonts, &mut found);
    found.unwrap()
}

fn container(inner: &str) -> (f32, f32) {
    scroll_size("", &format!("<div id=a>{}</div>", inner))
}

#[test]
fn empty_container_matches_its_padding_box() {
    assert_eq!(container(""), (100.0, 50.0));
    assert_eq!(
        scroll_size("padding: 5px", "<div id=a></div>"),
        (110.0, 60.0)
    );
}

#[test]
fn tall_content_extends_the_height() {
    assert_eq!(
        container("<div style=\"height: 200px\"></div>"),
        (100.0, 200.0)
    );
}

#[test]
fn wide_content_extends_the_width() {
    assert_eq!(
        container("<div style=\"width: 300px; height: 10px\"></div>"),
        (300.0, 50.0)
    );
}

#[test]
fn content_that_fits_changes_nothing() {
    assert_eq!(
        container("<div style=\"width: 50px; height: 20px\"></div>"),
        (100.0, 50.0)
    );
}

#[test]
fn bottom_padding_follows_overflowing_content() {
    assert_eq!(
        scroll_size(
            "padding-bottom: 10px",
            "<div id=a><div style=\"height: 200px\"></div></div>"
        ),
        (100.0, 210.0)
    );
}

#[test]
fn nested_overflow_counts() {
    assert_eq!(
        container(
            "<div style=\"height: 10px\"><div style=\"width: 250px; height: 120px\"></div></div>"
        ),
        (250.0, 120.0)
    );
}

#[test]
fn clipping_descendants_do_not_leak_their_overflow() {
    assert_eq!(
        container("<div style=\"width: 20px; height: 20px; overflow: hidden\"><div style=\"width: 500px; height: 500px\"></div></div>"),
        (100.0, 50.0)
    );
}

#[test]
fn unbreakable_text_extends_the_width() {
    let (width, _) = container("Supercalifragilisticexpialidocious");
    assert!(width > 100.0, "{}", width);
}

#[test]
fn wrapping_text_extends_the_height() {
    let (width, height) = container(
        "one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen",
    );
    assert_eq!(width, 100.0);
    assert!(height > 50.0, "{}", height);
}

#[test]
fn absolute_and_relative_offsets_count_but_fixed_does_not() {
    assert_eq!(
        scroll_size(
            "position: relative",
            "<div id=a><div style=\"position: absolute; left: 150px; top: 80px; width: 10px; height: 10px\"></div></div>"
        ),
        (160.0, 90.0)
    );
    assert_eq!(
        container("<div style=\"position: fixed; width: 500px; height: 500px\"></div>"),
        (100.0, 50.0)
    );
}

#[test]
fn images_and_floats_count() {
    assert_eq!(
        container("<div style=\"float: left; width: 180px; height: 70px\"></div>"),
        (180.0, 70.0)
    );
}
