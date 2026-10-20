// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{self, LayoutBox};
use foundation::{html, style, text};

const BLUE: Color = Color { r: 0, g: 0, b: 238 };
const BLACK: Color = Color { r: 0, g: 0, b: 0 };
const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 128, b: 0 };

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

fn collect(layout_box: &LayoutBox, colors: &mut Vec<(String, Color)>, rules: &mut usize) {
    for fragment in &layout_box.fragments {
        colors.push((fragment.text.clone(), fragment.color));
    }
    *rules += layout_box.rules.len();
    for child in &layout_box.children {
        collect(child, colors, rules);
    }
}

fn render(head: &str, body: &str) -> (Vec<(String, Color)>, usize) {
    let mut colors = Vec::new();
    let mut rules = 0;
    collect(&tree(head, body), &mut colors, &mut rules);
    (colors, rules)
}

fn color_of(head: &str, body: &str, word: &str) -> Color {
    render(head, body)
        .0
        .into_iter()
        .find(|(text, _)| text == word)
        .unwrap()
        .1
}

#[test]
fn link_with_href_is_blue_and_underlined() {
    let (colors, rules) = render("", "<a href=\"x\">go</a>");
    assert_eq!(colors[0].1, BLUE);
    assert_eq!(rules, 1);
}

#[test]
fn anchor_without_href_is_plain() {
    let (colors, rules) = render("", "<a name=\"x\">go</a>");
    assert_eq!(colors[0].1, BLACK);
    assert_eq!(rules, 0);
}

#[test]
fn author_color_overrides_the_default() {
    assert_eq!(
        color_of("a { color: red }", "<a href=\"x\">go</a>", "go"),
        RED
    );
}

#[test]
fn link_pseudo_class_matches_only_links() {
    let head = "a:link { color: #008000 }";
    assert_eq!(color_of(head, "<a href=\"x\">go</a>", "go"), GREEN);
    assert_eq!(color_of(head, "<a>go</a>", "go"), BLACK);
}

#[test]
fn any_link_matches_links() {
    assert_eq!(
        color_of(":any-link { color: red }", "<a href=\"x\">go</a>", "go"),
        RED
    );
}

#[test]
fn link_pseudo_class_outranks_a_type_selector() {
    let head = "a { color: red } a:link { color: #008000 }";
    assert_eq!(color_of(head, "<a href=\"x\">go</a>", "go"), GREEN);
}

#[test]
fn dynamic_pseudo_classes_never_match() {
    for pseudo in [
        "hover",
        "active",
        "focus",
        "visited",
        "focus-within",
        "target",
    ] {
        let head = format!("a:{} {{ color: red }}", pseudo);
        assert_eq!(
            color_of(&head, "<a href=\"x\">go</a>", "go"),
            BLUE,
            "{}",
            pseudo
        );
    }
}

#[test]
fn never_matching_selector_keeps_the_rest_of_its_list() {
    let head = "a:hover, p { color: red }";
    assert_eq!(color_of(head, "<p>text</p>", "text"), RED);
}

#[test]
fn unknown_pseudo_class_drops_the_rule() {
    let head = "p::first-line { color: red }";
    assert_eq!(color_of(head, "<p>text</p>", "text"), BLACK);
}

#[test]
fn link_color_is_inherited_by_children() {
    let (colors, rules) = render("", "<a href=\"x\"><b>go</b></a>");
    assert_eq!(colors[0].1, BLUE);
    assert_eq!(rules, 1);
}

#[test]
fn text_decoration_none_removes_the_underline() {
    let (_, rules) = render("a { text-decoration: none }", "<a href=\"x\">go</a>");
    assert_eq!(rules, 0);
}

#[test]
fn link_selector_with_class_combines() {
    let head = "a.nav:link { color: red }";
    assert_eq!(
        color_of(head, "<a class=\"nav\" href=\"x\">go</a>", "go"),
        RED
    );
    assert_eq!(color_of(head, "<a class=\"nav\">go</a>", "go"), BLACK);
}
