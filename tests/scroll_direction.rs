// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };
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

fn page(direction: &str, extra: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 40px; height: 20px; overflow-x: scroll; direction: {} }} #w {{ width: 100px; direction: ltr; height: 10px }} #w div {{ float: left; width: 50px; height: 10px }} {}</style></head><body><div id=a><div id=w><div style=\"background: blue\"></div><div style=\"background: red\"></div></div></div></body></html>",
        direction, extra
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

fn wheel(page: &mut Page, dx: f32) -> (f32, f32) {
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (dx, 0.0))
}

#[test]
fn ltr_containers_start_at_the_left() {
    let mut page = page("ltr", "");
    assert_eq!(pixel(&page, 5, 5), BLUE);
    wheel(&mut page, 30.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 25, 5), RED);
}

#[test]
fn rtl_containers_start_at_the_right() {
    let page = page("rtl", "");
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 35, 5), RED);
}

#[test]
fn scrolling_left_reveals_the_overflow() {
    let mut page = page("rtl", "");
    assert_eq!(wheel(&mut page, -30.0), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 25, 5), RED);
}

#[test]
fn the_range_is_limited_to_the_overflow() {
    let mut page = page("rtl", "");
    wheel(&mut page, -500.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 39, 5), BLUE);
    assert_eq!(wheel(&mut page, -10.0), (-10.0, 0.0));
}

#[test]
fn scrolling_right_at_the_origin_passes_the_scroll_on() {
    let mut page = page("rtl", "");
    assert_eq!(wheel(&mut page, 10.0), (10.0, 0.0));
    wheel(&mut page, -30.0);
    assert_eq!(wheel(&mut page, 10.0), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 15, 5), RED);
    assert_eq!(pixel(&page, 35, 5), RED);
}

#[test]
fn the_thumb_starts_at_the_right_end() {
    let mut page = page("rtl", "");
    assert_eq!(pixel(&page, 30, 16), THUMB);
    assert_eq!(pixel(&page, 5, 16), TRACK);
    wheel(&mut page, -500.0);
    assert_eq!(pixel(&page, 5, 16), THUMB);
    assert_eq!(pixel(&page, 35, 16), TRACK);
}

#[test]
fn dragging_the_thumb_moves_against_the_start() {
    let mut page = page("rtl", "");
    let grab = page.scroll_grab(SIZE, 0.0, (30.0, 16.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (10.0, 16.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 25, 5), BLUE);
    assert_eq!(pixel(&page, 39, 5), BLUE);
    page.scroll_drag(SIZE, 0.0, grab, (500.0, 16.0));
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn sticky_boxes_use_the_visible_port() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 40px; height: 20px; overflow-x: scroll; direction: rtl } #w { width: 100px; direction: ltr; height: 10px } #s { position: sticky; left: 0; width: 6px; height: 6px; background: lime }</style></head><body><div id=a><div id=w><div id=s></div></div></div></body></html>",
    );
    assert_eq!(pixel(&page, 2, 2), LIME);
    assert_eq!(
        pixel(&page, 7, 2),
        foundation::css::Color {
            r: 255,
            g: 255,
            b: 255
        }
    );
}

#[test]
fn sticky_ports_skip_the_left_bar() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 40px; height: 30px; overflow: scroll; direction: rtl } #w { width: 100px; direction: ltr; height: 40px } #s { position: sticky; left: 0; width: 6px; height: 6px; background: lime }</style></head><body><div id=a><div id=w><div id=s></div></div></div></body></html>",
    );
    assert_eq!(pixel(&page, 9, 2), LIME);
    assert_eq!(pixel(&page, 4, 2), THUMB);
}

#[test]
fn content_that_fits_does_not_scroll() {
    let mut page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 40px; height: 20px; overflow-x: auto; direction: rtl } #w { width: 30px; height: 10px; background: red }</style></head><body><div id=a><div id=w></div></div></body></html>",
    );
    assert_eq!(wheel(&mut page, -20.0), (-20.0, 0.0));
}
