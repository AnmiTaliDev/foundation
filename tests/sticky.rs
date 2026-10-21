// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn page(body: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} .s {{ position: sticky; top: 0; height: 10px; background-color: red }} .f {{ height: 40px }}</style></head><body>{}</body></html>",
        body
    ))
}

fn pixel(page: &Page, scroll: f32, x: usize, y: usize) -> Color {
    page.render(40, 50, scroll).canvas.pixel(x, y)
}

#[test]
fn sticky_scrolls_normally_until_it_reaches_its_offset() {
    let page =
        page("<div class=\"f\"></div><div class=\"s\"></div><div style=\"height:400px\"></div>");
    assert_eq!(pixel(&page, 0.0, 5, 45), RED);
    assert_eq!(pixel(&page, 0.0, 5, 35), WHITE);
    assert_eq!(pixel(&page, 20.0, 5, 25), RED);
    assert_eq!(pixel(&page, 20.0, 5, 15), WHITE);
}

#[test]
fn sticky_sticks_to_the_top_of_the_viewport() {
    let page =
        page("<div class=\"f\"></div><div class=\"s\"></div><div style=\"height:400px\"></div>");
    assert_eq!(pixel(&page, 60.0, 5, 0), RED);
    assert_eq!(pixel(&page, 60.0, 5, 9), RED);
    assert_eq!(pixel(&page, 60.0, 5, 10), WHITE);
    assert_eq!(pixel(&page, 200.0, 5, 5), RED);
}

#[test]
fn sticky_top_offset_leaves_a_gap() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; top: 8px; height: 10px; background-color: red }</style></head><body><div class=\"s\"></div><div style=\"height:400px\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 100.0, 5, 7), WHITE);
    assert_eq!(pixel(&page, 100.0, 5, 8), RED);
    assert_eq!(pixel(&page, 100.0, 5, 17), RED);
    assert_eq!(pixel(&page, 100.0, 5, 18), WHITE);
}

#[test]
fn sticky_stays_inside_its_containing_block() {
    let page = page(
        "<div style=\"height:40px\"><div class=\"s\"></div></div><div style=\"height:400px;background-color:blue\"></div>",
    );
    assert_eq!(pixel(&page, 20.0, 5, 5), RED);
    assert_eq!(pixel(&page, 20.0, 5, 25), BLUE);
    assert_eq!(pixel(&page, 100.0, 5, 5), BLUE);
}

#[test]
fn sticky_bottom_pins_to_the_bottom_edge() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; bottom: 0; height: 10px; background-color: red }</style></head><body><div style=\"height:200px\"></div><div class=\"s\"></div><div style=\"height:20px\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 0.0, 5, 49), RED);
    assert_eq!(pixel(&page, 0.0, 5, 40), RED);
    assert_eq!(pixel(&page, 0.0, 5, 39), WHITE);
    assert_eq!(pixel(&page, 180.0, 5, 19), WHITE);
    assert_eq!(pixel(&page, 180.0, 5, 20), RED);
    assert_eq!(pixel(&page, 180.0, 5, 29), RED);
    assert_eq!(pixel(&page, 180.0, 5, 30), WHITE);
}

#[test]
fn sticky_without_offsets_does_not_move() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; height: 10px; background-color: red }</style></head><body><div style=\"height:40px\"></div><div class=\"s\"></div><div style=\"height:400px\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 60.0, 5, 0), WHITE);
}

#[test]
fn sticky_content_and_descendants_move_together() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; top: 0; height: 20px; background-color: blue }</style></head><body><div style=\"height:40px\"></div><div class=\"s\"><div style=\"width:5px;height:5px;background-color:red\"></div></div><div style=\"height:400px\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 100.0, 2, 2), RED);
    assert_eq!(pixel(&page, 100.0, 10, 2), BLUE);
    assert_eq!(pixel(&page, 100.0, 2, 22), WHITE);
}

#[test]
fn fixed_descendants_of_a_sticky_box_do_not_shift() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; top: 0; height: 20px }</style></head><body><div style=\"height:40px\"></div><div class=\"s\"><div style=\"position:fixed;left:30px;top:30px;width:5px;height:5px;background-color:red\"></div></div><div style=\"height:400px\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 100.0, 31, 31), RED);
    assert_eq!(pixel(&page, 0.0, 31, 31), RED);
}

#[test]
fn sticky_is_painted_above_later_static_content() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; top: 0; height: 10px; background-color: red }</style></head><body><div style=\"height:20px\"></div><div class=\"s\"></div><div style=\"height:400px;background-color:blue\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 100.0, 5, 5), RED);
    assert_eq!(pixel(&page, 100.0, 5, 20), BLUE);
}

fn nested() -> Page {
    page(
        "<div class=\"s\" style=\"height:300px;background-color:blue\"><div style=\"height:20px\"></div><div class=\"s\"></div></div><div style=\"height:800px\"></div>",
    )
}

#[test]
fn nested_sticky_moves_with_its_sticky_parent() {
    let page = nested();
    assert_eq!(pixel(&page, 0.0, 5, 25), RED);
    assert_eq!(pixel(&page, 100.0, 5, 25), RED);
    assert_eq!(pixel(&page, 100.0, 5, 15), BLUE);
    assert_eq!(pixel(&page, 100.0, 5, 35), BLUE);
}

#[test]
fn nested_sticky_applies_its_own_offset_inside_the_parent() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .o { position: sticky; top: 0; height: 300px; background-color: blue } .i { position: sticky; top: 30px; height: 10px; background-color: red }</style></head><body><div class=\"o\"><div style=\"height:10px\"></div><div class=\"i\"></div></div><div style=\"height:800px\"></div></body></html>",
    );
    assert_eq!(pixel(&page, 100.0, 5, 25), BLUE);
    assert_eq!(pixel(&page, 100.0, 5, 35), RED);
    assert_eq!(pixel(&page, 100.0, 5, 45), BLUE);
}
