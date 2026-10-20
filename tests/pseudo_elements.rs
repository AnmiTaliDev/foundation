// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::generated::Part;
use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::{html, style, text};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLACK: Color = Color { r: 0, g: 0, b: 0 };

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

type Found = Vec<(String, Color, f32, f32)>;

fn collect(layout_box: &LayoutBox, found: &mut Found) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push((
                fragment.text.clone(),
                fragment.color,
                fragment.x,
                fragment.baseline,
            ));
        }
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

fn found(head: &str, body: &str) -> Found {
    let mut found = Vec::new();
    collect(&tree(head, body), &mut found);
    found
}

fn words(head: &str, body: &str) -> Vec<String> {
    found(head, body).into_iter().map(|f| f.0).collect()
}

#[test]
fn before_inserts_text_ahead_of_the_content() {
    assert_eq!(
        words("p::before { content: \"A\" }", "<p>b</p>"),
        ["A", "b"]
    );
}

#[test]
fn after_inserts_text_behind_the_content() {
    assert_eq!(words("p::after { content: \"Z\" }", "<p>b</p>"), ["b", "Z"]);
}

#[test]
fn single_colon_form_is_accepted() {
    assert_eq!(words("p:before { content: \"A\" }", "<p>b</p>"), ["A", "b"]);
    assert_eq!(words("p:after { content: \"Z\" }", "<p>b</p>"), ["b", "Z"]);
}

#[test]
fn text_joins_the_line_of_the_content() {
    let list = found("p::before { content: \"A \" }", "<p>b</p>");
    assert_eq!(list[0].3, list[1].3);
    assert!(list[1].2 > list[0].2);
}

#[test]
fn no_content_means_no_box() {
    assert_eq!(words("p::before { color: red }", "<p>b</p>"), ["b"]);
    assert_eq!(words("p::before { content: none }", "<p>b</p>"), ["b"]);
    assert_eq!(words("p::before { content: normal }", "<p>b</p>"), ["b"]);
}

#[test]
fn none_overrides_an_earlier_content() {
    let head = "p::before { content: \"A\" } p::before { content: none }";
    assert_eq!(words(head, "<p>b</p>"), ["b"]);
}

#[test]
fn content_on_a_real_element_is_ignored() {
    assert_eq!(words("p { content: \"A\" }", "<p>b</p>"), ["b"]);
}

#[test]
fn pseudo_element_has_its_own_style() {
    let list = found("p::before { content: \"A\"; color: red }", "<p>b</p>");
    assert_eq!(list[0].1, RED);
    assert_eq!(list[1].1, BLACK);
}

#[test]
fn pseudo_element_inherits_from_its_element() {
    let list = found("p { color: red } p::after { content: \"Z\" }", "<p>b</p>");
    assert_eq!(list[1].1, RED);
}

#[test]
fn attr_reads_the_element_attribute() {
    let head = "a::after { content: \" (\" attr(href) \")\" }";
    let list = words(head, "<a href=\"x.html\">go</a>");
    assert_eq!(list, ["go", "(x.html)"]);
}

#[test]
fn missing_attribute_yields_nothing() {
    assert_eq!(
        words(
            "p::before { content: \"[\" attr(data-x) \"]\" }",
            "<p>b</p>"
        ),
        ["[]", "b"]
    );
}

#[test]
fn string_escapes() {
    let list = words("p::before { content: \"\\41 \\42\\\"\" }", "<p>c</p>");
    assert_eq!(list, ["AB\"", "c"]);
}

#[test]
fn semicolon_and_colon_inside_a_string() {
    let list = words("p::before { content: \"a;b:c\"; color: red }", "<p>d</p>");
    assert_eq!(list[0], "a;b:c");
}

#[test]
fn empty_string_with_block_display_makes_a_box() {
    let plain = tree("", "<p>b</p>");
    let with = tree(
        "p::after { content: \"\"; display: block; height: 20px }",
        "<p>b</p>",
    );
    assert!(with.dimensions.content.height >= plain.dimensions.content.height + 20.0);
}

#[test]
fn block_pseudo_element_starts_a_new_line() {
    let list = found("p::before { content: \"A\"; display: block }", "<p>b</p>");
    assert!(list[1].3 > list[0].3);
}

#[test]
fn display_none_removes_the_pseudo_element() {
    let head = "p::before { content: \"A\"; display: none }";
    assert_eq!(words(head, "<p>b</p>"), ["b"]);
}

#[test]
fn both_pseudo_elements_together() {
    let head = "p::before { content: \"A\" } p::after { content: \"Z\" }";
    assert_eq!(words(head, "<p>b</p>"), ["A", "b", "Z"]);
}

#[test]
fn pseudo_element_applies_to_every_match() {
    let head = "p::before { content: \"-\" }";
    assert_eq!(words(head, "<p>a</p><p>b</p>"), ["-", "a", "-", "b"]);
}

#[test]
fn pseudo_element_composes_with_other_selectors() {
    let head = "li:first-child::before { content: \"*\" }";
    assert_eq!(words(head, "<div><p>a</p><p>b</p></div>"), ["a", "b"]);
    let head = "p:first-child::before { content: \"*\" }";
    assert_eq!(words(head, "<div><p>a</p><p>b</p></div>"), ["*", "a", "b"]);
}

#[test]
fn pseudo_element_must_end_the_selector() {
    assert!(css::parse("p::before b { content: \"x\" }")
        .rules
        .is_empty());
    assert!(css::parse("p::before > b { content: \"x\" }")
        .rules
        .is_empty());
}

#[test]
fn pseudo_element_inside_not_is_invalid() {
    assert!(css::parse("p:not(::before) { color: red }")
        .rules
        .is_empty());
}

#[test]
fn unsupported_pseudo_elements_drop_the_rule() {
    for name in ["::first-line", "::first-letter", "::marker", "::selection"] {
        let rule = format!("p{} {{ content: \"x\" }}", name);
        assert!(css::parse(&rule).rules.is_empty(), "{}", name);
    }
}

#[test]
fn pseudo_element_specificity_counts_as_a_type() {
    let sheet = css::parse("p::before { content: \"x\" } .a::after { content: \"x\" }");
    assert_eq!(sheet.rules[0].selectors[0].specificity(), (0, 0, 2));
    assert_eq!(sheet.rules[1].selectors[0].specificity(), (0, 1, 1));
}

#[test]
fn higher_specificity_wins_for_pseudo_elements() {
    let head = "p::before { content: \"A\" } p.k::before { content: \"B\" }";
    assert_eq!(words(head, "<p class=\"k\">c</p>"), ["B", "c"]);
}

#[test]
fn inline_style_does_not_reach_the_pseudo_element() {
    let head = "p::before { content: \"A\" }";
    let list = found(head, "<p style=\"color: red\">b</p>");
    assert_eq!(list[0].1, RED);
    let head = "p::before { content: \"A\"; color: blue }";
    let list = found(head, "<p style=\"color: red\">b</p>");
    assert_ne!(list[0].1, RED);
}

#[test]
fn content_parses_into_parts() {
    let sheet = css::parse("p::before { content: \"a\" attr(x) }");
    assert_eq!(
        sheet.rules[0].declarations[0].value,
        Value::Content(vec![Part::Literal("a".into()), Part::Attr("x".into())])
    );
}

#[test]
fn unsupported_content_values_are_dropped() {
    for value in ["counter()", "no-quote", "url(x.png)", "\"unterminated"] {
        let sheet = css::parse(&format!("p::before {{ content: {} }}", value));
        assert!(sheet.rules[0].declarations.is_empty(), "{}", value);
    }
}

#[test]
fn clearfix_pattern_keeps_floats_inside() {
    let head = ".c::after { content: \"\"; display: table; clear: both }";
    let body = "<div class=\"c\"><div style=\"float: left; width: 50px; height: 40px\"></div></div><p style=\"margin: 0\">x</p>";
    let baseline = |found: Found| found.into_iter().find(|f| f.0 == "x").unwrap().3;
    assert!(baseline(found(head, body)) >= 40.0);
    assert!(baseline(found("", body)) < 40.0);
}

#[test]
fn text_transform_applies_to_generated_text() {
    let head = "p::before { content: \"ab\"; text-transform: uppercase }";
    assert_eq!(words(head, "<p>c</p>"), ["AB", "c"]);
}

#[test]
fn supports_the_content_property() {
    assert!(foundation::supports::evaluate("(content: \"x\")"));
}
