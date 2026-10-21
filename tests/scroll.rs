// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const SIZE: (usize, usize) = (60, 60);

fn bands() -> &'static str {
    "<div style=\"height: 20px; background: red\"></div><div style=\"height: 20px; background: blue\"></div><div style=\"height: 20px; background: lime\"></div>"
}

fn page(container: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 20px; height: 20px; {} }}</style></head><body><div id=a>{}</div><div style=\"height: 400px\"></div></body></html>",
        container,
        bands()
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

fn wheel(page: &mut Page, point: (f32, f32), delta: (f32, f32)) -> (f32, f32) {
    page.scroll_by(SIZE, 0.0, point, delta)
}

#[test]
fn container_starts_at_the_top() {
    let page = page("overflow: auto");
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn scrolling_moves_the_content_and_consumes_the_delta() {
    let mut page = page("overflow: auto");
    assert_eq!(wheel(&mut page, (5.0, 5.0), (0.0, 20.0)), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(wheel(&mut page, (5.0, 5.0), (0.0, 20.0)), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn offset_is_clamped_and_the_rest_is_returned() {
    let mut page = page("overflow: auto");
    assert_eq!(wheel(&mut page, (5.0, 5.0), (0.0, 500.0)), (0.0, 460.0));
    assert_eq!(pixel(&page, 5, 5), LIME);
    assert_eq!(wheel(&mut page, (5.0, 5.0), (0.0, -500.0)), (0.0, -460.0));
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn content_stays_clipped_to_the_container() {
    let mut page = page("overflow: auto");
    wheel(&mut page, (5.0, 5.0), (0.0, 20.0));
    assert_eq!(pixel(&page, 5, 25), WHITE);
    assert_eq!(pixel(&page, 25, 5), WHITE);
}

#[test]
fn points_outside_the_container_leave_it_alone() {
    let mut page = page("overflow: auto");
    assert_eq!(wheel(&mut page, (40.0, 5.0), (0.0, 20.0)), (0.0, 20.0));
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn scroll_value_scrolls_and_hidden_does_not() {
    let mut scroll = page("overflow: scroll");
    assert_eq!(wheel(&mut scroll, (5.0, 5.0), (0.0, 20.0)), (0.0, 0.0));
    assert_eq!(pixel(&scroll, 5, 5), BLUE);
    let mut hidden = page("overflow: hidden");
    assert_eq!(wheel(&mut hidden, (5.0, 5.0), (0.0, 20.0)), (0.0, 20.0));
    assert_eq!(pixel(&hidden, 5, 5), RED);
}

#[test]
fn axes_are_independent() {
    let mut page = page("overflow-x: hidden; overflow-y: auto");
    assert_eq!(wheel(&mut page, (5.0, 5.0), (7.0, 20.0)), (7.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
}

#[test]
fn horizontal_scrolling_moves_wide_content() {
    let source = "<html><head><style>body { margin: 0 } #a { width: 20px; height: 20px; overflow: auto }</style></head><body><div id=a><div style=\"width: 60px; height: 20px; background: linear-gradient(to right, red 33%, blue 33% 66%, lime 66%)\"></div></div></body></html>";
    let mut page = Page::new(source);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(wheel(&mut page, (5.0, 5.0), (25.0, 0.0)), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
}

#[test]
fn nested_containers_scroll_the_innermost_first_then_the_parent() {
    let source = "<html><head><style>body { margin: 0 } #o { width: 20px; height: 20px; overflow: auto } #i { width: 20px; height: 20px; overflow: auto }</style></head><body><div id=o><div id=i><div style=\"height: 40px; background: red\"></div><div style=\"height: 20px; background: blue\"></div></div><div style=\"height: 20px; background: lime\"></div></div></body></html>";
    let mut page = Page::new(source);
    assert_eq!(wheel(&mut page, (5.0, 5.0), (0.0, 50.0)), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(wheel(&mut page, (5.0, 5.0), (0.0, 20.0)), (0.0, 10.0));
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn page_scroll_and_container_scroll_combine() {
    let mut page = page("overflow: auto");
    let remaining = page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 100.0));
    assert_eq!(remaining, (0.0, 60.0));
    let frame = page.render(SIZE.0, SIZE.1, 10.0);
    assert_eq!(frame.canvas.pixel(5, 0), LIME);
}

#[test]
fn container_scroll_follows_the_page_scroll_when_hit_testing() {
    let mut page = page("overflow: auto");
    assert_eq!(
        page.scroll_by(SIZE, 30.0, (5.0, 5.0), (0.0, 20.0)),
        (0.0, 20.0)
    );
    assert_eq!(
        page.scroll_by(SIZE, 10.0, (5.0, 5.0), (0.0, 20.0)),
        (0.0, 0.0)
    );
}

#[test]
fn offset_survives_rerendering() {
    let mut page = page("overflow: auto");
    wheel(&mut page, (5.0, 5.0), (0.0, 20.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 5, 5), BLUE);
}

#[test]
fn borders_and_padding_stay_in_place() {
    let mut page =
        page("overflow: auto; border: 2px solid lime; padding: 3px; width: 14px; height: 14px");
    wheel(&mut page, (8.0, 8.0), (0.0, 20.0));
    assert_eq!(pixel(&page, 0, 0), LIME);
    assert_eq!(pixel(&page, 8, 8), BLUE);
}
