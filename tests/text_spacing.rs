// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::paint::Canvas;
use foundation::{html, style, text, Page};

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

fn texts(body: &str) -> Vec<String> {
    words(body, 600.0).into_iter().map(|w| w.0).collect()
}

fn find<'a>(list: &'a [(String, f32, f32)], word: &str) -> &'a (String, f32, f32) {
    list.iter().find(|w| w.0 == word).unwrap()
}

#[test]
fn uppercase_changes_the_text() {
    assert_eq!(
        texts("<p style=\"text-transform: uppercase\">hello World</p>"),
        ["HELLO", "WORLD"]
    );
}

#[test]
fn lowercase_changes_the_text() {
    assert_eq!(
        texts("<p style=\"text-transform: lowercase\">Hello WORLD</p>"),
        ["hello", "world"]
    );
}

#[test]
fn capitalize_raises_the_first_letter_of_each_word() {
    assert_eq!(
        texts("<p style=\"text-transform: capitalize\">hello big wORLD</p>"),
        ["Hello", "Big", "WORLD"]
    );
}

#[test]
fn capitalize_skips_leading_punctuation() {
    assert_eq!(
        texts("<p style=\"text-transform: capitalize\">(hello) \"quoted\"</p>"),
        ["(Hello)", "\"Quoted\""]
    );
}

#[test]
fn none_keeps_the_text() {
    assert_eq!(
        texts("<p style=\"text-transform: none\">Mixed Case</p>"),
        ["Mixed", "Case"]
    );
}

#[test]
fn transform_is_inherited_and_can_be_overridden() {
    let body = "<div style=\"text-transform: uppercase\">a <span style=\"text-transform: none\">b</span> <b>c</b></div>";
    assert_eq!(texts(body), ["A", "b", "C"]);
}

#[test]
fn transform_changes_the_measured_width() {
    let lower = words("<span>mmm</span>", 600.0);
    let upper = words(
        "<span style=\"text-transform: uppercase\">mmm</span><span>x</span>",
        600.0,
    );
    let fonts = text::default_font();
    let wide = fonts.width(0, "MMM", 16.0);
    assert!((upper[1].1 - wide).abs() < 0.5);
    assert_eq!(lower.len(), 1);
}

#[test]
fn transform_works_with_pre_text() {
    let body = "<pre style=\"text-transform: uppercase\">ab\ncd</pre>";
    assert_eq!(texts(body), ["AB", "CD"]);
}

#[test]
fn letter_spacing_widens_the_text() {
    let fonts = text::default_font();
    let plain = words("<span>abc</span><span>x</span>", 600.0);
    let spaced = words(
        "<span style=\"letter-spacing: 3px\">abc</span><span>x</span>",
        600.0,
    );
    let delta = spaced[1].1 - plain[1].1;
    assert!(
        (delta - 9.0).abs() < 0.01,
        "{} {}",
        delta,
        fonts.width(0, "abc", 16.0)
    );
}

#[test]
fn negative_letter_spacing_tightens_the_text() {
    let plain = words("<span>abc</span><span>x</span>", 600.0);
    let tight = words(
        "<span style=\"letter-spacing: -1px\">abc</span><span>x</span>",
        600.0,
    );
    assert!((plain[1].1 - tight[1].1 - 3.0).abs() < 0.01);
}

#[test]
fn letter_spacing_in_em_scales_with_the_font() {
    let plain = words(
        "<span style=\"font-size: 20px\">ab</span><span>x</span>",
        600.0,
    );
    let spaced = words(
        "<span style=\"font-size: 20px; letter-spacing: 0.1em\">ab</span><span>x</span>",
        600.0,
    );
    assert!((spaced[1].1 - plain[1].1 - 4.0).abs() < 0.01);
}

#[test]
fn em_spacing_is_fixed_at_the_declaring_element() {
    let body = "<div style=\"font-size: 10px; letter-spacing: 1em\"><span style=\"font-size: 40px\">a</span><span>x</span></div>";
    let found = words(body, 600.0);
    let fonts = text::default_font();
    let expected = fonts.width(0, "a", 40.0) + 10.0;
    assert!(
        (found[1].1 - expected).abs() < 0.01,
        "{} {}",
        found[1].1,
        expected
    );
}

#[test]
fn letter_spacing_is_inherited() {
    let plain = words("<div><b>abc</b><span>x</span></div>", 600.0);
    let spaced = words(
        "<div style=\"letter-spacing: 2px\"><b>abc</b><span>x</span></div>",
        600.0,
    );
    assert!((spaced[1].1 - plain[1].1 - 6.0).abs() < 0.01);
}

#[test]
fn normal_resets_letter_spacing() {
    let body = "<div style=\"letter-spacing: 5px\"><span style=\"letter-spacing: normal\">abc</span><span>x</span></div>";
    let plain = words("<div><span>abc</span><span>x</span></div>", 600.0);
    let found = words(body, 600.0);
    assert_eq!(found[1].1, plain[1].1);
}

#[test]
fn letter_spacing_changes_wrapping() {
    let body = "<div style=\"width: 100px\">aaaa bbbb</div>";
    let plain = words(body, 600.0);
    assert_eq!(plain[0].2, plain[1].2);
    let spaced = words(
        "<div style=\"width: 100px; letter-spacing: 6px\">aaaa bbbb</div>",
        600.0,
    );
    assert!(spaced[1].2 > spaced[0].2);
}

#[test]
fn word_spacing_widens_the_gaps() {
    let plain = words("<span>a b c</span>", 600.0);
    let spaced = words("<span style=\"word-spacing: 10px\">a b c</span>", 600.0);
    assert!((spaced[1].1 - plain[1].1 - 10.0).abs() < 0.01);
    assert!((spaced[2].1 - plain[2].1 - 20.0).abs() < 0.01);
    assert_eq!(find(&plain, "a").1, find(&spaced, "a").1);
}

#[test]
fn word_spacing_applies_to_preserved_spaces() {
    let plain = words("<pre>a  b</pre>", 600.0);
    let spaced = words("<pre style=\"word-spacing: 5px\">a  b</pre>", 600.0);
    let b_plain = plain.iter().find(|w| w.0 == "b").unwrap().1;
    let b_spaced = spaced.iter().find(|w| w.0 == "b").unwrap().1;
    assert!((b_spaced - b_plain - 10.0).abs() < 0.01);
}

#[test]
fn word_spacing_changes_the_max_content_width() {
    let plain = words("<div style=\"float: left\">aa bb</div>", 600.0);
    let spaced = words(
        "<div style=\"float: left; word-spacing: 20px\">aa bb</div>",
        600.0,
    );
    assert!(spaced[1].1 - spaced[0].1 > plain[1].1 - plain[0].1 + 19.0);
}

#[test]
fn spacing_is_painted() {
    let render = |extra: &str| -> Canvas {
        let html = format!(
            "<html><head><style>body {{ margin: 0 }}</style></head><body><span style=\"{}\">II</span></body></html>",
            extra
        );
        Page::new(&html).render(80, 30, 0.0).canvas
    };
    let ink_columns = |canvas: &Canvas| -> Vec<usize> {
        (0..canvas.width)
            .filter(|x| (0..canvas.height).any(|y| canvas.pixel(*x, y).r < 128))
            .collect()
    };
    let plain = ink_columns(&render(""));
    let spaced = ink_columns(&render("letter-spacing: 10px"));
    let gap_plain = plain.windows(2).map(|w| w[1] - w[0]).max().unwrap();
    let gap_spaced = spaced.windows(2).map(|w| w[1] - w[0]).max().unwrap();
    assert!(gap_spaced >= gap_plain + 9, "{} {}", gap_spaced, gap_plain);
}

#[test]
fn the_underline_covers_the_spacing() {
    let body = "<span style=\"text-decoration: underline; letter-spacing: 4px\">abc</span>";
    let root = tree(body, 600.0);
    let mut width = 0.0;
    fn rules(layout_box: &LayoutBox, width: &mut f32) {
        for rule in &layout_box.rules {
            *width += rule.rect.width;
        }
        for child in &layout_box.children {
            rules(child, width);
        }
    }
    rules(&root, &mut width);
    let fonts = text::default_font();
    let expected = fonts.width(0, "abc", 16.0) + 12.0;
    assert!((width - expected).abs() < 0.5);
}

#[test]
fn invalid_values_are_ignored() {
    assert!(texts("<p style=\"text-transform: shouty\">abc</p>") == ["abc"]);
    let plain = words("<span>abc</span><span>x</span>", 600.0);
    let bad = words(
        "<span style=\"letter-spacing: wide\">abc</span><span>x</span>",
        600.0,
    );
    assert_eq!(plain[1].1, bad[1].1);
}

#[test]
fn supports_recognizes_the_properties() {
    assert!(foundation::supports::evaluate(
        "(text-transform: uppercase)"
    ));
    assert!(foundation::supports::evaluate("(letter-spacing: 1px)"));
    assert!(foundation::supports::evaluate("(word-spacing: 2px)"));
}
