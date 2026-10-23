// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, LayoutBox};
use foundation::{html, style, text};

const HEBREW: &str = "\u{05E9}\u{05DC}\u{05D5}\u{05DD}";
const HEBREW_VISUAL: &str = "\u{05DD}\u{05D5}\u{05DC}\u{05E9}";
const OTHER: &str = "\u{05D0}\u{05D1}\u{05D2}";
const OTHER_VISUAL: &str = "\u{05D2}\u{05D1}\u{05D0}";

fn lines(style_text: &str, body: &str) -> Vec<Vec<(f32, String)>> {
    fn walk(layout_box: &LayoutBox, out: &mut Vec<(f32, f32, String)>) {
        for fragment in &layout_box.fragments {
            out.push((fragment.baseline, fragment.x, fragment.text.clone()));
        }
        for child in &layout_box.children {
            walk(child, out);
        }
    }
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: 400px; {}\">{}</div></body></html>",
        style_text, body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    walk(&layout_root, &mut found);
    let mut grouped: Vec<(f32, Vec<(f32, String)>)> = Vec::new();
    for (baseline, x, text) in found {
        match grouped
            .iter_mut()
            .find(|(b, _)| (b - baseline).abs() < 0.01)
        {
            Some((_, line)) => line.push((x, text)),
            None => grouped.push((baseline, vec![(x, text)])),
        }
    }
    grouped
        .into_iter()
        .map(|(_, mut line)| {
            line.sort_by(|a, b| a.0.total_cmp(&b.0));
            line
        })
        .collect()
}

fn visual(style_text: &str, body: &str) -> Vec<String> {
    lines(style_text, body)
        .remove(0)
        .into_iter()
        .map(|(_, text)| text)
        .collect()
}

#[test]
fn latin_text_is_untouched() {
    assert_eq!(visual("", "abc def"), vec!["abc", "def"]);
    assert_eq!(visual("direction: rtl", "abc def"), vec!["abc", "def"]);
}

#[test]
fn hebrew_inside_ltr_text_is_reversed_in_place() {
    let body = format!("abc {} def", HEBREW);
    assert_eq!(visual("", &body), vec!["abc", HEBREW_VISUAL, "def"]);
}

#[test]
fn a_right_to_left_paragraph_runs_from_the_right() {
    let body = format!("{} {}", HEBREW, OTHER);
    assert_eq!(
        visual("direction: rtl", &body),
        vec![OTHER_VISUAL, HEBREW_VISUAL]
    );
}

#[test]
fn latin_words_in_a_right_to_left_paragraph_move_to_the_left() {
    let body = format!("{} abc", HEBREW);
    assert_eq!(visual("direction: rtl", &body), vec!["abc", HEBREW_VISUAL]);
}

#[test]
fn numbers_keep_their_digits_in_order() {
    let body = format!("{} 123", HEBREW);
    assert_eq!(visual("direction: rtl", &body), vec!["123", HEBREW_VISUAL]);
}

#[test]
fn trailing_punctuation_follows_the_paragraph_direction() {
    let body = format!("{},", HEBREW);
    assert_eq!(visual("", &body), vec![HEBREW_VISUAL, ","]);
    assert_eq!(
        visual("direction: rtl", &body),
        vec![format!(",{}", HEBREW_VISUAL)]
    );
}

#[test]
fn wrapped_lines_are_reordered_separately() {
    let body = format!("{} {} {} {}", HEBREW, OTHER, HEBREW, OTHER);
    let found = lines("direction: rtl; width: 90px", &body);
    assert_eq!(found.len(), 2, "{:?}", found);
    let texts: Vec<Vec<&str>> = found
        .iter()
        .map(|line| line.iter().map(|(_, t)| t.as_str()).collect())
        .collect();
    assert_eq!(texts[0], vec![OTHER_VISUAL, HEBREW_VISUAL]);
    assert_eq!(texts[1], vec![OTHER_VISUAL, HEBREW_VISUAL]);
}

#[test]
fn right_to_left_text_aligns_to_the_right_edge() {
    let body = format!("{} {}", HEBREW, OTHER);
    let found = lines("direction: rtl", &body);
    let last = found[0].last().unwrap();
    let fonts = text::default_font();
    let width = fonts.width(fonts.select(&[], false, false), &last.1, 16.0);
    assert!((last.0 + width - 400.0).abs() < 0.5, "{} {}", last.0, width);
}

#[test]
fn gaps_follow_the_words_they_separate() {
    let body = format!("{} {}", HEBREW, OTHER);
    let found = lines("direction: rtl", &body);
    let line = &found[0];
    let fonts = text::default_font();
    let face = fonts.select(&[], false, false);
    let first_width = fonts.width(face, &line[0].1, 16.0);
    let gap = line[1].0 - (line[0].0 + first_width);
    let space = fonts.width(face, " ", 16.0);
    assert!((gap - space).abs() < 0.5, "{} {}", gap, space);
}

#[test]
fn decorated_inline_boxes_keep_the_logical_order() {
    let body = format!("{} <span style=\"padding: 0 2px\">{}</span>", HEBREW, OTHER);
    let result = visual("direction: rtl", &body);
    assert_eq!(result.len(), 2);
    assert_eq!(result[0], HEBREW_VISUAL);
}

#[test]
fn plain_spans_do_not_stop_reordering() {
    let body = format!("{} <span>{}</span>", HEBREW, OTHER);
    let result = visual("direction: rtl", &body);
    assert_eq!(result, vec![OTHER_VISUAL, HEBREW_VISUAL]);
}

#[test]
fn marks_stay_attached_after_reversal() {
    let word = "\u{05E9}\u{05B0}\u{05DC}";
    let reversed = "\u{05DC}\u{05E9}\u{05B0}";
    assert_eq!(visual("", word), vec![reversed]);
}

#[test]
fn mixed_words_stay_on_one_line_when_possible() {
    let body = format!("ab{}cd", HEBREW);
    let found = lines("", &body);
    assert_eq!(found.len(), 1);
    let texts: Vec<&str> = found[0].iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(texts, vec!["ab", HEBREW_VISUAL, "cd"]);
}
