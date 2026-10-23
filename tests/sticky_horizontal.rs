// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const SIZE: (usize, usize) = (60, 60);

fn page(body: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 40px; height: 20px; overflow-x: scroll }} #s {{ position: sticky; width: 10px; height: 10px; background: red }}</style></head><body>{}</body></html>",
        body
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

fn scroll(page: &mut Page, amount: f32) {
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (amount, 0.0));
}

#[test]
fn a_left_sticky_box_stays_at_the_left_edge() {
    let mut page = page(
        "<div id=a><div style=\"width: 200px\"><div id=s style=\"left: 0\"></div></div></div>",
    );
    assert_eq!(pixel(&page, 5, 5), RED);
    scroll(&mut page, 30.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 15, 5), WHITE);
    scroll(&mut page, 100.0);
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn without_sticky_the_box_scrolls_away() {
    let mut page = page("<div id=a><div style=\"width: 200px\"><div id=s style=\"position: static\"></div></div></div>");
    scroll(&mut page, 30.0);
    assert_eq!(pixel(&page, 5, 5), WHITE);
}

#[test]
fn the_left_offset_is_respected() {
    let mut page = page(
        "<div id=a><div style=\"width: 200px\"><div id=s style=\"left: 5px\"></div></div></div>",
    );
    scroll(&mut page, 30.0);
    assert_eq!(pixel(&page, 4, 5), WHITE);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 14, 5), RED);
    assert_eq!(pixel(&page, 15, 5), WHITE);
}

#[test]
fn a_right_sticky_box_stays_at_the_right_edge() {
    let page = page("<div id=a><div style=\"display: flex; width: 200px\"><div style=\"width: 150px; height: 10px\"></div><div id=s style=\"right: 0\"></div></div></div>");
    assert_eq!(pixel(&page, 25, 5), WHITE);
    assert_eq!(pixel(&page, 35, 5), RED);
    assert_eq!(pixel(&page, 39, 5), RED);
}

#[test]
fn the_vertical_scrollbar_narrows_the_port() {
    let page = page("<div id=a style=\"overflow-y: scroll\"><div style=\"display: flex; width: 200px\"><div style=\"width: 150px; height: 10px\"></div><div id=s style=\"right: 0\"></div></div></div>");
    assert_eq!(pixel(&page, 21, 5), WHITE);
    assert_eq!(pixel(&page, 22, 5), RED);
    assert_eq!(pixel(&page, 31, 5), RED);
}

#[test]
fn a_right_sticky_box_returns_to_its_place_when_scrolled_into_view() {
    let mut page = page("<div id=a><div style=\"display: flex; width: 200px\"><div style=\"width: 150px; height: 10px\"></div><div id=s style=\"right: 0\"></div></div></div>");
    scroll(&mut page, 120.0);
    assert_eq!(pixel(&page, 25, 5), WHITE);
    assert_eq!(pixel(&page, 30, 5), RED);
    assert_eq!(pixel(&page, 39, 5), RED);
}

#[test]
fn the_box_stays_inside_its_containing_block() {
    let mut page = page("<div id=a><div style=\"width: 200px\"><div style=\"width: 50px\"><div id=s style=\"left: 0\"></div></div></div></div>");
    scroll(&mut page, 45.0);
    assert_eq!(pixel(&page, 2, 5), RED);
    assert_eq!(pixel(&page, 6, 5), WHITE);
}

#[test]
fn a_negative_margin_box_sticks_inside_the_viewport() {
    let page = page("<div id=s style=\"left: 0; margin-left: -10px\"></div>");
    assert_eq!(pixel(&page, 2, 5), RED);
    assert_eq!(pixel(&page, 12, 5), WHITE);
}

#[test]
fn table_cells_can_stick() {
    let mut page = page("<div id=a><table style=\"width: 200px; border-spacing: 0\"><tr><td id=s style=\"left: 0; padding: 0\"></td><td>x</td></tr></table></div>");
    scroll(&mut page, 50.0);
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn horizontal_and_vertical_sticky_combine() {
    let mut page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 40px; height: 40px; overflow: auto } #s { position: sticky; left: 0; top: 0; width: 10px; height: 10px; background: red }</style></head><body><div id=a><div style=\"width: 200px; height: 200px\"><div id=s></div></div></div></body></html>",
    );
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (30.0, 30.0));
    assert_eq!(pixel(&page, 5, 5), RED);
}
