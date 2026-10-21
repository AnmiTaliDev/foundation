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
fn overflowing_auto_shows_a_vertical_bar() {
    let page = page("overflow: auto", "height: 80px");
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 35, 19), THUMB);
    assert_eq!(pixel(&page, 35, 21), TRACK);
    assert_eq!(pixel(&page, 35, 39), TRACK);
}

#[test]
fn thumb_follows_the_scroll_offset() {
    let mut page = page("overflow: auto", "height: 80px");
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 20.0));
    assert_eq!(pixel(&page, 35, 5), TRACK);
    assert_eq!(pixel(&page, 35, 9), TRACK);
    assert_eq!(pixel(&page, 35, 11), THUMB);
    assert_eq!(pixel(&page, 35, 29), THUMB);
    assert_eq!(pixel(&page, 35, 31), TRACK);
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 100.0));
    assert_eq!(pixel(&page, 35, 21), THUMB);
    assert_eq!(pixel(&page, 35, 39), THUMB);
    assert_eq!(pixel(&page, 35, 19), TRACK);
}

#[test]
fn auto_without_overflow_shows_no_bar() {
    let page = page("overflow: auto", "height: 20px");
    assert_eq!(pixel(&page, 35, 5), RED);
}

#[test]
fn scroll_always_shows_the_bars() {
    let page = page("overflow: scroll", "height: 20px");
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 35, 29), THUMB);
    assert_eq!(pixel(&page, 5, 35), THUMB);
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn horizontal_bar_sits_at_the_bottom() {
    let page = page(
        "overflow-x: auto; overflow-y: hidden",
        "width: 80px; height: 20px",
    );
    assert_eq!(pixel(&page, 5, 35), THUMB);
    assert_eq!(pixel(&page, 19, 35), THUMB);
    assert_eq!(pixel(&page, 21, 35), TRACK);
    assert_eq!(pixel(&page, 35, 5), RED);
}

#[test]
fn both_bars_leave_the_corner_empty() {
    let page = page("overflow: auto", "width: 80px; height: 80px");
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 5, 35), THUMB);
    assert_eq!(pixel(&page, 35, 35), RED);
}

#[test]
fn hidden_containers_have_no_bars() {
    let page = page("overflow: hidden", "height: 80px");
    assert_eq!(pixel(&page, 35, 5), RED);
}

#[test]
fn bars_stay_inside_the_padding_box() {
    let page = page(
        "overflow: auto; border: 2px solid lime; padding: 3px; width: 34px; height: 34px",
        "height: 80px",
    );
    assert_eq!(pixel(&page, 1, 5), Color { r: 0, g: 255, b: 0 });
    assert_eq!(pixel(&page, 43, 5), Color { r: 0, g: 255, b: 0 });
    assert_eq!(pixel(&page, 40, 5), THUMB);
}
