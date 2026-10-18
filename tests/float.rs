// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::inline::Fragment;
use foundation::layout::{BoxKind, ImageMap, LayoutBox};
use foundation::{html, layout, render, style, text};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn with_layout(source: &str, width: f32, check: impl FnOnce(&LayoutBox)) {
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(width, 600.0)).unwrap();
    let font = text::default_font();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(width, 600.0),
        &font,
        &ImageMap::new(),
    );
    check(&layout_root);
}

fn body<'a, 'b>(root: &'b LayoutBox<'a>) -> &'b LayoutBox<'a> {
    &root.children[0]
}

fn fragments_of<'a, 'b>(layout_box: &'b LayoutBox<'a>, out: &mut Vec<&'b Fragment>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        out.extend(layout_box.fragments.iter());
    }
    for child in &layout_box.children {
        fragments_of(child, out);
    }
}

#[test]
fn left_and_right_floats_sit_at_the_edges() {
    with_layout(
        "<html><style>body { margin: 0 } div { height: 20px; width: 30px } .l { float: left } .r { float: right }</style><body><div class=\"l\"></div><div class=\"r\"></div></body></html>",
        100.0,
        |root| {
            let left = &body(root).children[0].dimensions.content;
            let right = &body(root).children[1].dimensions.content;
            assert_eq!((left.x, left.y), (0.0, 0.0));
            assert_eq!((right.x, right.y), (70.0, 0.0));
        },
    );
}

#[test]
fn floats_do_not_move_in_flow_blocks() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: left; width: 30px; height: 50px } .b { height: 10px }</style><body><div class=\"f\"></div><div class=\"b\"></div></body></html>",
        100.0,
        |root| {
            let block = &body(root).children[1].dimensions.content;
            assert_eq!((block.x, block.y, block.width), (0.0, 0.0, 100.0));
            assert_eq!(body(root).dimensions.content.height, 10.0);
        },
    );
}

#[test]
fn floats_stack_and_wrap_below_when_full() {
    with_layout(
        "<html><style>body { margin: 0 } div { float: left; width: 60px; height: 20px } .m { margin: 5px } </style><body><div></div><div></div><div class=\"m\"></div></body></html>",
        100.0,
        |root| {
            let first = &body(root).children[0].dimensions.content;
            let second = &body(root).children[1].dimensions.content;
            let third = &body(root).children[2].dimensions.content;
            assert_eq!((first.x, first.y), (0.0, 0.0));
            assert_eq!((second.x, second.y), (0.0, 20.0));
            assert_eq!((third.x, third.y), (5.0, 45.0));
        },
    );
}

#[test]
fn side_by_side_floats_use_margin_boxes() {
    with_layout(
        "<html><style>body { margin: 0 } div { float: left; width: 20px; height: 20px; margin: 0 5px }</style><body><div></div><div></div></body></html>",
        100.0,
        |root| {
            assert_eq!(body(root).children[0].dimensions.content.x, 5.0);
            assert_eq!(body(root).children[1].dimensions.content.x, 35.0);
        },
    );
}

#[test]
fn float_width_shrinks_to_fit_content() {
    let font = text::default_font();
    let width = text::text_width(&font, "abc", 16.0);
    with_layout(
        "<html><style>body { margin: 0 } div { float: left; padding: 3px }</style><body><div>abc</div></body></html>",
        200.0,
        |root| {
            let float = &body(root).children[0].dimensions;
            assert!((float.content.width - width).abs() < 0.001);
            assert_eq!(float.margin_box().width, float.content.width + 6.0);
        },
    );
    with_layout(
        "<html><style>body { margin: 0 } div { float: left }</style><body><div>one two three four five six seven</div></body></html>",
        60.0,
        |root| {
            assert_eq!(body(root).children[0].dimensions.content.width, 60.0);
        },
    );
}

#[test]
fn text_flows_beside_a_float_and_returns_below_it() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: left; width: 50px; height: 30px }</style><body><div class=\"f\"></div><p>aa bb cc dd ee ff gg hh ii jj kk ll mm nn oo pp qq rr ss tt</p></body></html>",
        200.0,
        |root| {
            let mut fragments = Vec::new();
            fragments_of(root, &mut fragments);
            assert!(fragments.iter().any(|f| f.baseline < 30.0));
            for fragment in &fragments {
                if fragment.baseline - 16.0 < 30.0 {
                    assert!(fragment.x >= 50.0, "{} {}", fragment.x, fragment.baseline);
                }
            }
            assert!(fragments.iter().any(|f| f.x == 0.0));
        },
    );
}

#[test]
fn text_line_moves_below_a_float_when_the_word_does_not_fit() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: left; width: 90px; height: 40px }</style><body><div class=\"f\"></div><p>wide</p></body></html>",
        100.0,
        |root| {
            let mut fragments = Vec::new();
            fragments_of(root, &mut fragments);
            assert_eq!(fragments.len(), 1);
            assert_eq!(fragments[0].x, 0.0);
            assert!(fragments[0].baseline > 40.0);
        },
    );
}

#[test]
fn right_float_shortens_lines_from_the_right() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: right; width: 150px; height: 100px }</style><body><div class=\"f\"></div><p>aaaa bbbb cccc dddd eeee ffff</p></body></html>",
        200.0,
        |root| {
            let font = text::default_font();
            let mut fragments = Vec::new();
            fragments_of(root, &mut fragments);
            for fragment in &fragments {
                let end = fragment.x + text::text_width(&font, &fragment.text, 16.0);
                assert!(end <= 50.0 + 0.001, "{}", end);
            }
        },
    );
}

#[test]
fn clear_moves_blocks_below_floats() {
    with_layout(
        "<html><style>body { margin: 0 } .l { float: left; width: 10px; height: 30px } .r { float: right; width: 10px; height: 50px } .b { height: 5px }</style><body><div class=\"l\"></div><div class=\"r\"></div><div class=\"b\" style=\"clear: left\"></div><div class=\"b\" style=\"clear: both\"></div></body></html>",
        100.0,
        |root| {
            assert_eq!(body(root).children[2].dimensions.content.y, 30.0);
            assert_eq!(body(root).children[3].dimensions.content.y, 50.0);
            assert_eq!(body(root).dimensions.content.height, 55.0);
        },
    );
}

#[test]
fn clear_applies_to_floats_too() {
    with_layout(
        "<html><style>body { margin: 0 } div { float: left; width: 10px; height: 20px } .c { clear: left }</style><body><div></div><div class=\"c\"></div></body></html>",
        100.0,
        |root| {
            let second = &body(root).children[1].dimensions.content;
            assert_eq!((second.x, second.y), (0.0, 20.0));
        },
    );
}

#[test]
fn overflow_hidden_contains_floats_and_plain_blocks_do_not() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: left; width: 10px; height: 40px } .p { background-color: red }</style><body><div class=\"p\" style=\"overflow: hidden\"><div class=\"f\"></div></div><div class=\"p\"><div class=\"f\"></div></div></body></html>",
        100.0,
        |root| {
            assert_eq!(body(root).children[0].dimensions.content.height, 40.0);
            assert_eq!(body(root).children[1].dimensions.content.height, 0.0);
            assert_eq!(body(root).children[1].dimensions.content.y, 40.0);
        },
    );
}

#[test]
fn root_contains_floats() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: left; width: 10px; height: 70px }</style><body><div class=\"f\"></div></body></html>",
        100.0,
        |root| {
            assert_eq!(root.dimensions.margin_box().height, 70.0);
        },
    );
}

#[test]
fn float_is_positioned_inside_its_containing_block() {
    with_layout(
        "<html><style>body { margin: 0 } .c { margin-left: 10px; padding-top: 4px; width: 80px } .f { float: right; width: 20px; height: 5px }</style><body><div class=\"c\"><div class=\"f\"></div></div></body></html>",
        100.0,
        |root| {
            let float = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((float.x, float.y), (70.0, 4.0));
        },
    );
}

#[test]
fn floated_inline_elements_become_blocks() {
    with_layout(
        "<html><style>body { margin: 0 } span { float: right; width: 20px; height: 10px }</style><body>text <span></span></body></html>",
        100.0,
        |root| {
            let float = &body(root).children[1].dimensions.content;
            assert_eq!(float.x, 80.0);
            assert_eq!(float.width, 20.0);
        },
    );
}

#[test]
fn floated_image_uses_its_attribute_size() {
    with_layout(
        "<html><style>body { margin: 0 }</style><body><img src=\"x.png\" width=\"20\" height=\"10\" style=\"float: right\"></body></html>",
        100.0,
        |root| {
            let image = &body(root).children[0].dimensions.content;
            assert_eq!((image.x, image.width, image.height), (80.0, 20.0, 10.0));
        },
    );
}

#[test]
fn render_draws_floats_on_both_sides() {
    let canvas = render(
        "<html><style>body { margin: 0 } div { float: left; width: 5px; height: 5px; background-color: red } .r { float: right }</style><body><div></div><div class=\"r\"></div></body></html>",
        20,
        10,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(4, 4), RED);
    assert_eq!(canvas.pixel(5, 0), WHITE);
    assert_eq!(canvas.pixel(15, 0), RED);
    assert_eq!(canvas.pixel(19, 4), RED);
    assert_eq!(canvas.pixel(0, 5), WHITE);
}
