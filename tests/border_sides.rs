// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(style: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"margin: 10px; width: 60px; height: 60px; {}\"></div></body></html>",
        style
    );
    Page::new(&html).render(100, 100, 0.0).canvas
}

#[test]
fn sides_take_their_own_color() {
    let canvas =
        render("border: 3px solid red; border-right-color: blue; border-bottom-color: lime");
    assert_eq!(canvas.pixel(40, 11), RED);
    assert_eq!(canvas.pixel(11, 40), RED);
    assert_eq!(canvas.pixel(10 + 3 + 60 + 3 - 2, 40), BLUE);
    assert_eq!(canvas.pixel(40, 10 + 3 + 60 + 3 - 2), LIME);
}

#[test]
fn border_color_shorthand_follows_the_box_order() {
    let canvas = render("border: 3px solid; border-color: red blue lime white");
    assert_eq!(canvas.pixel(40, 11), RED);
    assert_eq!(canvas.pixel(10 + 3 + 60 + 3 - 2, 40), BLUE);
    assert_eq!(canvas.pixel(40, 10 + 3 + 60 + 3 - 2), LIME);
    assert_eq!(canvas.pixel(11, 40), WHITE);
}

#[test]
fn two_values_pair_vertical_and_horizontal() {
    let canvas = render("border: 3px solid; border-color: red blue");
    assert_eq!(canvas.pixel(40, 11), RED);
    assert_eq!(canvas.pixel(40, 10 + 3 + 60 + 3 - 2), RED);
    assert_eq!(canvas.pixel(11, 40), BLUE);
}

#[test]
fn side_shorthand_sets_one_side() {
    let canvas = render("border-left: 4px solid blue");
    assert_eq!(canvas.pixel(11, 40), BLUE);
    assert_eq!(canvas.pixel(13, 40), BLUE);
    assert_eq!(canvas.pixel(14, 40), WHITE);
    assert_eq!(canvas.pixel(40, 10), WHITE);
}

#[test]
fn side_shorthand_overrides_only_its_side() {
    let canvas = render("border: 2px solid red; border-top: 5px dashed blue");
    assert_eq!(canvas.pixel(10, 12), BLUE);
    assert_eq!(canvas.pixel(11, 40), RED);
    assert_eq!(canvas.pixel(40, 10 + 5 + 60 + 1), RED);
}

#[test]
fn later_shorthand_overrides_earlier_side_color() {
    let canvas = render("border: 3px solid; border-top-color: blue; border-color: red");
    assert_eq!(canvas.pixel(40, 11), RED);
}

#[test]
fn missing_color_defaults_to_text_color_per_side() {
    let canvas = render("color: lime; border: 3px solid; border-left-color: red");
    assert_eq!(canvas.pixel(11, 40), RED);
    assert_eq!(canvas.pixel(40, 11), LIME);
}

#[test]
fn currentcolor_is_accepted_per_side() {
    let canvas = render("color: lime; border: 3px solid red; border-top-color: currentcolor");
    assert_eq!(canvas.pixel(40, 11), LIME);
    assert_eq!(canvas.pixel(11, 40), RED);
}

#[test]
fn rgba_side_color_blends() {
    let canvas = render("border: 4px solid; border-top-color: rgba(255, 0, 0, 0.5)");
    let pixel = canvas.pixel(40, 11);
    assert!(pixel.r > 200 && pixel.g > 100 && pixel.g < 160);
}

#[test]
fn sides_keep_their_own_styles() {
    let canvas = render("border: 2px solid red; border-top: 2px dashed blue");
    assert_eq!(canvas.pixel(11, 10), BLUE);
    assert_eq!(canvas.pixel(17, 10), WHITE);
    assert_eq!(canvas.pixel(40, 40), WHITE);
}

#[test]
fn rounded_borders_use_the_first_visible_color() {
    let canvas = render(
        "border: 4px solid; border-top-color: blue; border-right-color: red; border-radius: 10px",
    );
    assert_eq!(canvas.pixel(40, 11), BLUE);
}

#[test]
fn invalid_colors_are_dropped() {
    let sheet =
        css::parse("p { border-color: solid; border-top-color: 5px; border-color: red blue }");
    let names: Vec<&str> = sheet.rules[0]
        .declarations
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(names.len(), 4);
    let sheet = css::parse("p { border-top: 2px solid red }");
    assert_eq!(sheet.rules[0].declarations.len(), 3);
    assert!(sheet.rules[0]
        .declarations
        .iter()
        .any(|d| d.name == "border-top-color" && d.value == Value::Color(RED)));
}

#[test]
fn supports_the_new_properties() {
    assert!(foundation::supports::evaluate("(border-top-color: red)"));
    assert!(foundation::supports::evaluate(
        "(border-left: 1px solid red)"
    ));
}
