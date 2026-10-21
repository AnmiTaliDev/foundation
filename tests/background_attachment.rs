// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{parse_declarations, Color, Value};
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const RED_FILL: &str = "linear-gradient(red, red)";
const LIME_FILL: &str = "linear-gradient(lime, lime)";

fn render(style: &str, scroll: f32) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: 100px; height: 1000px; {}\"></div></body></html>",
        style
    );
    Page::new(&html).render(100, 100, scroll).canvas
}

#[test]
fn scroll_attachment_moves_with_the_content() {
    let style = format!("background: {RED_FILL} 0 0 / 10px 10px no-repeat scroll");
    assert_eq!(render(&style, 0.0).pixel(5, 5), RED);
    assert_eq!(render(&style, 300.0).pixel(5, 5), WHITE);
}

#[test]
fn fixed_attachment_stays_in_the_viewport() {
    let style = format!("background: {RED_FILL} 0 0 / 10px 10px no-repeat fixed");
    assert_eq!(render(&style, 0.0).pixel(5, 5), RED);
    let canvas = render(&style, 300.0);
    assert_eq!(canvas.pixel(5, 5), RED);
    assert_eq!(canvas.pixel(5, 50), WHITE);
}

#[test]
fn fixed_attachment_positions_against_the_viewport() {
    let style = format!(
        "background-image: {RED_FILL}; background-size: 10px 10px; background-repeat: no-repeat; background-position: 100% 100%; background-attachment: fixed"
    );
    let canvas = render(&style, 300.0);
    assert_eq!(canvas.pixel(95, 95), RED);
    assert_eq!(canvas.pixel(5, 5), WHITE);
}

#[test]
fn fixed_attachment_is_clipped_to_the_element() {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"margin-top: 40px; width: 100px; height: 20px; background: {RED_FILL} fixed\"></div><div style=\"height: 1000px\"></div></body></html>"
    );
    let canvas = Page::new(&html).render(100, 100, 0.0).canvas;
    assert_eq!(canvas.pixel(50, 20), WHITE);
    assert_eq!(canvas.pixel(50, 50), RED);
    assert_eq!(canvas.pixel(50, 80), WHITE);
}

#[test]
fn fixed_attachment_applies_to_each_layer() {
    let style = format!(
        "background-image: {RED_FILL}, {LIME_FILL}; background-size: 10px 10px; background-repeat: no-repeat; background-position: 0 0, 0 350px; background-attachment: fixed, scroll"
    );
    let canvas = render(&style, 300.0);
    assert_eq!(canvas.pixel(5, 5), RED);
    assert_eq!(canvas.pixel(5, 55), LIME);
}

#[test]
fn local_attachment_behaves_like_scroll() {
    let style = format!("background: {RED_FILL} 0 0 / 10px 10px no-repeat local");
    assert_eq!(render(&style, 300.0).pixel(5, 5), WHITE);
}

#[test]
fn shorthand_sets_the_attachment_longhand() {
    let declarations = parse_declarations(&format!("background: {RED_FILL} fixed"));
    let attachment = declarations
        .iter()
        .find(|declaration| declaration.name == "background-attachment")
        .map(|declaration| declaration.value.clone());
    assert_eq!(attachment, Some(Value::Keyword("fixed".to_string())));
}

#[test]
fn supports_recognizes_the_property() {
    assert!(foundation::supports::evaluate(
        "(background-attachment: fixed)"
    ));
}

#[test]
fn unknown_attachment_is_dropped() {
    let sheet = parse_declarations("background-attachment: sticky");
    assert!(sheet.is_empty());
}
