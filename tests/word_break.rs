// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::{html, style, text};

fn tree(body: &str, width: f32) -> LayoutBox<'static> {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    );
    let root = Box::leak(Box::new(html::parse(&source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(width, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new())
}

fn fragments(layout_box: &LayoutBox, found: &mut Vec<(String, f32, f32)>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push((fragment.text.clone(), fragment.x, fragment.baseline));
        }
    }
    for child in &layout_box.children {
        fragments(child, found);
    }
}

fn words(body: &str, width: f32) -> Vec<(String, f32, f32)> {
    let mut found = Vec::new();
    fragments(&tree(body, width), &mut found);
    found
}

const LONG: &str = "abcdefghijklmnopqrstuvwxyz";

fn tw(text: &str) -> f32 {
    text::text_width(&text::default_font(), text, 16.0)
}

fn fragments_in(body: &str, width: f32) -> Vec<String> {
    words(body, width).into_iter().map(|w| w.0).collect()
}

#[test]
fn long_word_overflows_by_default() {
    let list = words(&format!("<p>{}</p>", LONG), 60.0);
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].0, LONG);
}

#[test]
fn overflow_wrap_break_word_splits_a_long_word() {
    let list = words(
        &format!("<p style=\"overflow-wrap: break-word\">{}</p>", LONG),
        60.0,
    );
    assert!(list.len() > 2);
    let joined: String = list.iter().map(|w| w.0.as_str()).collect();
    assert_eq!(joined, LONG);
    for word in &list {
        assert!(tw(&word.0) <= 60.0 + 0.001);
    }
}

#[test]
fn overflow_wrap_anywhere_and_word_wrap_alias() {
    for style in [
        "overflow-wrap: anywhere",
        "word-wrap: break-word",
        "word-break: break-word",
    ] {
        let list = words(&format!("<p style=\"{}\">{}</p>", style, LONG), 60.0);
        assert!(list.len() > 2, "{}", style);
    }
}

#[test]
fn break_word_leaves_words_that_fit_alone() {
    let list = fragments_in("<p style=\"overflow-wrap: break-word\">ab cd ef</p>", 400.0);
    assert_eq!(list, ["ab", "cd", "ef"]);
}

#[test]
fn break_word_does_not_fill_the_current_line() {
    let list = words(
        &format!("<p style=\"overflow-wrap: break-word\">ab {}</p>", LONG),
        100.0,
    );
    assert_eq!(list[0].0, "ab");
    assert!(list[1].2 > list[0].2);
    assert_eq!(list[1].1, 0.0);
}

#[test]
fn break_all_fills_the_current_line() {
    let list = words(
        &format!("<p style=\"word-break: break-all\">ab {}</p>", LONG),
        100.0,
    );
    let first_line = list.iter().filter(|w| w.2 == list[0].2).count();
    assert!(first_line > 4);
    assert!(list.iter().any(|w| w.2 > list[0].2));
}

#[test]
fn break_all_keeps_every_line_within_the_width() {
    let list = words(
        &format!(
            "<p style=\"word-break: break-all\">ab {} {}</p>",
            LONG, LONG
        ),
        100.0,
    );
    let mut lines: Vec<f32> = Vec::new();
    for word in &list {
        let end = word.1 + tw(&word.0);
        match lines.iter().position(|y| *y == word.2) {
            Some(index) => lines[index] = lines[index].max(end),
            None => lines.push(end),
        }
    }
    assert!(lines.len() > 4);
    for end in lines {
        assert!(end <= 100.0 + 0.5);
    }
    let joined: String = list.iter().map(|w| w.0.as_str()).collect();
    assert_eq!(joined, format!("ab{}{}", LONG, LONG));
}

#[test]
fn break_all_is_inherited() {
    let list = words(
        &format!("<div style=\"word-break: break-all\"><p>{}</p></div>", LONG),
        60.0,
    );
    assert!(list.len() > 2);
}

#[test]
fn keep_all_and_normal_do_not_break() {
    for style in ["word-break: keep-all", "word-break: normal"] {
        let list = words(&format!("<p style=\"{}\">{}</p>", style, LONG), 60.0);
        assert_eq!(list.len(), 1, "{}", style);
    }
}

#[test]
fn nowrap_prevents_break_all() {
    let list = words(
        &format!(
            "<p style=\"white-space: nowrap; word-break: break-all\">{}</p>",
            LONG
        ),
        60.0,
    );
    assert_eq!(list.len(), 1);
}

#[test]
fn break_all_preserves_underline_continuity() {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><p style=\"word-break: break-all; text-decoration: underline\">{}</p></body></html>",
        LONG
    );
    let root = Box::leak(Box::new(html::parse(&source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    let layout_box = layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new());
    fn rules(layout_box: &LayoutBox, count: &mut usize) {
        *count += layout_box.rules.len();
        for child in &layout_box.children {
            rules(child, count);
        }
    }
    let mut count = 0;
    rules(&layout_box, &mut count);
    assert_eq!(count, 1);
}

const CLIP: &str = "overflow: hidden; text-overflow: ellipsis; white-space: nowrap; width: 100px";

#[test]
fn ellipsis_replaces_the_overflowing_tail() {
    let list = words(
        &format!("<p style=\"{}\">alpha beta gamma delta epsilon</p>", CLIP),
        400.0,
    );
    let last = list.last().unwrap();
    assert!(last.0.ends_with('\u{2026}') || last.0 == "\u{2026}");
    assert!(last.1 + tw(&last.0) <= 100.0 + 0.5);
    assert!(list.len() < 5);
}

#[test]
fn ellipsis_truncates_inside_a_word() {
    let list = words(&format!("<p style=\"{}\">{}</p>", CLIP, LONG), 400.0);
    assert_eq!(list.len(), 2);
    assert_eq!(list[1].0, "\u{2026}");
    assert!(list[0].0.starts_with("abc"));
    assert!(list[0].0.len() < LONG.len());
    assert!(list[1].1 + tw("\u{2026}") <= 100.0 + 0.5);
    assert!((list[1].1 - tw(&list[0].0)).abs() < 0.01);
}

#[test]
fn text_that_fits_gets_no_ellipsis() {
    let list = fragments_in(&format!("<p style=\"{}\">short text</p>", CLIP), 400.0);
    assert_eq!(list, ["short", "text"]);
}

#[test]
fn clip_is_the_default() {
    let list = fragments_in(
        "<p style=\"overflow: hidden; white-space: nowrap; width: 100px\">alpha beta gamma delta epsilon</p>",
        400.0,
    );
    assert_eq!(list.len(), 5);
    assert!(!list.iter().any(|w| w.contains('\u{2026}')));
}

#[test]
fn ellipsis_needs_a_clipping_container() {
    let list = fragments_in(
        "<p style=\"text-overflow: ellipsis; white-space: nowrap; width: 100px\">alpha beta gamma delta epsilon</p>",
        400.0,
    );
    assert_eq!(list.len(), 5);
}

#[test]
fn text_overflow_is_not_inherited() {
    let list = fragments_in(
        "<div style=\"overflow: hidden; text-overflow: ellipsis; width: 100px\"><p style=\"white-space: nowrap\">alpha beta gamma delta epsilon</p></div>",
        400.0,
    );
    assert_eq!(list.len(), 5);
}

#[test]
fn ellipsis_applies_to_each_overflowing_line_of_pre() {
    let list = words(
        "<pre style=\"overflow: hidden; text-overflow: ellipsis; width: 100px\">aaaaaaaaaaaaaaaaaaaaaaaaaa\nbb\ncccccccccccccccccccccccccc</pre>",
        400.0,
    );
    let marks = list.iter().filter(|w| w.0.ends_with('\u{2026}')).count();
    assert_eq!(marks, 2);
    assert!(list.iter().any(|w| w.0 == "bb"));
}

#[test]
fn ellipsis_keeps_the_style_of_the_cut_text() {
    let list = words(
        "<p style=\"overflow: hidden; text-overflow: ellipsis; white-space: nowrap; width: 100px\"><b>alpha beta gamma delta epsilon</b></p>",
        400.0,
    );
    let last = list.last().unwrap();
    assert!(last.0.ends_with('\u{2026}'));
    assert_eq!(last.2, list[0].2);
}

#[test]
fn supports_the_new_properties() {
    for property in [
        "word-break: break-all",
        "overflow-wrap: anywhere",
        "text-overflow: ellipsis",
    ] {
        assert!(foundation::supports::evaluate(&format!("({})", property)));
    }
}
