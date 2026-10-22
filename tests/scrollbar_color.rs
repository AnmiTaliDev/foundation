// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color};
use foundation::Page;

const GREEN: Color = Color { r: 0, g: 128, b: 0 };
const NAVY: Color = Color { r: 0, g: 0, b: 128 };
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
fn two_colors_set_thumb_and_track() {
    let page = page("scrollbar-color: green navy");
    assert_eq!(pixel(&page, 35, 5), GREEN);
    assert_eq!(pixel(&page, 35, 35), NAVY);
}

#[test]
fn auto_keeps_the_default_colors() {
    let page = page("scrollbar-color: auto");
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 35, 35), TRACK);
}

#[test]
fn color_functions_and_hex_work() {
    let page = page("scrollbar-color: #008000 rgb(0, 0, 128)");
    assert_eq!(pixel(&page, 35, 5), GREEN);
    assert_eq!(pixel(&page, 35, 35), NAVY);
}

#[test]
fn colors_are_inherited() {
    let page = Page::new(
        "<html><head><style>body { margin: 0; scrollbar-color: green navy } #a { width: 40px; height: 40px; overflow: auto }</style></head><body><div id=a><div style=\"background: red; height: 80px\"></div></div></body></html>",
    );
    assert_eq!(pixel(&page, 35, 5), GREEN);
    assert_eq!(pixel(&page, 35, 35), NAVY);
}

#[test]
fn auto_resets_inherited_colors() {
    let page = Page::new(
        "<html><head><style>body { margin: 0; scrollbar-color: green navy } #a { width: 40px; height: 40px; overflow: auto; scrollbar-color: auto }</style></head><body><div id=a><div style=\"background: red; height: 80px\"></div></div></body></html>",
    );
    assert_eq!(pixel(&page, 35, 5), THUMB);
    assert_eq!(pixel(&page, 35, 35), TRACK);
}

#[test]
fn colors_apply_to_horizontal_bars() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 40px; height: 40px; overflow-x: scroll; scrollbar-color: green navy }</style></head><body><div id=a><div style=\"background: red; width: 80px; height: 10px\"></div></div></body></html>",
    );
    assert_eq!(pixel(&page, 5, 35), GREEN);
    assert_eq!(pixel(&page, 35, 35), NAVY);
}

#[test]
fn parsing_requires_two_colors_or_auto() {
    let sheet = css::parse(
        "a { scrollbar-color: red } b { scrollbar-color: red blue } c { scrollbar-color: red 4px } d { scrollbar-color: auto } e { scrollbar-color: red blue green }",
    );
    assert!(sheet.rules[0].declarations.is_empty());
    assert_eq!(sheet.rules[1].declarations.len(), 2);
    assert!(sheet.rules[2].declarations.is_empty());
    assert_eq!(sheet.rules[3].declarations.len(), 2);
    assert!(sheet.rules[4].declarations.is_empty());
}
