// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

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
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 40px; height: 40px; {} }}</style></head><body><div id=a><div style=\"background: red; {}\"></div></div><div style=\"height: 400px\"></div></body></html>",
        container, content
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

#[test]
fn pressing_on_the_thumb_grabs_it() {
    let page = page("overflow: auto", "height: 80px");
    let grab = page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).unwrap();
    assert!(grab.vertical);
}

#[test]
fn pressing_on_the_track_or_content_grabs_nothing() {
    let page = page("overflow: auto", "height: 80px");
    assert!(page.scroll_grab(SIZE, 0.0, (35.0, 30.0)).is_none());
    assert!(page.scroll_grab(SIZE, 0.0, (5.0, 5.0)).is_none());
    assert!(page.scroll_grab(SIZE, 0.0, (50.0, 5.0)).is_none());
}

#[test]
fn dragging_moves_the_thumb_with_the_pointer() {
    let mut page = page("overflow: auto", "height: 80px");
    let grab = page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (35.0, 15.0));
    assert_eq!(pixel(&page, 35, 9), TRACK);
    assert_eq!(pixel(&page, 35, 11), THUMB);
    assert_eq!(pixel(&page, 35, 29), THUMB);
    assert_eq!(pixel(&page, 35, 31), TRACK);
}

#[test]
fn dragging_is_relative_to_the_grab_point() {
    let mut page = page("overflow: auto", "height: 80px");
    let grab = page.scroll_grab(SIZE, 0.0, (35.0, 15.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (35.0, 25.0));
    assert_eq!(pixel(&page, 35, 9), TRACK);
    assert_eq!(pixel(&page, 35, 11), THUMB);
}

#[test]
fn dragging_past_the_track_clamps() {
    let mut page = page("overflow: auto", "height: 80px");
    let grab = page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (35.0, 500.0));
    assert_eq!(pixel(&page, 35, 19), TRACK);
    assert_eq!(pixel(&page, 35, 21), THUMB);
    assert_eq!(pixel(&page, 35, 39), THUMB);
    page.scroll_drag(SIZE, 0.0, grab, (35.0, -500.0));
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 35, 21), TRACK);
}

#[test]
fn dragging_to_the_end_leaves_nothing_to_scroll() {
    let mut page = page("overflow: auto", "height: 80px");
    let grab = page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (35.0, 500.0));
    let (_, remaining) = page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 10.0));
    assert_eq!(remaining, 10.0);
}

#[test]
fn horizontal_thumbs_drag_too() {
    let mut page = page("overflow-x: scroll", "width: 80px; height: 10px");
    let grab = page.scroll_grab(SIZE, 0.0, (5.0, 35.0)).unwrap();
    assert!(!grab.vertical);
    page.scroll_drag(SIZE, 0.0, grab, (15.0, 35.0));
    assert_eq!(pixel(&page, 9, 35), TRACK);
    assert_eq!(pixel(&page, 11, 35), THUMB);
}

#[test]
fn page_scroll_is_taken_into_account() {
    let mut page = page("overflow: auto", "height: 80px");
    assert!(page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).is_some());
    assert!(page.scroll_grab(SIZE, 20.0, (35.0, 5.0)).is_none());
    let grab = page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (35.0, 15.0));
    assert_eq!(pixel(&page, 35, 11), THUMB);
}
