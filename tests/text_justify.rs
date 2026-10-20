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

fn find_all<'a>(list: &'a [(String, f32, f32)], word: &str) -> Vec<&'a (String, f32, f32)> {
    list.iter().filter(|w| w.0 == word).collect()
}

fn ww() -> f32 {
    text::text_width(&text::default_font(), "ww", 16.0)
}

const WORDS: &str = "ww ww ww ww ww ww ww ww ww ww";

fn lines_of(list: &[(String, f32, f32)]) -> Vec<Vec<&(String, f32, f32)>> {
    let mut lines: Vec<Vec<&(String, f32, f32)>> = Vec::new();
    for word in list {
        match lines.last_mut() {
            Some(line) if line[0].2 == word.2 => line.push(word),
            _ => lines.push(vec![word]),
        }
    }
    lines
}

#[test]
fn justified_line_reaches_both_edges() {
    let list = words(
        &format!("<p style=\"text-align: justify\">{}</p>", WORDS),
        200.0,
    );
    let lines = lines_of(&list);
    assert!(lines.len() > 1);
    let first = &lines[0];
    assert_eq!(first[0].1, 0.0);
    let last = first[first.len() - 1];
    assert!((last.1 + ww() - 200.0).abs() < 0.01);
}

#[test]
fn justified_gaps_are_equal() {
    let list = words(
        &format!("<p style=\"text-align: justify\">{}</p>", WORDS),
        200.0,
    );
    let lines = lines_of(&list);
    let first = &lines[0];
    assert!(first.len() > 2);
    let gap = first[1].1 - first[0].1;
    for pair in first.windows(2) {
        assert!((pair[1].1 - pair[0].1 - gap).abs() < 0.01);
    }
}

#[test]
fn last_line_is_not_justified() {
    let justified = words(
        &format!("<p style=\"text-align: justify\">{}</p>", WORDS),
        200.0,
    );
    let plain = words(&format!("<p>{}</p>", WORDS), 200.0);
    let a = lines_of(&justified);
    let b = lines_of(&plain);
    assert_eq!(a.len(), b.len());
    let xs = |line: &Vec<&(String, f32, f32)>| line.iter().map(|w| w.1).collect::<Vec<_>>();
    assert_eq!(xs(a.last().unwrap()), xs(b.last().unwrap()));
}

#[test]
fn forced_break_ends_a_line_unjustified() {
    let list = words(
        "<p style=\"text-align: justify\">ww ww<br>ww ww ww ww ww ww ww ww ww ww</p>",
        200.0,
    );
    let first = find_all(&list, "ww")[0];
    let second = find_all(&list, "ww")[1];
    assert_eq!(first.1, 0.0);
    assert!(
        (second.1 - first.1 - ww() - text::text_width(&text::default_font(), " ", 16.0)).abs()
            < 0.01
    );
}

#[test]
fn pre_is_not_justified() {
    let justified = words(
        "<pre style=\"text-align: justify\">ww ww\nww ww ww</pre>",
        200.0,
    );
    let plain = words("<pre>ww ww\nww ww ww</pre>", 200.0);
    assert_eq!(justified, plain);
}

#[test]
fn single_word_lines_stay_left() {
    let list = words(
        "<p style=\"text-align: justify; width: 40px\">wwwwwwwwwwww wwwwwwwwwwww</p>",
        300.0,
    );
    for word in &list {
        assert_eq!(word.1, 0.0);
    }
}

#[test]
fn justify_is_inherited() {
    let list = words(
        &format!("<div style=\"text-align: justify\"><p>{}</p></div>", WORDS),
        200.0,
    );
    let lines = lines_of(&list);
    let first = &lines[0];
    let last = first[first.len() - 1];
    assert!((last.1 + ww() - 200.0).abs() < 0.01);
}

#[test]
fn end_aligns_like_right() {
    let end = words("<p style=\"text-align: end\">ww</p>", 200.0);
    let right = words("<p style=\"text-align: right\">ww</p>", 200.0);
    assert_eq!(end[0].1, right[0].1);
    assert!(end[0].1 > 100.0);
}

#[test]
fn indent_shifts_the_first_line_only() {
    let list = words(
        &format!("<p style=\"text-indent: 30px\">{}</p>", WORDS),
        200.0,
    );
    let lines = lines_of(&list);
    assert!(lines.len() > 1);
    assert_eq!(lines[0][0].1, 30.0);
    assert_eq!(lines[1][0].1, 0.0);
}

#[test]
fn indent_narrows_the_first_line() {
    let plain = words(&format!("<p>{}</p>", WORDS), 200.0);
    let indented = words(
        &format!("<p style=\"text-indent: 60px\">{}</p>", WORDS),
        200.0,
    );
    assert!(lines_of(&indented)[0].len() < lines_of(&plain)[0].len());
}

#[test]
fn indent_accepts_em_and_percent() {
    let em = words("<p style=\"text-indent: 2em\">ww</p>", 200.0);
    assert_eq!(em[0].1, 32.0);
    let percent = words("<p style=\"text-indent: 10%\">ww</p>", 200.0);
    assert_eq!(percent[0].1, 20.0);
}

#[test]
fn negative_indent_hangs_left() {
    let list = words(
        "<div style=\"padding-left: 40px\"><p style=\"text-indent: -20px\">ww ww ww ww ww ww ww ww ww ww ww ww</p></div>",
        200.0,
    );
    let lines = lines_of(&list);
    assert_eq!(lines[0][0].1, 20.0);
    assert_eq!(lines[1][0].1, 40.0);
}

#[test]
fn indent_is_inherited() {
    let list = words("<div style=\"text-indent: 25px\"><p>ww</p></div>", 200.0);
    assert_eq!(list[0].1, 25.0);
}

#[test]
fn indent_skips_text_after_a_block_child() {
    let list = words("<div style=\"text-indent: 25px\"><p>aa</p>bb</div>", 200.0);
    assert_eq!(find_all(&list, "aa")[0].1, 25.0);
    assert_eq!(find_all(&list, "bb")[0].1, 0.0);
}

#[test]
fn indent_applies_to_centered_text_as_offset() {
    let plain = words("<p style=\"text-align: center\">ww</p>", 200.0);
    let indented = words(
        "<p style=\"text-align: center; text-indent: 40px\">ww</p>",
        200.0,
    );
    assert!((indented[0].1 - plain[0].1 - 20.0).abs() < 0.01);
}

#[test]
fn supports_the_new_property() {
    assert!(foundation::supports::evaluate("(text-indent: 1px)"));
}
