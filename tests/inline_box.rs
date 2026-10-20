// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{BoxKind, ImageMap, LayoutBox};
use foundation::{html, layout, render, style, text};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn with_layout(css_text: &str, body_html: &str, width: f32, check: impl FnOnce(&LayoutBox)) {
    let source = format!(
        "<html><style>body {{ margin: 0 }} {}</style><body>{}</body></html>",
        css_text, body_html
    );
    let root = html::parse(&source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width, 400.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let font = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &font, &ImageMap::new());
    check(&layout_root);
}

fn paragraphs<'a, 'b>(root: &'b LayoutBox<'a>) -> Vec<&'b LayoutBox<'a>> {
    let mut found = Vec::new();
    collect(root, &mut found);
    found
}

fn collect<'a, 'b>(layout_box: &'b LayoutBox<'a>, out: &mut Vec<&'b LayoutBox<'a>>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        out.push(layout_box);
    }
    for child in &layout_box.children {
        collect(child, out);
    }
}

fn width_of(text_value: &str) -> f32 {
    text::text_width(&text::default_font(), text_value, 16.0)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn padding_border_and_margin_advance_the_text() {
    with_layout(
        "span { padding: 0 5px; border-width: 2px; margin: 0 3px }",
        "<span>ab</span>cd",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            let fragments = &line.fragments;
            assert!(close(fragments[0].x, 3.0 + 2.0 + 5.0));
            let after = 10.0 + width_of("ab") + 10.0;
            assert!(close(fragments[1].x, after));
        },
    );
}

#[test]
fn box_rect_covers_padding_and_border_but_not_the_margin() {
    with_layout(
        "span { background-color: lime; padding: 4px 6px; border-width: 1px; border-color: red; margin: 0 10px }",
        "<span>ab</span>",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            assert_eq!(line.inline_boxes.len(), 1);
            let inline_box = &line.inline_boxes[0];
            let rect = inline_box.rect;
            assert!(close(rect.x, 10.0));
            assert!(close(rect.width, 1.0 + 6.0 + width_of("ab") + 6.0 + 1.0));
            let font = text::default_font();
            let content = text::ascent(&font, 16.0) + text::descent(&font, 16.0);
            assert!(close(rect.height, content + 2.0 * (4.0 + 1.0)));
            assert!(close(rect.y, line.fragments[0].baseline - text::ascent(&font, 16.0) - 5.0));
            assert_eq!(inline_box.border, [1.0, 1.0, 1.0, 1.0]);
            assert_eq!(inline_box.background, Some(GREEN));
            assert_eq!(inline_box.border_color, Some(RED));
        },
    );
}

#[test]
fn vertical_padding_does_not_change_the_line_height() {
    let font = text::default_font();
    with_layout(
        "span { padding: 20px 0; border-width: 5px }",
        "<span>ab</span>",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            assert!(close(
                line.dimensions.content.height,
                text::line_height(&font, 16.0)
            ));
        },
    );
}

#[test]
fn a_wrapped_box_is_sliced_between_lines() {
    with_layout(
        "span { padding: 0 6px; border-width: 1px; border-color: red; background-color: lime }",
        "<span>aaaa bbbb cccc dddd</span>",
        90.0,
        |root| {
            let line = paragraphs(root)[0];
            let boxes = &line.inline_boxes;
            assert!(boxes.len() >= 2);
            let first = &boxes[0];
            let last = &boxes[boxes.len() - 1];
            assert_eq!(first.border[3], 1.0);
            assert_eq!(first.border[1], 0.0);
            assert_eq!(last.border[3], 0.0);
            assert_eq!(last.border[1], 1.0);
            assert!(close(first.rect.x, 0.0));
            assert!(last.rect.y > first.rect.y);
            for middle in &boxes[1..boxes.len() - 1] {
                assert_eq!(middle.border[1], 0.0);
                assert_eq!(middle.border[3], 0.0);
            }
        },
    );
}

#[test]
fn nested_boxes_are_painted_outer_first_and_stay_inside() {
    with_layout(
        ".o { background-color: red; padding: 0 8px } .i { background-color: lime; padding: 0 3px }",
        "<span class=\"o\">a<span class=\"i\">b</span>c</span>",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            assert_eq!(line.inline_boxes.len(), 2);
            let outer = &line.inline_boxes[0];
            let inner = &line.inline_boxes[1];
            assert_eq!(outer.background, Some(RED));
            assert_eq!(inner.background, Some(GREEN));
            assert!(inner.rect.x >= outer.rect.x);
            assert!(inner.rect.x + inner.rect.width <= outer.rect.x + outer.rect.width);
            assert!(close(outer.rect.width, 8.0 + width_of("a") + 3.0 + width_of("b") + 3.0 + width_of("c") + 8.0));
            assert!(close(inner.rect.x, 8.0 + width_of("a")));
        },
    );
}

#[test]
fn closing_edges_stack_in_the_right_order() {
    with_layout(
        ".o { padding-right: 9px; background-color: red } .i { padding-right: 4px; background-color: lime }",
        "<span class=\"o\"><span class=\"i\">ab</span></span>",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            let outer = &line.inline_boxes[0];
            let inner = &line.inline_boxes[1];
            assert!(close(inner.rect.width, width_of("ab") + 4.0));
            assert!(close(outer.rect.width, width_of("ab") + 4.0 + 9.0));
        },
    );
}

#[test]
fn empty_decorated_boxes_have_no_effect() {
    with_layout(
        "span { padding: 0 20px; background-color: red; margin: 0 5px }",
        "ab<span></span>cd",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            assert!(line.inline_boxes.is_empty());
            assert!(close(line.fragments[0].x, 0.0));
        },
    );
}

#[test]
fn spaces_between_words_inside_a_box_are_covered() {
    with_layout(
        "span { background-color: lime }",
        "x <span>ab cd</span> y",
        400.0,
        |root| {
            let line = paragraphs(root)[0];
            assert_eq!(line.inline_boxes.len(), 1);
            let rect = line.inline_boxes[0].rect;
            let fragments = &line.fragments;
            assert!(close(rect.x, fragments[1].x));
            assert!(close(rect.x + rect.width, fragments[2].x + width_of("cd")));
        },
    );
}

#[test]
fn alignment_uses_the_decorated_width() {
    with_layout(
        "span { padding: 0 10px } body { text-align: center }",
        "<span>ab</span>",
        200.0,
        |root| {
            let line = paragraphs(root)[0];
            let total = 20.0 + width_of("ab");
            assert!(close(line.fragments[0].x, (200.0 - total) / 2.0 + 10.0));
        },
    );
}

#[test]
fn long_words_break_inside_a_box_and_keep_both_edges() {
    with_layout(
        "span { padding: 0 5px; border-width: 1px; border-color: red; background-color: lime; overflow-wrap: break-word }",
        "<span>abcdefghijklmnopqrstuvwxyz</span>",
        70.0,
        |root| {
            let line = paragraphs(root)[0];
            let boxes = &line.inline_boxes;
            assert!(boxes.len() >= 2);
            assert_eq!(boxes[0].border[3], 1.0);
            assert_eq!(boxes[boxes.len() - 1].border[1], 1.0);
        },
    );
}

#[test]
fn render_paints_padding_background_and_border() {
    let canvas = render(
        "<html><style>body { margin: 0 } span { background-color: lime; padding: 0 10px; border-width: 2px; border-color: red }</style><body><span>I</span></body></html>",
        60,
        40,
    );
    let mut found_border = false;
    let mut found_padding = false;
    for y in 0..40 {
        if canvas.pixel(0, y) == RED {
            found_border = true;
        }
        if canvas.pixel(4, y) == GREEN {
            found_padding = true;
        }
    }
    assert!(found_border);
    assert!(found_padding);
    assert_eq!(canvas.pixel(59, 5), WHITE);
}

#[test]
fn inline_box_moves_with_a_floated_container() {
    with_layout(
        ".f { float: right; width: 100px } span { background-color: blue; padding: 0 2px }",
        "<div class=\"f\"><span>ab</span></div>",
        300.0,
        |root| {
            let line = paragraphs(root)[0];
            assert!(close(line.inline_boxes[0].rect.x, 200.0));
            assert_eq!(line.inline_boxes[0].background, Some(BLUE));
        },
    );
}
