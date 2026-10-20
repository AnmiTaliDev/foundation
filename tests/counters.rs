// SPDX-License-Identifier: GPL-3.0-only

use foundation::counters::Counters;
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
fn increment_numbers_siblings() {
    let head = "body { counter-reset: n } p { counter-increment: n } p::before { content: counter(n) \".\" }";
    assert_eq!(
        words(head, "<p>a</p><p>b</p><p>c</p>"),
        ["1.", "a", "2.", "b", "3.", "c"]
    );
}

#[test]
fn counter_defaults_to_zero() {
    let head = "p::before { content: counter(n) }";
    assert_eq!(words(head, "<p>a</p>"), ["0", "a"]);
}

#[test]
fn increment_without_reset_creates_the_counter() {
    let head = "p { counter-increment: n } p::before { content: counter(n) }";
    assert_eq!(words(head, "<p>a</p><p>b</p>"), ["1", "a", "2", "b"]);
}

#[test]
fn reset_value_and_increment_step() {
    let head = "body { counter-reset: n 10 } p { counter-increment: n 5 } p::before { content: counter(n) }";
    assert_eq!(words(head, "<p>a</p><p>b</p>"), ["15", "a", "20", "b"]);
}

#[test]
fn negative_values() {
    let head = "body { counter-reset: n -3 } p { counter-increment: n -2 } p::before { content: counter(n) }";
    assert_eq!(words(head, "<p>a</p><p>b</p>"), ["-5", "a", "-7", "b"]);
}

#[test]
fn set_overrides_the_value() {
    let head =
        "p { counter-increment: n } .s { counter-set: n 40 } p::before { content: counter(n) }";
    assert_eq!(
        words(head, "<p>a</p><p class=\"s\">b</p><p>c</p>"),
        ["1", "a", "40", "b", "41", "c"]
    );
}

#[test]
fn operations_run_in_reset_increment_set_order() {
    let head = "p { counter-reset: n 5; counter-increment: n; counter-set: n 20 } p::before { content: counter(n) }";
    assert_eq!(words(head, "<p>a</p>"), ["20", "a"]);
}

#[test]
fn several_counters_in_one_declaration() {
    let head = "body { counter-reset: a 1 b 2 } p { counter-increment: a b 3 } p::before { content: counter(a) \"-\" counter(b) }";
    assert_eq!(words(head, "<p>x</p>"), ["2-5", "x"]);
}

#[test]
fn none_clears_the_operation() {
    let head = "p { counter-increment: n } .k { counter-increment: none } p::before { content: counter(n) }";
    assert_eq!(
        words(head, "<p>a</p><p class=\"k\">b</p><p>c</p>"),
        ["1", "a", "1", "b", "2", "c"]
    );
}

#[test]
fn reset_in_a_child_scopes_to_its_siblings_and_descendants() {
    let head =
        "div { counter-reset: n } p { counter-increment: n } p::before { content: counter(n) }";
    let body = "<div><p>a</p><p>b</p></div><div><p>c</p></div>";
    assert_eq!(words(head, body), ["1", "a", "2", "b", "1", "c"]);
}

#[test]
fn counter_is_not_visible_outside_the_scope() {
    let head = "div { counter-reset: n } div p { counter-increment: n } span::before { content: counter(n) }";
    let body = "<section><div><p>a</p></div></section><span>s</span>";
    assert_eq!(words(head, body), ["a", "0", "s"]);
}

#[test]
fn nested_counters_use_the_innermost_instance() {
    let head =
        "ol { counter-reset: n } li { counter-increment: n } li::before { content: counter(n) }";
    let body = "<ol><li>a<ol><li>b</li><li>c</li></ol></li><li>d</li></ol>";
    assert_eq!(words(head, body), ["1", "a", "1", "b", "2", "c", "2", "d"]);
}

#[test]
fn counters_joins_nested_instances() {
    let head = "ol { counter-reset: n } li { counter-increment: n } li::before { content: counters(n, \".\") \" \" }";
    let body = "<ol><li>a<ol><li>b</li><li>c</li></ol></li><li>d</li></ol>";
    assert_eq!(
        words(head, body),
        ["1", "a", "1.1", "b", "1.2", "c", "2", "d"]
    );
}

#[test]
fn counters_with_a_single_level() {
    let head = "body { counter-reset: n 4 } p::before { content: counters(n, \"-\") }";
    assert_eq!(words(head, "<p>a</p>"), ["4", "a"]);
}

#[test]
fn counter_styles() {
    let head = "body { counter-reset: n 3 } \
        .a::before { content: counter(n, lower-roman) } \
        .b::before { content: counter(n, upper-alpha) } \
        .c::before { content: counter(n, decimal-leading-zero) } \
        .d::before { content: counter(n, upper-roman) } \
        .e::before { content: counter(n, lower-alpha) } \
        .f::before { content: counter(n, none) \"x\" } \
        .g::before { content: counter(n, disc) }";
    let list = words(
        head,
        "<p class=\"a\">1</p><p class=\"b\">2</p><p class=\"c\">3</p><p class=\"d\">4</p><p class=\"e\">5</p><p class=\"f\">6</p><p class=\"g\">7</p>",
    );
    assert_eq!(
        list,
        ["iii", "1", "C", "2", "03", "3", "III", "4", "c", "5", "x", "6", "\u{2022}", "7"]
    );
}

#[test]
fn style_falls_back_to_decimal_when_not_representable() {
    let head = "body { counter-reset: n 0 } p::before { content: counter(n, lower-alpha) }";
    assert_eq!(words(head, "<p>a</p>"), ["0", "a"]);
}

#[test]
fn display_none_has_no_counter_effect() {
    let head = "p { counter-increment: n } .h { display: none } p::before { content: counter(n) }";
    let body = "<p>a</p><p class=\"h\">b</p><p>c</p>";
    assert_eq!(words(head, body), ["1", "a", "2", "c"]);
}

#[test]
fn pseudo_element_can_increment_a_counter() {
    let head =
        "body { counter-reset: n } p::before { counter-increment: n; content: counter(n) \" \" }";
    assert_eq!(words(head, "<p>a</p><p>b</p>"), ["1", "a", "2", "b"]);
}

#[test]
fn after_sees_changes_made_by_children() {
    let head =
        "div { counter-reset: n } span { counter-increment: n } div::after { content: counter(n) }";
    assert_eq!(
        words(head, "<div><span>a</span><span>b</span></div>"),
        ["a", "b", "2"]
    );
}

#[test]
fn heading_numbering_pattern() {
    let head = "body { counter-reset: h } h2 { counter-reset: s; counter-increment: h } \
        h3 { counter-increment: s } \
        h2::before { content: counter(h) \". \" } \
        h3::before { content: counter(h) \".\" counter(s) \" \" }";
    let body = "<h2>A</h2><h3>a</h3><h3>b</h3><h2>B</h2><h3>c</h3>";
    assert_eq!(
        words(head, body),
        ["1.", "A", "1.1", "a", "1.2", "b", "2.", "B", "2.1", "c"]
    );
}

#[test]
fn counter_ops_parse() {
    let sheet = css::parse("p { counter-reset: a 2 b; counter-increment: c; counter-set: d -1 }");
    let declarations = &sheet.rules[0].declarations;
    assert_eq!(
        declarations[0].value,
        Value::Counters(vec![("a".into(), 2), ("b".into(), 0)])
    );
    assert_eq!(
        declarations[1].value,
        Value::Counters(vec![("c".into(), 1)])
    );
    assert_eq!(
        declarations[2].value,
        Value::Counters(vec![("d".into(), -1)])
    );
}

#[test]
fn invalid_counter_ops_are_dropped() {
    for value in ["", "1", "none a", "a 1.5"] {
        let sheet = css::parse(&format!("p {{ counter-reset: {} }}", value));
        assert!(sheet.rules[0].declarations.is_empty(), "{}", value);
    }
}

#[test]
fn invalid_counter_functions_are_dropped() {
    for value in [
        "counter()",
        "counter(n, bogus)",
        "counters(n)",
        "counters(n, x)",
        "counter(n, decimal, x)",
        "counters(n, \".\", decimal, x)",
    ] {
        let sheet = css::parse(&format!("p::before {{ content: {} }}", value));
        assert!(sheet.rules[0].declarations.is_empty(), "{}", value);
    }
}

#[test]
fn separator_may_contain_parentheses_and_commas() {
    let head = "ol { counter-reset: n } li { counter-increment: n } li::before { content: counters(n, \"),\") }";
    let body = "<ol><li>a<ol><li>b</li></ol></li></ol>";
    assert_eq!(words(head, body), ["1", "a", "1),1", "b"]);
}

#[test]
fn counters_struct_scopes_by_frame() {
    let mut counters = Counters::new();
    counters.reset("n", 1);
    counters.enter();
    counters.reset("n", 5);
    assert_eq!(counters.values("n"), [1, 5]);
    counters.increment("n", 2);
    assert_eq!(counters.value("n"), 7);
    counters.leave();
    assert_eq!(counters.values("n"), [1]);
    counters.set("m", 9);
    assert_eq!(counters.value("m"), 9);
}

#[test]
fn supports_the_counter_properties() {
    assert!(foundation::supports::evaluate("(counter-reset: a)"));
    assert!(foundation::supports::evaluate("(counter-increment: a)"));
    assert!(foundation::supports::evaluate("(counter-set: a 1)"));
}

#[test]
fn counter_reset_scope_reaches_following_siblings() {
    let head =
        "div { counter-reset: n } p { counter-increment: n } span::before { content: counter(n) }";
    let body = "<div><p>a</p></div><span>s</span>";
    assert_eq!(words(head, body), ["a", "1", "s"]);
}
