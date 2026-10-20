// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::decoration::{Decoration, Line, LineStyle};
use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::paint::Canvas;
use foundation::{html, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLACK: Color = Color { r: 0, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn tree(body: &str, width: usize) -> LayoutBox<'static> {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    );
    let root = Box::leak(Box::new(html::parse(&source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(width as f32, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new())
}

fn rules(layout_box: &LayoutBox, found: &mut Vec<(f32, f32, f32, f32, Color, bool)>) {
    for rule in &layout_box.rules {
        let r = rule.rect;
        found.push((r.x, r.y, r.width, r.height, rule.color, rule.over_text));
    }
    for child in &layout_box.children {
        rules(child, found);
    }
}

fn all_rules(body: &str, width: usize) -> Vec<(f32, f32, f32, f32, Color, bool)> {
    let root = tree(body, width);
    let mut found = Vec::new();
    rules(&root, &mut found);
    found
}

fn baselines(layout_box: &LayoutBox, found: &mut Vec<f32>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push(fragment.baseline);
        }
    }
    for child in &layout_box.children {
        baselines(child, found);
    }
}

fn render(body: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    );
    Page::new(&html).render(200, 60, 0.0).canvas
}

fn declarations(text: &str) -> Vec<(String, Value)> {
    css::parse_declarations(text)
        .into_iter()
        .map(|d| (d.name, d.value))
        .collect()
}

fn find<'a>(list: &'a [(String, Value)], name: &str) -> &'a Value {
    &list.iter().find(|(n, _)| n == name).unwrap().1
}

#[test]
fn no_decoration_means_no_rules() {
    assert!(all_rules("<p>plain text</p>", 400).is_empty());
}

#[test]
fn underline_draws_one_rule_below_the_baseline() {
    let root = tree(
        "<span style=\"text-decoration: underline\">hello</span>",
        400,
    );
    let mut found = Vec::new();
    rules(&root, &mut found);
    let mut base = Vec::new();
    baselines(&root, &mut base);
    assert_eq!(found.len(), 1);
    assert!(found[0].1 > base[0]);
    assert!(found[0].1 < base[0] + 5.0);
    assert_eq!(found[0].3, 1.0);
    assert!(found[0].2 > 20.0);
    assert!(!found[0].5);
}

#[test]
fn line_through_is_over_the_text_and_above_the_baseline() {
    let root = tree("<s>strike</s>", 400);
    let mut found = Vec::new();
    rules(&root, &mut found);
    let mut base = Vec::new();
    baselines(&root, &mut base);
    assert_eq!(found.len(), 1);
    assert!(found[0].1 < base[0]);
    assert!(found[0].5);
}

#[test]
fn overline_is_above_the_text() {
    let root = tree("<span style=\"text-decoration: overline\">top</span>", 400);
    let mut found = Vec::new();
    rules(&root, &mut found);
    let mut base = Vec::new();
    baselines(&root, &mut base);
    assert!(found[0].1 < base[0] - 8.0);
}

#[test]
fn several_lines_make_several_rules() {
    let found = all_rules(
        "<span style=\"text-decoration: underline overline line-through\">x</span>",
        400,
    );
    assert_eq!(found.len(), 3);
}

#[test]
fn the_rule_spans_the_words_and_the_space_between() {
    let fonts = text::default_font();
    let found = all_rules(
        "<span style=\"text-decoration: underline\">ab cd</span>",
        400,
    );
    assert_eq!(found.len(), 1);
    let expected = fonts.width(0, "ab cd", 16.0);
    assert!(
        (found[0].2 - expected).abs() < 0.5,
        "{} {}",
        found[0].2,
        expected
    );
}

#[test]
fn the_rule_does_not_cross_a_wrapped_line() {
    let found = all_rules(
        "<div style=\"width: 50px; text-decoration: underline\">aaa bbb ccc ddd</div>",
        400,
    );
    assert!(found.len() > 1);
    let mut ys: Vec<i32> = found.iter().map(|r| r.1 as i32).collect();
    ys.dedup();
    assert_eq!(ys.len(), found.len());
}

#[test]
fn the_default_color_is_the_text_color() {
    let found = all_rules(
        "<span style=\"color: red; text-decoration: underline\">x</span>",
        400,
    );
    assert_eq!(found[0].4, RED);
}

#[test]
fn an_explicit_color_wins() {
    let found = all_rules(
        "<span style=\"color: red; text-decoration: underline blue\">x</span>",
        400,
    );
    assert_eq!(found[0].4, Color { r: 0, g: 0, b: 255 });
}

#[test]
fn children_inherit_the_decoration_of_the_parent() {
    let found = all_rules(
        "<div style=\"text-decoration: underline\">a <b>bold</b> c</div>",
        400,
    );
    assert!(!found.is_empty());
    let fonts = text::default_font();
    let bold = fonts.select(&["sans-serif".to_string()], true, false);
    let total: f32 = found.iter().map(|r| r.2).sum();
    let minimum = fonts.width(0, "a ", 16.0) + fonts.width(bold, "bold", 16.0);
    assert!(total >= minimum);
}

#[test]
fn a_child_cannot_remove_the_parent_decoration() {
    let with = all_rules(
        "<div style=\"text-decoration: underline\"><span style=\"text-decoration: none\">hi</span></div>",
        400,
    );
    assert_eq!(with.len(), 1);
}

#[test]
fn a_child_adds_its_own_decoration() {
    let found = all_rules(
        "<div style=\"text-decoration: underline\"><span style=\"text-decoration: line-through\">hi</span></div>",
        400,
    );
    assert_eq!(found.len(), 2);
}

#[test]
fn inline_blocks_and_floats_do_not_inherit() {
    let found = all_rules(
        "<div style=\"text-decoration: underline\"><span style=\"display: inline-block\">in</span></div>",
        400,
    );
    assert!(found.is_empty());
    let found = all_rules(
        "<div style=\"text-decoration: underline\"><div style=\"float: left\">fl</div></div>",
        400,
    );
    assert!(found.is_empty());
}

#[test]
fn double_makes_two_rules() {
    let found = all_rules(
        "<span style=\"text-decoration: underline double\">word</span>",
        400,
    );
    assert_eq!(found.len(), 2);
    assert!(found[1].1 > found[0].1);
}

#[test]
fn dotted_and_dashed_make_many_short_rules() {
    let dotted = all_rules(
        "<span style=\"text-decoration: underline dotted\">a long enough word</span>",
        400,
    );
    assert!(dotted.len() > 10);
    assert!(dotted.iter().all(|r| r.2 <= 1.0));
    let dashed = all_rules(
        "<span style=\"text-decoration: underline dashed\">a long enough word</span>",
        400,
    );
    assert!(dashed.len() > 5);
    assert!(dashed.len() < dotted.len());
}

#[test]
fn thickness_is_applied() {
    let found = all_rules(
        "<span style=\"text-decoration: underline 3px\">word</span>",
        400,
    );
    assert_eq!(found[0].3, 3.0);
}

#[test]
fn thickness_scales_with_the_font_size() {
    let found = all_rules(
        "<span style=\"font-size: 48px; text-decoration: underline\">W</span>",
        400,
    );
    assert_eq!(found[0].3, 3.0);
}

#[test]
fn anchors_and_u_are_underlined_by_default() {
    assert_eq!(all_rules("<a href=\"x\">link</a>", 400).len(), 1);
    assert_eq!(all_rules("<u>under</u>", 400).len(), 1);
    assert_eq!(all_rules("<ins>ins</ins>", 400).len(), 1);
    assert_eq!(all_rules("<del>gone</del>", 400).len(), 1);
}

#[test]
fn an_author_can_remove_the_underline_of_a_link() {
    let source = "<html><head><style>a { text-decoration: none }</style></head><body><a href=\"x\">link</a></body></html>";
    let root = Box::leak(Box::new(html::parse(source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    let tree = layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    rules(&tree, &mut found);
    assert!(found.is_empty());
}

#[test]
fn the_underline_is_painted() {
    let with = render("<span style=\"text-decoration: underline; color: red\">MMMM</span>");
    let without = render("<span style=\"color: red\">MMMM</span>");
    let count = |canvas: &Canvas| {
        (0..canvas.height)
            .flat_map(|y| (0..canvas.width).map(move |x| (x, y)))
            .filter(|(x, y)| canvas.pixel(*x, *y) != WHITE)
            .count()
    };
    assert!(count(&with) > count(&without));
    let row = (0..60)
        .find(|y| (0..40).all(|x| with.pixel(x, *y) == RED))
        .expect("a full row of the rule color");
    assert!(row > 10);
}

#[test]
fn the_rule_scrolls_with_the_page() {
    let html = "<html><head><style>body { margin: 0 }</style></head><body><div style=\"height: 60px\"></div><span style=\"text-decoration: underline\">MMMM</span><div style=\"height: 500px\"></div></body></html>";
    let top = Page::new(html).render(200, 120, 0.0).canvas;
    let scrolled = Page::new(html).render(200, 120, 30.0).canvas;
    let rows = |canvas: &Canvas| {
        (0..canvas.height)
            .filter(|y| (0..30).all(|x| canvas.pixel(x, *y) == BLACK))
            .collect::<Vec<_>>()
    };
    let first = rows(&top);
    let second = rows(&scrolled);
    assert_eq!(first.len(), 1);
    assert_eq!(second.len(), 1);
    assert_eq!(first[0], second[0] + 30);
}

#[test]
fn the_shorthand_expands_to_four_longhands() {
    let list = declarations("text-decoration: underline dashed red 2px");
    assert_eq!(list.len(), 4);
    assert_eq!(
        *find(&list, "text-decoration-line"),
        Value::Keyword("underline".into())
    );
    assert_eq!(
        *find(&list, "text-decoration-style"),
        Value::Keyword("dashed".into())
    );
    assert_eq!(*find(&list, "text-decoration-color"), Value::Color(RED));
    assert_eq!(
        *find(&list, "text-decoration-thickness"),
        Value::Length(2.0)
    );
}

#[test]
fn the_shorthand_resets_omitted_parts() {
    let list = declarations("text-decoration: underline");
    assert_eq!(
        *find(&list, "text-decoration-style"),
        Value::Keyword("solid".into())
    );
    assert_eq!(
        *find(&list, "text-decoration-color"),
        Value::Keyword("currentcolor".into())
    );
    let none = declarations("text-decoration: none");
    assert_eq!(
        *find(&none, "text-decoration-line"),
        Value::Keyword("none".into())
    );
}

#[test]
fn longhands_are_validated() {
    assert!(css::parse_declarations("text-decoration-line: sideways").is_empty());
    assert!(css::parse_declarations("text-decoration-style: wobbly").is_empty());
    assert!(css::parse_declarations("text-decoration-color: 12").is_empty());
    assert!(css::parse_declarations("text-decoration: underline bogus").is_empty());
    assert_eq!(
        css::parse_declarations("text-decoration-line: underline line-through").len(),
        1
    );
}

#[test]
fn wavy_is_drawn_as_solid() {
    let found = all_rules(
        "<span style=\"text-decoration: underline wavy\">word</span>",
        400,
    );
    assert_eq!(found.len(), 1);
}

#[test]
fn decoration_values_compare() {
    let a = Decoration {
        line: Line::Underline,
        style: LineStyle::Solid,
        color: foundation::color::Rgba {
            color: BLACK,
            alpha: 255,
        },
        thickness: None,
    };
    assert_eq!(a, a);
}

#[test]
fn supports_recognizes_text_decoration() {
    assert!(foundation::supports::evaluate(
        "(text-decoration: underline)"
    ));
    assert!(foundation::supports::evaluate(
        "(text-decoration-thickness: 2px)"
    ));
}
