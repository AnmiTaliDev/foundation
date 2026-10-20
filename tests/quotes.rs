// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Value};
use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::{html, style, text};

fn tree(head: &str, body: &str) -> LayoutBox<'static> {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }} {}</style></head><body>{}</body></html>",
        head, body
    );
    let root = Box::leak(Box::new(html::parse(&source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new())
}

fn collect(layout_box: &LayoutBox, found: &mut Vec<String>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push(fragment.text.clone());
        }
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

fn words(head: &str, body: &str) -> Vec<String> {
    let mut found = Vec::new();
    collect(&tree(head, body), &mut found);
    found
}

#[test]
fn q_element_gets_default_quotes() {
    assert_eq!(
        words("", "<p><q>hi</q></p>"),
        ["\u{201C}", "hi", "\u{201D}"]
    );
}

#[test]
fn nested_q_uses_the_second_pair() {
    let list = words("", "<p><q>a<q>b</q>c</q></p>");
    assert_eq!(
        list,
        ["\u{201C}", "a", "\u{2018}", "b", "\u{2019}", "c", "\u{201D}"]
    );
}

#[test]
fn depth_beyond_the_list_reuses_the_last_pair() {
    let list = words("", "<p><q><q><q>x</q></q></q></p>");
    assert_eq!(list[1], "\u{2018}");
    assert_eq!(list[2], "\u{2018}");
    assert_eq!(list[3], "x");
    assert_eq!(list[4], "\u{2019}");
}

#[test]
fn quotes_property_sets_the_pairs() {
    let list = words("q { quotes: \"<\" \">\" \"[\" \"]\" }", "<q>a<q>b</q></q>");
    assert_eq!(list, ["<", "a", "[", "b", "]", ">"]);
}

#[test]
fn quotes_is_inherited() {
    let list = words(
        "",
        "<div style=\"quotes: '\\00AB' '\\00BB'\"><p><q>x</q></p></div>",
    );
    assert_eq!(list, ["\u{AB}", "x", "\u{BB}"]);
}

#[test]
fn quotes_none_hides_the_marks() {
    let list = words("q { quotes: none }", "<q>x</q>");
    assert_eq!(list, ["x"]);
}

#[test]
fn quotes_none_still_tracks_depth() {
    let head = "p { quotes: none } div { quotes: \"<\" \">\" \"[\" \"]\" }";
    let list = words(head, "<p><q>a</q></p><div><q>b</q></div>");
    assert_eq!(list, ["a", "<", "b", ">"]);
}

#[test]
fn quotes_auto_restores_the_default() {
    let list = words("q { quotes: \"<\" \">\" } q { quotes: auto }", "<q>x</q>");
    assert_eq!(list, ["\u{201C}", "x", "\u{201D}"]);
}

#[test]
fn open_and_close_quote_in_author_content() {
    let head = "p::before { content: open-quote } p::after { content: close-quote }";
    assert_eq!(words(head, "<p>x</p>"), ["\u{201C}", "x", "\u{201D}"]);
}

#[test]
fn no_open_quote_raises_the_depth_without_text() {
    let head = "p::before { content: no-open-quote } q::before { content: open-quote }";
    let list = words(head, "<p>a</p><q>b</q>");
    assert_eq!(list[1], "\u{2018}");
}

#[test]
fn no_close_quote_lowers_the_depth_without_text() {
    let head = "p::before { content: open-quote } p::after { content: no-close-quote }";
    let list = words(head, "<p>a</p><p>b</p>");
    assert_eq!(list[0], "\u{201C}");
    assert_eq!(list[2], "\u{201C}");
}

#[test]
fn close_quote_at_depth_zero_renders_nothing() {
    let head = "p::before { content: close-quote }";
    assert_eq!(words(head, "<p>x</p>"), ["x"]);
}

#[test]
fn quote_depth_follows_document_order_across_elements() {
    let head = "p::before { content: open-quote }";
    let list = words(head, "<p>a</p><p>b</p>");
    assert_eq!(list, ["\u{201C}", "a", "\u{2018}", "b"]);
}

#[test]
fn quote_keywords_combine_with_strings_and_attr() {
    let head = "a::after { content: open-quote attr(href) close-quote }";
    let list = words(head, "<a href=\"u\">x</a>");
    assert_eq!(list, ["x", "\u{201C}u\u{201D}"]);
}

#[test]
fn quotes_parse() {
    let sheet = css::parse("q { quotes: \"a\" \"b\" 'c' 'd' } p { quotes: none }");
    assert_eq!(
        sheet.rules[0].declarations[0].value,
        Value::Quotes(vec![("a".into(), "b".into()), ("c".into(), "d".into())])
    );
    assert_eq!(
        sheet.rules[1].declarations[0].value,
        Value::Keyword("none".into())
    );
}

#[test]
fn invalid_quotes_are_dropped() {
    for value in ["\"a\"", "\"a\" \"b\" \"c\"", "a b", "\"a\" b", ""] {
        let sheet = css::parse(&format!("q {{ quotes: {} }}", value));
        assert!(sheet.rules[0].declarations.is_empty(), "{}", value);
    }
}

#[test]
fn supports_quotes() {
    assert!(foundation::supports::evaluate("(quotes: none)"));
}
