// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::paint::Canvas;
use foundation::{html, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const IB: &str = ".ib { display: inline-block }";

fn render(body: &str, width: usize) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }} {}</style></head><body>{}</body></html>",
        IB, body
    );
    Page::new(&html).render(width, 120, 0.0).canvas
}

fn fragments(body: &str, width: usize) -> Vec<(String, f32, f32)> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} {}</style></head><body>{}</body></html>",
        IB, body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width as f32, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    collect(&layout_root, &mut found);
    found
}

fn collect(layout_box: &LayoutBox, found: &mut Vec<(String, f32, f32)>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push((fragment.text.clone(), fragment.x, fragment.baseline));
        }
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

#[test]
fn inline_blocks_sit_side_by_side() {
    let canvas = render(
        "<span class=\"ib\" style=\"width:20px;height:10px;background-color:red\"></span><span class=\"ib\" style=\"width:20px;height:10px;background-color:blue\"></span>",
        100,
    );
    assert_eq!(canvas.pixel(0, 5), RED);
    assert_eq!(canvas.pixel(19, 5), RED);
    assert_eq!(canvas.pixel(20, 5), BLUE);
    assert_eq!(canvas.pixel(39, 5), BLUE);
    assert_eq!(canvas.pixel(40, 5), WHITE);
}

#[test]
fn whitespace_between_inline_blocks_is_a_gap() {
    let canvas = render(
        "<span class=\"ib\" style=\"width:20px;height:10px;background-color:red\"></span> <span class=\"ib\" style=\"width:20px;height:10px;background-color:blue\"></span>",
        100,
    );
    let space = text::text_width(&text::default_font(), " ", 16.0);
    assert_eq!(canvas.pixel(19, 5), RED);
    assert_eq!(canvas.pixel(21, 5), WHITE);
    assert_eq!(canvas.pixel((20.0 + space).round() as usize + 1, 5), BLUE);
}

#[test]
fn inline_blocks_wrap_onto_new_lines() {
    let canvas = render(
        "<span class=\"ib\" style=\"width:20px;height:10px;background-color:red\"></span><span class=\"ib\" style=\"width:20px;height:10px;background-color:red\"></span><span class=\"ib\" style=\"width:20px;height:10px;background-color:blue\"></span>",
        50,
    );
    assert_eq!(canvas.pixel(30, 5), RED);
    assert_eq!(canvas.pixel(45, 5), WHITE);
    assert_eq!(canvas.pixel(5, 15), BLUE);
    assert_eq!(canvas.pixel(25, 15), WHITE);
}

#[test]
fn auto_width_shrinks_to_the_content() {
    let canvas = render(
        "<span class=\"ib\" style=\"background-color:red\">abc</span>",
        200,
    );
    let width = text::text_width(&text::default_font(), "abc", 16.0).round() as usize;
    assert_eq!(canvas.pixel(width - 1, 0), RED);
    assert_eq!(canvas.pixel(width + 2, 0), WHITE);
}

#[test]
fn auto_width_is_capped_by_the_container() {
    let canvas = render(
        "<div style=\"width:60px\"><span class=\"ib\" style=\"background-color:red\">one two three four five</span></div>",
        200,
    );
    assert_eq!(canvas.pixel(59, 0), RED);
    assert_eq!(canvas.pixel(61, 0), WHITE);
    assert_eq!(canvas.pixel(30, 40), RED);
}

#[test]
fn margins_padding_and_borders_take_part_in_the_width() {
    let canvas = render(
        "<span class=\"ib\" style=\"margin:0 5px;padding:0 3px;border:2px solid blue;width:10px;height:10px;background-color:red\"></span><span class=\"ib\" style=\"width:5px;height:5px;background-color:red\"></span>",
        100,
    );
    assert_eq!(canvas.pixel(4, 5), WHITE);
    assert_eq!(canvas.pixel(5, 5), BLUE);
    assert_eq!(canvas.pixel(7, 5), RED);
    assert_eq!(canvas.pixel(10, 5), RED);
    assert_eq!(canvas.pixel(21, 5), RED);
    assert_eq!(canvas.pixel(24, 5), BLUE);
    assert_eq!(canvas.pixel(27, 2), WHITE);
    assert_eq!(canvas.pixel(32, 11), RED);
    assert_eq!(canvas.pixel(36, 11), WHITE);
}

#[test]
fn text_baselines_align_with_the_inline_block_baseline() {
    let found = fragments(
        "<span class=\"ib\" style=\"font-size:32px\">Big</span> small",
        300,
    );
    assert_eq!(found.len(), 2);
    assert_eq!(found[0].2, found[1].2);
}

#[test]
fn an_empty_inline_block_rests_on_the_baseline() {
    let found = fragments(
        "<span class=\"ib\" style=\"width:10px;height:10px;background-color:red\"></span>text",
        300,
    );
    let baseline = found[0].2.round() as usize;
    let canvas = render(
        "<span class=\"ib\" style=\"width:10px;height:10px;background-color:red\"></span>text",
        300,
    );
    assert_eq!(canvas.pixel(2, baseline - 1), RED);
    assert_eq!(canvas.pixel(2, baseline), WHITE);
    assert_eq!(canvas.pixel(2, baseline - 11), WHITE);
}

#[test]
fn overflow_hidden_puts_the_bottom_edge_on_the_baseline() {
    let html = "<span class=\"ib\" style=\"overflow:hidden;height:30px;width:10px;background-color:red\">x</span>text";
    let found = fragments(html, 300);
    let text_fragment = found.iter().find(|f| f.0 == "text").unwrap();
    let baseline = text_fragment.2.round() as usize;
    let canvas = render(html, 300);
    assert_eq!(canvas.pixel(8, baseline - 1), RED);
    assert_eq!(canvas.pixel(8, baseline), WHITE);
}

#[test]
fn a_tall_inline_block_grows_the_line() {
    let canvas = render(
        "<div><span class=\"ib\" style=\"width:10px;height:50px;background-color:red\"></span></div><div style=\"width:10px;height:5px;background-color:blue\"></div>",
        100,
    );
    assert_eq!(canvas.pixel(5, 49), RED);
    assert_eq!(canvas.pixel(5, 52), BLUE);
}

#[test]
fn text_align_centers_inline_blocks() {
    let canvas = render(
        "<div style=\"text-align:center;width:100px\"><span class=\"ib\" style=\"width:20px;height:10px;background-color:red\"></span></div>",
        100,
    );
    assert_eq!(canvas.pixel(39, 5), WHITE);
    assert_eq!(canvas.pixel(40, 5), RED);
    assert_eq!(canvas.pixel(59, 5), RED);
    assert_eq!(canvas.pixel(60, 5), WHITE);
}

#[test]
fn inline_blocks_nest_and_hold_block_content() {
    let canvas = render(
        "<span class=\"ib\" style=\"padding:4px;background-color:blue\"><span class=\"ib\" style=\"width:10px;height:10px;background-color:red\"></span></span>",
        100,
    );
    assert_eq!(canvas.pixel(1, 1), BLUE);
    assert_eq!(canvas.pixel(6, 6), RED);
    assert_eq!(canvas.pixel(13, 13), RED);
    assert_eq!(canvas.pixel(16, 6), BLUE);
}

#[test]
fn block_children_stack_inside_an_inline_block() {
    let canvas = render(
        "<span class=\"ib\" style=\"width:20px\"><div style=\"height:5px;background-color:red\"></div><div style=\"height:5px;background-color:blue\"></div></span>",
        100,
    );
    assert_eq!(canvas.pixel(10, 2), RED);
    assert_eq!(canvas.pixel(10, 7), BLUE);
    assert_eq!(canvas.pixel(25, 2), WHITE);
}

#[test]
fn absolute_children_use_the_inline_block_as_their_container() {
    let canvas = render(
        "<span style=\"margin-left:30px\" class=\"ib\"><span class=\"ib\" style=\"position:relative;width:20px;height:20px\"><div style=\"position:absolute;left:2px;top:3px;width:5px;height:5px;background-color:red\"></div></span></span>",
        100,
    );
    assert_eq!(canvas.pixel(31, 4), WHITE);
    assert_eq!(canvas.pixel(33, 4), RED);
    assert_eq!(canvas.pixel(33, 9), WHITE);
}

#[test]
fn relative_offsets_move_an_inline_block() {
    let canvas = render(
        "<span class=\"ib\" style=\"position:relative;left:10px;top:5px;width:10px;height:10px;background-color:red\"></span>",
        100,
    );
    assert_eq!(canvas.pixel(5, 5), WHITE);
    assert_eq!(canvas.pixel(15, 10), RED);
}

#[test]
fn floated_or_absolute_inline_blocks_behave_like_blocks() {
    let canvas = render(
        "<span class=\"ib\" style=\"float:right;width:20px;height:10px;background-color:red\"></span>",
        100,
    );
    assert_eq!(canvas.pixel(85, 5), RED);
    assert_eq!(canvas.pixel(5, 5), WHITE);
}

#[test]
fn inline_blocks_in_a_shrink_to_fit_container_widen_it() {
    let canvas = render(
        "<div style=\"float:left;background-color:blue\"><span class=\"ib\" style=\"width:30px;height:10px;background-color:red\"></span></div>",
        100,
    );
    assert_eq!(canvas.pixel(29, 5), RED);
    assert_eq!(canvas.pixel(31, 5), WHITE);
}
