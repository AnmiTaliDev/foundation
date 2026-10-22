// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color};
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

fn page(container: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 40px; height: 40px; overflow: auto; {} }}</style></head><body><div id=a><div style=\"background: red; height: 80px\"></div></div></body></html>",
        container
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

#[test]
fn auto_is_eight_pixels() {
    let page = page("scrollbar-width: auto");
    assert_eq!(pixel(&page, 31, 5), RED);
    assert_eq!(pixel(&page, 32, 5), THUMB);
    assert_eq!(pixel(&page, 32, 35), TRACK);
}

#[test]
fn thin_is_four_pixels() {
    let page = page("scrollbar-width: thin");
    assert_eq!(pixel(&page, 35, 5), RED);
    assert_eq!(pixel(&page, 36, 5), THUMB);
    assert_eq!(pixel(&page, 39, 5), THUMB);
    assert_eq!(pixel(&page, 36, 35), TRACK);
}

#[test]
fn none_hides_the_bars_but_keeps_scrolling() {
    let mut page = page("scrollbar-width: none");
    assert_eq!(pixel(&page, 35, 5), RED);
    assert_eq!(pixel(&page, 39, 5), RED);
    let (_, remaining) = page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 20.0));
    assert_eq!(remaining, 0.0);
    assert!(page.scroll_grab(SIZE, 0.0, (35.0, 5.0)).is_none());
}

#[test]
fn none_also_hides_always_visible_bars() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 40px; height: 40px; overflow: scroll; scrollbar-width: none }</style></head><body><div id=a><div style=\"background: red; height: 10px\"></div></div></body></html>",
    );
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(
        pixel(&page, 39, 20),
        Color {
            r: 255,
            g: 255,
            b: 255
        }
    );
}

#[test]
fn thin_bars_can_be_dragged() {
    let mut page = page("scrollbar-width: thin");
    let grab = page.scroll_grab(SIZE, 0.0, (37.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (37.0, 15.0));
    assert_eq!(pixel(&page, 37, 9), TRACK);
    assert_eq!(pixel(&page, 37, 11), THUMB);
}

#[test]
fn parsing_drops_unknown_values() {
    let sheet = css::parse(
        "a { scrollbar-width: wide } b { scrollbar-width: thin } c { scrollbar-width: 4px } d { scrollbar-width: none }",
    );
    assert!(sheet.rules[0].declarations.is_empty());
    assert_eq!(sheet.rules[1].declarations.len(), 1);
    assert!(sheet.rules[2].declarations.is_empty());
    assert_eq!(sheet.rules[3].declarations.len(), 1);
}
