// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, LayoutBox};
use foundation::{css, html, style, text};

fn fragments(body: &str) -> Vec<(f32, f32)> {
    fn walk(layout_box: &LayoutBox, out: &mut Vec<(f32, f32)>, fonts: &text::Fonts) {
        for fragment in &layout_box.fragments {
            let width = fonts.width(fragment.face, &fragment.text, fragment.font_size);
            out.push((fragment.x, fragment.x + width));
        }
        for child in &layout_box.children {
            walk(child, out, fonts);
        }
    }
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 200px }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 400.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    walk(&layout_root, &mut found, &fonts);
    found
}

fn first(style: &str) -> (f32, f32) {
    fragments(&format!("<div id=a style=\"{}\">word</div>", style))[0]
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn ltr_text_starts_at_the_left() {
    let (left, _) = first("");
    assert!(close(left, 0.0));
    let (left, _) = first("direction: ltr");
    assert!(close(left, 0.0));
}

#[test]
fn rtl_text_starts_at_the_right() {
    let (_, right) = first("direction: rtl");
    assert!(close(right, 200.0));
}

#[test]
fn start_and_end_follow_the_direction() {
    assert!(close(first("text-align: start").0, 0.0));
    assert!(close(first("text-align: end").1, 200.0));
    assert!(close(first("direction: rtl; text-align: start").1, 200.0));
    assert!(close(first("direction: rtl; text-align: end").0, 0.0));
}

#[test]
fn physical_alignment_ignores_the_direction() {
    assert!(close(first("direction: rtl; text-align: left").0, 0.0));
    assert!(close(first("direction: rtl; text-align: right").1, 200.0));
    assert!(close(first("direction: ltr; text-align: right").1, 200.0));
}

#[test]
fn centering_ignores_the_direction() {
    let (left, right) = first("direction: rtl; text-align: center");
    assert!(close(left + (right - left) / 2.0, 100.0));
}

#[test]
fn the_direction_is_inherited() {
    let found =
        fragments("<div id=a style=\"direction: rtl\"><p style=\"margin: 0\">word</p></div>");
    assert!(close(found[0].1, 200.0));
}

#[test]
fn a_child_can_switch_back() {
    let found = fragments(
        "<div id=a style=\"direction: rtl\"><p style=\"margin: 0; direction: ltr\">word</p></div>",
    );
    assert!(close(found[0].0, 0.0));
}

#[test]
fn every_line_is_aligned_to_the_start() {
    let found = fragments("<div id=a style=\"direction: rtl; width: 60px\">aaaa bbbb cccc</div>");
    assert!(found.len() > 1);
    for (_, right) in found {
        assert!(close(right, 60.0), "{}", right);
    }
}

#[test]
fn invalid_directions_are_dropped() {
    let sheet = css::parse("a { direction: sideways } b { direction: rtl } c { direction: ltr }");
    assert!(sheet.rules[0].declarations.is_empty());
    assert_eq!(sheet.rules[1].declarations.len(), 1);
    assert_eq!(sheet.rules[2].declarations.len(), 1);
}

#[test]
fn direction_is_supported() {
    assert!(foundation::supports::evaluate("(direction: rtl)"));
}
