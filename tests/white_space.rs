// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::{html, style, text};

fn fragments(body: &str, width: usize) -> Vec<(String, f32, f32)> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} pre {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width as f32, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    collect(&layout_root, &mut found);
    found
}

fn collect(layout_box: &LayoutBox, found: &mut Vec<(String, f32, f32)>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push((fragment.text.clone(), fragment.x, fragment.baseline));
        }
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

fn texts(found: &[(String, f32, f32)]) -> Vec<&str> {
    found.iter().map(|(text, _, _)| text.as_str()).collect()
}

fn lines(found: &[(String, f32, f32)]) -> usize {
    let mut baselines: Vec<i32> = found.iter().map(|(_, _, y)| y.round() as i32).collect();
    baselines.dedup();
    baselines.len()
}

#[test]
fn pre_keeps_line_breaks() {
    let found = fragments("<pre>one\ntwo\nthree</pre>", 400);
    assert_eq!(texts(&found), ["one", "two", "three"]);
    assert_eq!(lines(&found), 3);
}

#[test]
fn pre_drops_a_leading_newline() {
    let found = fragments("<pre>\none\ntwo</pre>", 400);
    assert_eq!(texts(&found), ["one", "two"]);
    assert_eq!(lines(&found), 2);
}

#[test]
fn pre_keeps_a_second_leading_newline_as_a_blank_line() {
    let found = fragments("<pre>\n\none</pre>", 400);
    assert_eq!(lines(&found), 2);
    let blank_height = found[1].2 - found[0].2;
    assert!(blank_height > 10.0);
}

#[test]
fn pre_keeps_blank_lines() {
    let found = fragments("<pre>a\n\nb</pre>", 400);
    let a = found.iter().find(|f| f.0 == "a").unwrap();
    let b = found.iter().find(|f| f.0 == "b").unwrap();
    let step = text::line_height(&text::default_font(), 16.0);
    assert!(b.2 - a.2 > step * 1.5);
}

#[test]
fn pre_ignores_a_trailing_newline() {
    let with = fragments("<pre>a\nb\n</pre>", 400);
    let without = fragments("<pre>a\nb</pre>", 400);
    assert_eq!(lines(&with), lines(&without));
}

#[test]
fn pre_keeps_runs_of_spaces() {
    let found = fragments("<pre>a    b</pre>", 400);
    let fonts = text::default_font();
    let face = fonts.select(&["monospace".to_string()], false, false);
    let first = found.iter().find(|f| f.0.starts_with('a')).unwrap();
    let second = found.iter().find(|f| f.0 == "b").unwrap();
    let gap = second.1 - first.1;
    assert!((gap - fonts.width(face, "a    ", 16.0)).abs() < 0.5);
}

#[test]
fn pre_keeps_leading_indentation() {
    let found = fragments("<pre>a\n    b</pre>", 400);
    let fonts = text::default_font();
    let face = fonts.select(&["monospace".to_string()], false, false);
    let a = found.iter().find(|f| f.0 == "a").unwrap();
    let b = found.iter().find(|f| f.0 == "b").unwrap();
    assert!((b.1 - a.1 - fonts.width(face, "    ", 16.0)).abs() < 0.5);
}

#[test]
fn pre_does_not_wrap() {
    let found = fragments(
        "<pre>one two three four five six seven eight nine ten</pre>",
        60,
    );
    assert_eq!(lines(&found), 1);
}

#[test]
fn tab_advances_eight_spaces() {
    let fonts = text::default_font();
    let face = fonts.select(&["monospace".to_string()], false, false);
    let space = fonts.width(face, " ", 16.0);
    assert!((fonts.width(face, "\t", 16.0) - space * 8.0).abs() < 0.01);
}

#[test]
fn pre_wrap_wraps_and_keeps_breaks() {
    let body = "<div style=\"white-space: pre-wrap\">aaa bbb ccc ddd\neee</div>";
    let wide = fragments(body, 600);
    assert_eq!(lines(&wide), 2);
    let narrow = fragments(body, 60);
    assert!(lines(&narrow) > 3);
}

#[test]
fn pre_wrap_keeps_spaces() {
    let found = fragments("<div style=\"white-space: pre-wrap\">a   b</div>", 600);
    let joined: String = found.iter().map(|f| f.0.as_str()).collect();
    assert_eq!(joined, "a   b");
}

#[test]
fn pre_line_collapses_spaces_and_keeps_breaks() {
    let found = fragments(
        "<div style=\"white-space: pre-line\">a    b\n   c</div>",
        600,
    );
    assert_eq!(texts(&found), ["a", "b", "c"]);
    assert_eq!(lines(&found), 2);
}

#[test]
fn nowrap_collapses_but_does_not_wrap() {
    let body = "<div style=\"white-space: nowrap\">one   two three four five six seven eight</div>";
    let found = fragments(body, 60);
    assert_eq!(lines(&found), 1);
    assert_eq!(found.len(), 8);
}

#[test]
fn nowrap_ignores_newlines() {
    let found = fragments("<div style=\"white-space: nowrap\">a\nb</div>", 600);
    assert_eq!(lines(&found), 1);
}

#[test]
fn normal_still_collapses_newlines() {
    let found = fragments("<div>a\n\n   b</div>", 600);
    assert_eq!(lines(&found), 1);
}

#[test]
fn white_space_is_inherited() {
    let found = fragments(
        "<div style=\"white-space: pre\"><span>a\nb</span></div>",
        600,
    );
    assert_eq!(lines(&found), 2);
}

#[test]
fn white_space_can_be_overridden_in_children() {
    let found = fragments(
        "<pre>a\n<span style=\"white-space: normal\">b\nc</span></pre>",
        600,
    );
    let b = found.iter().find(|f| f.0 == "b").unwrap();
    let c = found.iter().find(|f| f.0 == "c").unwrap();
    assert!((b.2 - c.2).abs() < 0.5);
}

#[test]
fn inline_content_after_a_preformatted_newline_starts_a_line() {
    let found = fragments("<pre>a\n<b>b</b></pre>", 600);
    let a = found.iter().find(|f| f.0 == "a").unwrap();
    let b = found.iter().find(|f| f.0 == "b").unwrap();
    assert!(b.2 > a.2);
}

#[test]
fn nowrap_widens_the_min_content_of_a_float() {
    let found = fragments(
        "<div style=\"float:left; white-space: nowrap\">aaa bbb ccc</div>",
        20,
    );
    assert_eq!(lines(&found), 1);
}

#[test]
fn supports_recognizes_white_space() {
    assert!(foundation::supports::evaluate("(white-space: pre)"));
}
