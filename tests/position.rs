// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{ImageMap, LayoutBox};
use foundation::{html, layout, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn with_layout(source: &str, size: (f32, f32), check: impl FnOnce(&LayoutBox)) {
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(size.0, size.1);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let font = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &font, &ImageMap::new());
    check(&layout_root);
}

fn body<'a, 'b>(root: &'b LayoutBox<'a>) -> &'b LayoutBox<'a> {
    &root.children[0]
}

const WINDOW: (f32, f32) = (200.0, 100.0);

#[test]
fn relative_offsets_shift_without_changing_the_flow() {
    with_layout(
        "<html><style>body { margin: 0 } div { height: 10px } .r { position: relative; left: 7px; top: 3px }</style><body><div class=\"r\"></div><div></div></body></html>",
        WINDOW,
        |root| {
            let shifted = &body(root).children[0].dimensions.content;
            let next = &body(root).children[1].dimensions.content;
            assert_eq!((shifted.x, shifted.y), (7.0, 3.0));
            assert_eq!((next.x, next.y), (0.0, 10.0));
            assert_eq!(body(root).dimensions.content.height, 20.0);
        },
    );
}

#[test]
fn relative_right_and_bottom_move_the_opposite_way_and_left_wins() {
    with_layout(
        "<html><style>body { margin: 0 } div { height: 10px; position: relative } .a { right: 5px; bottom: 2px } .b { left: 4px; right: 50px } .c { left: 10% }</style><body><div class=\"a\"></div><div class=\"b\"></div><div class=\"c\"></div></body></html>",
        WINDOW,
        |root| {
            let a = &body(root).children[0].dimensions.content;
            let b = &body(root).children[1].dimensions.content;
            let c = &body(root).children[2].dimensions.content;
            assert_eq!((a.x, a.y), (-5.0, -2.0));
            assert_eq!(b.x, 4.0);
            assert_eq!(c.x, 20.0);
        },
    );
}

#[test]
fn relative_box_moves_its_descendants() {
    with_layout(
        "<html><style>body { margin: 0 } .r { position: relative; left: 10px; top: 10px; height: 20px } .i { height: 5px }</style><body><div class=\"r\"><div class=\"i\"></div></div></body></html>",
        WINDOW,
        |root| {
            let inner = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((inner.x, inner.y), (10.0, 10.0));
        },
    );
}

#[test]
fn absolute_is_placed_inside_the_nearest_positioned_ancestor() {
    with_layout(
        "<html><style>body { margin: 0 } .p { position: relative; margin-left: 10px; margin-top: 20px; padding: 5px; width: 100px; height: 50px } .a { position: absolute; left: 3px; top: 4px; width: 10px; height: 10px }</style><body><div class=\"p\"><div class=\"a\"></div></div></body></html>",
        WINDOW,
        |root| {
            let parent = &body(root).children[0].dimensions;
            let area = parent.padding_box();
            let abs = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((area.x, area.y), (10.0, 20.0));
            assert_eq!((abs.x, abs.y), (area.x + 3.0, area.y + 4.0));
            assert_eq!((abs.width, abs.height), (10.0, 10.0));
        },
    );
}

#[test]
fn absolute_right_and_bottom_anchor_to_the_padding_box() {
    with_layout(
        "<html><style>body { margin: 0 } .p { position: relative; padding: 5px; width: 100px; height: 50px } .a { position: absolute; right: 2px; bottom: 3px; width: 10px; height: 10px; margin: 1px }</style><body><div class=\"p\"><div class=\"a\"></div></div></body></html>",
        WINDOW,
        |root| {
            let abs = &body(root).children[0].children[0].dimensions.content;
            assert_eq!(abs.x, 110.0 - 2.0 - 1.0 - 10.0);
            assert_eq!(abs.y, 60.0 - 3.0 - 1.0 - 10.0);
        },
    );
}

#[test]
fn absolute_without_positioned_ancestor_uses_the_window() {
    with_layout(
        "<html><style>body { margin: 8px } .a { position: absolute; right: 0; bottom: 0; width: 20px; height: 10px } .b { position: absolute; left: 5px; top: 6px; width: 1px; height: 1px }</style><body><div class=\"a\"></div><div class=\"b\"></div></body></html>",
        WINDOW,
        |root| {
            let a = &body(root).children[0].dimensions.content;
            let b = &body(root).children[1].dimensions.content;
            assert_eq!((a.x, a.y), (180.0, 90.0));
            assert_eq!((b.x, b.y), (5.0, 6.0));
        },
    );
}

#[test]
fn absolute_boxes_do_not_take_part_in_the_flow() {
    with_layout(
        "<html><style>body { margin: 0 } .a { position: absolute; left: 0; top: 0; width: 50px; height: 50px } .b { height: 10px }</style><body><div class=\"a\"></div><div class=\"b\"></div></body></html>",
        WINDOW,
        |root| {
            let flow = &body(root).children[1].dimensions.content;
            assert_eq!((flow.x, flow.y), (0.0, 0.0));
            assert_eq!(body(root).dimensions.content.height, 10.0);
        },
    );
}

#[test]
fn absolute_without_offsets_keeps_its_static_position() {
    with_layout(
        "<html><style>body { margin: 0 } .b { height: 20px } .a { position: absolute; width: 5px; height: 5px; margin-left: 3px }</style><body><div class=\"b\"></div><div class=\"a\"></div></body></html>",
        WINDOW,
        |root| {
            let abs = &body(root).children[1].dimensions.content;
            assert_eq!((abs.x, abs.y), (3.0, 20.0));
        },
    );
}

#[test]
fn absolute_width_and_height_follow_the_offsets() {
    with_layout(
        "<html><style>body { margin: 0 } .p { position: relative; width: 100px; height: 50px } .a { position: absolute; left: 10px; right: 20px; top: 5px; bottom: 5px }</style><body><div class=\"p\"><div class=\"a\"></div></div></body></html>",
        WINDOW,
        |root| {
            let abs = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((abs.x, abs.width), (10.0, 70.0));
            assert_eq!((abs.y, abs.height), (5.0, 40.0));
        },
    );
}

#[test]
fn absolute_auto_margins_center_the_box() {
    with_layout(
        "<html><style>body { margin: 0 } .p { position: relative; width: 100px; height: 50px } .a { position: absolute; left: 0; right: 0; top: 0; bottom: 0; width: 40px; height: 10px; margin: auto }</style><body><div class=\"p\"><div class=\"a\"></div></div></body></html>",
        WINDOW,
        |root| {
            let abs = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((abs.x, abs.y), (30.0, 20.0));
        },
    );
}

#[test]
fn absolute_width_shrinks_to_fit() {
    let font = text::default_font();
    let width = text::text_width(&font, "abc def", 16.0);
    with_layout(
        "<html><style>body { margin: 0 } .a { position: absolute; left: 4px; top: 0 }</style><body><div class=\"a\">abc def</div></body></html>",
        WINDOW,
        |root| {
            let abs = &body(root).children[0].dimensions.content;
            assert!((abs.width - width).abs() < 0.001);
            assert_eq!(abs.x, 4.0);
        },
    );
}

#[test]
fn absolute_inside_absolute_uses_the_outer_box() {
    with_layout(
        "<html><style>body { margin: 0 } .o { position: absolute; left: 30px; top: 40px; width: 50px; height: 50px } .i { position: absolute; left: 1px; top: 2px; width: 5px; height: 5px }</style><body><div class=\"o\"><div class=\"i\"></div></div></body></html>",
        WINDOW,
        |root| {
            let inner = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((inner.x, inner.y), (31.0, 42.0));
        },
    );
}

#[test]
fn absolute_follows_a_relative_shift_of_its_container() {
    with_layout(
        "<html><style>body { margin: 0 } .p { position: relative; left: 10px; top: 10px; width: 50px; height: 50px } .a { position: absolute; left: 1px; top: 1px; width: 5px; height: 5px }</style><body><div class=\"p\"><div class=\"a\"></div></div></body></html>",
        WINDOW,
        |root| {
            let abs = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((abs.x, abs.y), (11.0, 11.0));
        },
    );
}

#[test]
fn absolute_inside_a_float_follows_it() {
    with_layout(
        "<html><style>body { margin: 0 } .f { float: right; position: relative; width: 20px; height: 10px; left: -5px } .a { position: absolute; left: 2px; top: 3px; width: 4px; height: 4px }</style><body><div class=\"f\"><div class=\"a\"></div></div></body></html>",
        WINDOW,
        |root| {
            let float = &body(root).children[0].dimensions.content;
            let abs = &body(root).children[0].children[0].dimensions.content;
            assert_eq!(float.x, 175.0);
            assert_eq!((abs.x, abs.y), (177.0, 3.0));
        },
    );
}

#[test]
fn position_overrides_float_and_inline_display() {
    with_layout(
        "<html><style>body { margin: 0 } span { position: absolute; float: left; left: 5px; top: 6px; width: 10px; height: 10px }</style><body>text<span></span></body></html>",
        WINDOW,
        |root| {
            let span = &body(root).children[1].dimensions.content;
            assert_eq!((span.x, span.y, span.width), (5.0, 6.0, 10.0));
        },
    );
}

#[test]
fn fixed_uses_the_window_even_inside_a_positioned_box() {
    with_layout(
        "<html><style>body { margin: 0 } .p { position: relative; left: 30px; top: 30px; height: 10px } .f { position: fixed; right: 0; top: 0; width: 20px; height: 5px }</style><body><div class=\"p\"><div class=\"f\"></div></div></body></html>",
        WINDOW,
        |root| {
            let fixed = &body(root).children[0].children[0].dimensions.content;
            assert_eq!((fixed.x, fixed.y), (180.0, 0.0));
        },
    );
}

#[test]
fn fixed_boxes_stay_put_while_the_page_scrolls() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .tall { height: 200px; background-color: red } .f { position: fixed; left: 0; top: 0; width: 10px; height: 10px; background-color: lime }</style><body><div class=\"tall\"></div><div class=\"f\"></div></body></html>",
    );
    for scroll in [0.0, 100.0] {
        let canvas = page.render(30, 20, scroll).canvas;
        assert_eq!(canvas.pixel(0, 0), GREEN);
        assert_eq!(canvas.pixel(9, 9), GREEN);
        assert_eq!(canvas.pixel(15, 0), RED);
        assert_eq!(canvas.pixel(0, 15), RED);
    }
}

#[test]
fn positive_z_index_paints_above_later_boxes() {
    let page = Page::new(
        "<html><style>body { margin: 0 } div { position: absolute; left: 0; top: 0; width: 10px; height: 10px } .a { z-index: 2; background-color: red } .b { z-index: 1; background-color: lime }</style><body><div class=\"a\"></div><div class=\"b\"></div></body></html>",
    );
    assert_eq!(page.render(20, 20, 0.0).canvas.pixel(5, 5), RED);
}

#[test]
fn equal_z_index_follows_tree_order_and_beats_normal_flow() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .p { position: absolute; left: 0; top: 0; width: 10px; height: 10px } .r { background-color: red } .g { background-color: lime } .n { height: 10px; background-color: blue }</style><body><div class=\"p r\"></div><div class=\"p g\"></div><div class=\"n\"></div></body></html>",
    );
    assert_eq!(page.render(20, 20, 0.0).canvas.pixel(5, 5), GREEN);
}

#[test]
fn negative_z_index_paints_below_normal_content() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .back { position: absolute; left: 0; top: 0; width: 10px; height: 10px; z-index: -1; background-color: lime } .n { height: 10px; background-color: blue }</style><body><div class=\"back\"></div><div class=\"n\"></div></body></html>",
    );
    assert_eq!(page.render(20, 20, 0.0).canvas.pixel(5, 5), BLUE);
}

#[test]
fn relative_boxes_paint_above_normal_siblings() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .r { position: relative; top: 5px; height: 10px; background-color: red } .n { height: 10px; background-color: blue }</style><body><div class=\"r\"></div><div class=\"n\"></div></body></html>",
    );
    let canvas = page.render(20, 30, 0.0).canvas;
    assert_eq!(canvas.pixel(5, 12), RED);
    assert_eq!(canvas.pixel(5, 17), BLUE);
    assert_eq!(canvas.pixel(5, 2), WHITE);
}

#[test]
fn sticky_is_treated_as_static() {
    with_layout(
        "<html><style>body { margin: 0 } div { height: 10px; position: sticky; top: 5px }</style><body><div></div></body></html>",
        WINDOW,
        |root| {
            assert_eq!(body(root).children[0].dimensions.content.y, 0.0);
        },
    );
}

#[test]
fn negative_z_index_inside_a_non_context_positioned_box_goes_below_the_box() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .p { position: relative; height: 10px; background-color: blue } .back { position: absolute; left: 0; top: 0; width: 10px; height: 10px; z-index: -1; background-color: lime }</style><body><div class=\"p\"><div class=\"back\"></div></div></body></html>",
    );
    assert_eq!(page.render(20, 20, 0.0).canvas.pixel(5, 5), BLUE);
}

#[test]
fn z_index_creates_a_stacking_context_for_descendants() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .p { position: relative; z-index: 0; height: 10px; background-color: blue } .back { position: absolute; left: 0; top: 0; width: 10px; height: 10px; z-index: -1; background-color: lime }</style><body><div class=\"p\"><div class=\"back\"></div></div></body></html>",
    );
    assert_eq!(page.render(20, 20, 0.0).canvas.pixel(5, 5), GREEN);
}
