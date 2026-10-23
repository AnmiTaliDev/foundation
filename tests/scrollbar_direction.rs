// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const TRACK: Color = Color {
    r: 241,
    g: 241,
    b: 241,
};
const THUMB: Color = Color {
    r: 193,
    g: 193,
    b: 193,
};

const SIZE: (usize, usize) = (60, 60);

fn page(container: &str, content: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 40px; height: 40px; {} }}</style></head><body><div id=a><div style=\"background: red; {}\"></div></div></body></html>",
        container, content
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

#[test]
fn ltr_vertical_bars_sit_on_the_right() {
    let page = page("overflow: auto", "height: 80px");
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn rtl_vertical_bars_sit_on_the_left() {
    let page = page("overflow: auto; direction: rtl", "height: 80px");
    assert_eq!(pixel(&page, 3, 5), THUMB);
    assert_eq!(pixel(&page, 3, 35), TRACK);
    assert_eq!(pixel(&page, 35, 5), RED);
}

#[test]
fn the_corner_moves_the_horizontal_bar_in_rtl() {
    let page = page(
        "overflow: scroll; direction: rtl",
        "height: 80px; width: 80px",
    );
    assert_eq!(pixel(&page, 3, 35), RED);
    assert_eq!(pixel(&page, 12, 35), TRACK);
    assert_eq!(pixel(&page, 35, 35), THUMB);
    assert_eq!(pixel(&page, 3, 5), THUMB);
}

#[test]
fn thumbs_are_grabbed_where_they_are_drawn() {
    let page = page("overflow: auto; direction: rtl", "height: 80px");
    assert!(page.scroll_grab(SIZE, 0.0, (3.0, 5.0)).is_some());
    assert!(page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).is_none());
}

#[test]
fn dragging_works_on_the_left_bar() {
    let mut page = page("overflow: auto; direction: rtl", "height: 80px");
    let grab = page.scroll_grab(SIZE, 0.0, (3.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (3.0, 15.0));
    assert_eq!(pixel(&page, 3, 9), TRACK);
    assert_eq!(pixel(&page, 3, 11), THUMB);
}
