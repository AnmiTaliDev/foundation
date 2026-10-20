// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(style: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"margin: 30px; width: 40px; height: 40px; padding: 10px; border: 5px solid rgba(0, 0, 0, 0); {}\"></div></body></html>",
        style
    );
    Page::new(&html).render(130, 130, 0.0).canvas
}

const TILE: &str = "background-image: linear-gradient(red, red); background-size: 10px 10px; background-repeat: no-repeat;";

#[test]
fn background_covers_the_border_box_by_default() {
    let canvas = render("background-color: lime");
    assert_eq!(canvas.pixel(32, 50), LIME);
    assert_eq!(canvas.pixel(37, 50), LIME);
    assert_eq!(canvas.pixel(47, 50), LIME);
    assert_eq!(canvas.pixel(28, 50), WHITE);
}

#[test]
fn clip_to_padding_box_excludes_the_border() {
    let canvas = render("background-color: lime; background-clip: padding-box");
    assert_eq!(canvas.pixel(32, 50), WHITE);
    assert_eq!(canvas.pixel(37, 50), LIME);
    assert_eq!(canvas.pixel(97, 50), WHITE);
}

#[test]
fn clip_to_content_box_excludes_border_and_padding() {
    let canvas = render("background-color: lime; background-clip: content-box");
    assert_eq!(canvas.pixel(32, 50), WHITE);
    assert_eq!(canvas.pixel(37, 50), WHITE);
    assert_eq!(canvas.pixel(47, 50), LIME);
    assert_eq!(canvas.pixel(87, 50), WHITE);
}

#[test]
fn clip_applies_to_images() {
    let canvas =
        render("background-image: linear-gradient(red, red); background-clip: content-box");
    assert_eq!(canvas.pixel(37, 50), WHITE);
    assert_eq!(canvas.pixel(47, 50), RED);
}

#[test]
fn origin_defaults_to_the_padding_box() {
    let canvas = render(TILE);
    assert_eq!(canvas.pixel(37, 37), RED);
    assert_eq!(canvas.pixel(32, 32), WHITE);
    assert_eq!(canvas.pixel(47, 47), WHITE);
}

#[test]
fn origin_border_box_moves_the_image_to_the_corner() {
    let canvas = render(&format!("{} background-origin: border-box", TILE));
    assert_eq!(canvas.pixel(32, 32), RED);
    assert_eq!(canvas.pixel(42, 42), WHITE);
}

#[test]
fn origin_content_box_moves_the_image_inside_the_padding() {
    let canvas = render(&format!("{} background-origin: content-box", TILE));
    assert_eq!(canvas.pixel(47, 47), RED);
    assert_eq!(canvas.pixel(56, 56), WHITE);
    assert_eq!(canvas.pixel(37, 37), WHITE);
}

#[test]
fn origin_sets_the_positioning_area_for_percentages() {
    let canvas = render(&format!(
        "{} background-origin: content-box; background-position: 100% 100%",
        TILE
    ));
    assert_eq!(canvas.pixel(80, 80), RED);
    assert_eq!(canvas.pixel(47, 47), WHITE);
}

#[test]
fn origin_and_clip_are_independent() {
    let canvas = render(&format!(
        "{} background-origin: border-box; background-clip: content-box",
        TILE
    ));
    assert_eq!(canvas.pixel(32, 32), WHITE);
    assert_eq!(canvas.pixel(47, 47), WHITE);
}

#[test]
fn shorthand_with_one_box_sets_origin_and_clip() {
    let canvas =
        render("background: linear-gradient(red, red) 0 0 / 10px 10px no-repeat content-box");
    assert_eq!(canvas.pixel(47, 47), RED);
    assert_eq!(canvas.pixel(37, 37), WHITE);
}

#[test]
fn shorthand_with_two_boxes_sets_origin_then_clip() {
    let canvas = render("background: lime border-box content-box");
    assert_eq!(canvas.pixel(32, 50), WHITE);
    assert_eq!(canvas.pixel(47, 50), LIME);
    let image = render("background: red 0 0 / 10px 10px no-repeat padding-box border-box");
    assert_eq!(image.pixel(32, 50), RED);
}

#[test]
fn rounded_corners_follow_the_clip_box() {
    let border = render("background-color: lime; border-radius: 20px");
    assert_eq!(border.pixel(36, 40), LIME);
    let padding =
        render("background-color: lime; border-radius: 20px; background-clip: padding-box");
    assert_eq!(padding.pixel(36, 40), WHITE);
    assert_eq!(padding.pixel(50, 50), LIME);
}

#[test]
fn invalid_values_are_dropped() {
    let canvas = render("background-color: lime; background-clip: text");
    assert_eq!(canvas.pixel(32, 50), LIME);
    let canvas =
        render("background-color: lime; background-clip: padding-box; background-clip: 5px");
    assert_eq!(canvas.pixel(32, 50), WHITE);
}

#[test]
fn supports_the_new_properties() {
    assert!(foundation::supports::evaluate(
        "(background-clip: padding-box)"
    ));
    assert!(foundation::supports::evaluate(
        "(background-origin: content-box)"
    ));
}
