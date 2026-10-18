// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const GREEN: Color = Color { r: 0, g: 128, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn pixel(style: &str, x: usize, y: usize) -> Color {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }} div {{ width: 20px; height: 20px }} {}</style></head><body><div></div></body></html>",
        style
    );
    Page::new(&html).render(60, 60, 0.0).canvas.pixel(x, y)
}

#[test]
fn border_shorthand_accepts_currentcolor() {
    assert_eq!(
        pixel("div { color: red; border: 4px solid currentcolor }", 1, 10),
        RED
    );
}

#[test]
fn borders_default_to_the_text_color() {
    assert_eq!(pixel("div { color: blue; border: 4px solid }", 1, 10), BLUE);
    assert_eq!(pixel("div { color: blue; border-width: 4px }", 10, 1), BLUE);
    assert_eq!(
        pixel("div { border: 4px solid }", 1, 10),
        Color { r: 0, g: 0, b: 0 }
    );
}

#[test]
fn border_color_follows_the_element_own_color() {
    let html = "<html><head><style>body { margin: 0; color: red } div { width: 20px; height: 20px; border: 4px solid; color: blue }</style></head><body><div></div></body></html>";
    let canvas = Page::new(html).render(60, 60, 0.0).canvas;
    assert_eq!(canvas.pixel(1, 10), BLUE);
}

#[test]
fn explicit_border_color_wins() {
    assert_eq!(
        pixel("div { color: red; border: 4px solid green }", 1, 10),
        GREEN
    );
    assert_eq!(
        pixel(
            "div { color: red; border: 4px solid; border-color: currentcolor }",
            1,
            10
        ),
        RED
    );
}

#[test]
fn background_color_can_be_currentcolor() {
    assert_eq!(
        pixel(
            "div { color: green; background-color: currentcolor }",
            10,
            10
        ),
        GREEN
    );
    assert_eq!(
        pixel("div { color: green; background: currentcolor }", 10, 10),
        GREEN
    );
}

#[test]
fn color_currentcolor_keeps_the_inherited_value() {
    let html = "<html><head><style>body { margin: 0; color: red } div { width: 20px; height: 20px; color: currentcolor; background-color: currentcolor }</style></head><body><div></div></body></html>";
    let canvas = Page::new(html).render(60, 60, 0.0).canvas;
    assert_eq!(canvas.pixel(10, 10), RED);
}

#[test]
fn no_border_is_drawn_without_a_width() {
    assert_eq!(
        pixel("div { color: red; border-color: currentcolor }", 1, 10),
        WHITE
    );
}

#[test]
fn translucent_text_color_gives_a_translucent_border() {
    let color = pixel(
        "div { color: rgba(0, 0, 255, 0.5); border: 4px solid }",
        1,
        10,
    );
    assert!(
        (color.r as i32 - 128).abs() <= 1 && color.b == 255,
        "{:?}",
        color
    );
}
