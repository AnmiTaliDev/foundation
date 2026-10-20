// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color};
use foundation::layout::{self, LayoutBox};
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

fn collect(layout_box: &LayoutBox, found: &mut Vec<(String, Color)>) {
    for fragment in &layout_box.fragments {
        found.push((fragment.text.clone(), fragment.color));
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

fn red(head: &str, body: &str) -> Vec<String> {
    let mut found = Vec::new();
    collect(&tree(head, body), &mut found);
    found
        .into_iter()
        .filter(|(_, color)| *color == RED)
        .map(|(text, _)| text)
        .collect()
}

const LIST: &str = "<div><p>a</p><p>b</p><p>c</p><p>d</p><p>e</p></div>";

fn marked(selector: &str) -> Vec<String> {
    red(&format!("{} {{ color: red }}", selector), LIST)
}

#[test]
fn first_and_last_child() {
    assert_eq!(marked("p:first-child"), ["a"]);
    assert_eq!(marked("p:last-child"), ["e"]);
}

#[test]
fn nth_child_forms() {
    assert_eq!(marked("p:nth-child(2)"), ["b"]);
    assert_eq!(marked("p:nth-child(odd)"), ["a", "c", "e"]);
    assert_eq!(marked("p:nth-child(even)"), ["b", "d"]);
    assert_eq!(marked("p:nth-child(2n+1)"), ["a", "c", "e"]);
    assert_eq!(marked("p:nth-child(3n)"), ["c"]);
    assert_eq!(marked("p:nth-child(n+4)"), ["d", "e"]);
    assert_eq!(marked("p:nth-child(-n+2)"), ["a", "b"]);
    assert_eq!(marked("p:nth-child( 2n + 2 )"), ["b", "d"]);
}

#[test]
fn nth_last_child() {
    assert_eq!(marked("p:nth-last-child(2)"), ["d"]);
    assert_eq!(marked("p:nth-last-child(odd)"), ["a", "c", "e"]);
}

#[test]
fn nth_child_counts_elements_only() {
    let body = "<div>text<p>a</p>more<p>b</p><!-- c --><p>c</p></div>";
    assert_eq!(red("p:nth-child(2) { color: red }", body), ["b"]);
}

#[test]
fn type_based_positions() {
    let body = "<div><p>a</p><span>b</span><p>c</p><span>d</span></div>";
    assert_eq!(red("p:first-of-type { color: red }", body), ["a"]);
    assert_eq!(red("span:last-of-type { color: red }", body), ["d"]);
    assert_eq!(red("p:nth-of-type(2) { color: red }", body), ["c"]);
    assert_eq!(red("span:nth-last-of-type(2) { color: red }", body), ["b"]);
}

#[test]
fn only_child_and_only_of_type() {
    let body = "<div><p>a</p></div><div><p>b</p><p>c</p></div><div><i>d</i><p>e</p></div>";
    assert_eq!(red("p:only-child { color: red }", body), ["a"]);
    assert_eq!(red("p:only-of-type { color: red }", body), ["a", "e"]);
}

#[test]
fn root_matches_the_document_element() {
    assert_eq!(red(":root { color: red }", "<p>a</p>"), ["a"]);
    assert!(red("p:root { color: red }", "<p>a</p>").is_empty());
}

#[test]
fn empty_matches_elements_without_children() {
    let body = "<p></p><p>t</p><p></p><p><b></b>u</p>";
    assert_eq!(red("p:empty + p { color: red }", body), ["t", "u"]);
    assert_eq!(red("p:not(:empty) { color: red }", body), ["t", "u"]);
}

#[test]
fn not_excludes_matches() {
    assert_eq!(marked("p:not(:first-child)"), ["b", "c", "d", "e"]);
    assert_eq!(
        marked("p:not(:first-child):not(:last-child)"),
        ["b", "c", "d"]
    );
    let body = "<p class=\"a\">x</p><p>y</p>";
    assert_eq!(red("p:not(.a) { color: red }", body), ["y"]);
}

#[test]
fn not_accepts_a_selector_list() {
    assert_eq!(marked("p:not(:first-child, :last-child)"), ["b", "c", "d"]);
}

#[test]
fn is_and_where_match_any_of_the_list() {
    assert_eq!(marked("p:is(:first-child, :last-child)"), ["a", "e"]);
    assert_eq!(marked("p:where(:nth-child(2), :nth-child(4))"), ["b", "d"]);
}

#[test]
fn where_has_no_specificity() {
    let sheet = css::parse(":where(#a) { color: red } p { color: blue }");
    assert_eq!(sheet.rules[0].selectors[0].specificity(), (0, 0, 0));
    let body = "<p id=\"a\">x</p>";
    assert!(red(":where(#a) { color: red } p { color: blue }", body).is_empty());
}

#[test]
fn is_takes_the_most_specific_argument() {
    let sheet = css::parse(":is(#a, .b, p) { color: red }");
    assert_eq!(sheet.rules[0].selectors[0].specificity(), (1, 0, 0));
}

#[test]
fn not_specificity_follows_its_argument() {
    let sheet = css::parse("p:not(.a) { color: red }");
    assert_eq!(sheet.rules[0].selectors[0].specificity(), (0, 1, 1));
}

#[test]
fn attribute_presence() {
    let body = "<p title=\"t\">a</p><p>b</p>";
    assert_eq!(red("p[title] { color: red }", body), ["a"]);
}

#[test]
fn attribute_operators() {
    let body =
        "<p lang=\"en-US\">a</p><p lang=\"en\">b</p><p lang=\"fr\">c</p><p class=\"x y\">d</p>";
    assert_eq!(red("[lang=en] { color: red }", body), ["b"]);
    assert_eq!(red("[lang|=en] { color: red }", body), ["a", "b"]);
    assert_eq!(red("[lang^=en] { color: red }", body), ["a", "b"]);
    assert_eq!(red("[lang$=US] { color: red }", body), ["a"]);
    assert_eq!(red("[lang*=n-] { color: red }", body), ["a"]);
    assert_eq!(red("[class~=y] { color: red }", body), ["d"]);
    assert!(red("[class~=z] { color: red }", body).is_empty());
}

#[test]
fn attribute_values_can_be_quoted() {
    let body = "<p title=\"a b\">x</p><p title=\"c\">y</p>";
    assert_eq!(red("[title=\"a b\"] { color: red }", body), ["x"]);
    assert_eq!(red("[title='c'] { color: red }", body), ["y"]);
}

#[test]
fn attribute_case_flag() {
    let body = "<p title=\"ABC\">x</p>";
    assert!(red("[title=abc] { color: red }", body).is_empty());
    assert_eq!(red("[title=abc i] { color: red }", body), ["x"]);
}

#[test]
fn attribute_name_is_case_insensitive() {
    let body = "<p TITLE=\"t\">x</p>";
    assert_eq!(red("[title] { color: red }", body), ["x"]);
}

#[test]
fn empty_attribute_values_never_match_substring_operators() {
    let body = "<p title=\"t\">x</p>";
    assert!(red("[title^=\"\"] { color: red }", body).is_empty());
    assert!(red("[title*=\"\"] { color: red }", body).is_empty());
}

#[test]
fn attribute_selector_counts_as_a_class() {
    let sheet = css::parse("p[title] { color: red }");
    assert_eq!(sheet.rules[0].selectors[0].specificity(), (0, 1, 1));
}

#[test]
fn adjacent_sibling_combinator() {
    let body = "<h2>t</h2><p>a</p><p>b</p>";
    assert_eq!(red("h2 + p { color: red }", body), ["a"]);
    assert_eq!(red("h2+p { color: red }", body), ["a"]);
}

#[test]
fn adjacent_sibling_skips_text_nodes() {
    let body = "<h2>t</h2> text <p>a</p>";
    assert_eq!(red("h2 + p { color: red }", body), ["a"]);
}

#[test]
fn general_sibling_combinator() {
    let body = "<h2>t</h2><p>a</p><div>m</div><p>b</p>";
    assert_eq!(red("h2 ~ p { color: red }", body), ["a", "b"]);
    assert_eq!(red("h2~p { color: red }", body), ["a", "b"]);
}

#[test]
fn sibling_combinator_does_not_look_forward() {
    let body = "<p>a</p><h2>t</h2>";
    assert!(red("h2 + p { color: red }", body).is_empty());
    assert!(red("h2 ~ p { color: red }", body).is_empty());
}

#[test]
fn sibling_combinator_chains_with_descendants() {
    let body = "<div><h2>t</h2><p><b>a</b></p></div><p><b>c</b></p>";
    assert_eq!(red("div h2 + p b { color: red }", body), ["a"]);
}

#[test]
fn sibling_combinator_respects_ancestors() {
    let body = "<div class=\"k\"><h2>t</h2><p>a</p></div><div><h2>u</h2><p>b</p></div>";
    assert_eq!(red(".k > h2 + p { color: red }", body), ["a"]);
}

#[test]
fn combined_pseudo_and_attribute_in_one_compound() {
    let body = "<a href=\"x\">a</a><a href=\"y\" class=\"k\">b</a><a class=\"k\">c</a>";
    assert_eq!(red("a.k[href]:last-child { color: red }", body).len(), 0);
    assert_eq!(red("a[href].k:nth-child(2) { color: red }", body), ["b"]);
}

#[test]
fn selector_list_with_functional_pseudo_commas() {
    let body = "<p>a</p><i>b</i><b>c</b>";
    assert_eq!(red("p, b:not(p, i) { color: red }", body), ["a", "c"]);
}

#[test]
fn invalid_selectors_drop_the_rule() {
    let body = "<p>a</p>";
    for selector in [
        "p::first-line",
        "p:nth-child(x)",
        "p:unknown",
        "p:not()",
        "p[",
        "p +",
        "+ p",
        "p > > p",
        "[=x]",
        "p:nth-child(2 of p)",
    ] {
        let head = format!("{} {{ color: red }}", selector);
        assert!(red(&head, body).is_empty(), "{}", selector);
        assert_eq!(css::parse(&head).rules.len(), 0, "{}", selector);
    }
}

#[test]
fn black_remains_the_default() {
    let mut found = Vec::new();
    collect(&tree("", "<p>a</p>"), &mut found);
    assert_eq!(found[0].1, BLACK);
}
