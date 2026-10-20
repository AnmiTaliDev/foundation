// SPDX-License-Identifier: GPL-3.0-only

use std::fs;

use foundation::css::{self, Color, Value};
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const RED_FILL: &str = "linear-gradient(red, red)";
const LIME_FILL: &str = "linear-gradient(lime, lime)";
const BLUE_FILL: &str = "linear-gradient(blue, blue)";

fn render(style: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"margin: 30px; width: 40px; height: 40px; {}\"></div></body></html>",
        style
    );
    Page::new(&html).render(130, 130, 0.0).canvas
}

#[test]
fn first_layer_paints_above_the_next() {
    let canvas = render(&format!(
        "background: {RED_FILL} 0 0 / 20px 20px no-repeat, {LIME_FILL} 10px 10px / 20px 20px no-repeat"
    ));
    assert_eq!(canvas.pixel(35, 35), RED);
    assert_eq!(canvas.pixel(45, 45), RED);
    assert_eq!(canvas.pixel(55, 55), LIME);
    assert_eq!(canvas.pixel(35, 55), WHITE);
}

#[test]
fn shorthand_color_comes_from_the_last_layer_and_sits_below() {
    let canvas = render(&format!(
        "background: {RED_FILL} 0 0 / 10px 10px no-repeat, {BLUE_FILL} 10px 0 / 10px 10px no-repeat, lime"
    ));
    assert_eq!(canvas.pixel(32, 32), RED);
    assert_eq!(canvas.pixel(45, 32), BLUE);
    assert_eq!(canvas.pixel(60, 60), LIME);
}

#[test]
fn longhand_lists_apply_per_layer() {
    let canvas = render(&format!(
        "background-image: {RED_FILL}, {LIME_FILL}; background-size: 20px 20px; background-repeat: no-repeat; background-position: 0 0, 20px 20px"
    ));
    assert_eq!(canvas.pixel(35, 35), RED);
    assert_eq!(canvas.pixel(55, 55), LIME);
    assert_eq!(canvas.pixel(35, 55), WHITE);
}

#[test]
fn shorter_lists_repeat_cyclically() {
    let canvas = render(&format!(
        "background-image: {RED_FILL}, {LIME_FILL}, {BLUE_FILL}; background-size: 10px 10px; background-repeat: no-repeat; background-position: 0 0, 10px 0"
    ));
    assert_eq!(canvas.pixel(32, 32), RED);
    assert_eq!(canvas.pixel(42, 32), LIME);
    assert_eq!(canvas.pixel(52, 32), WHITE);
}

#[test]
fn repeat_and_size_differ_per_layer() {
    let canvas = render(&format!(
        "background-image: {RED_FILL}, {LIME_FILL}; background-size: 10px 10px, 40px 10px; background-repeat: repeat-y, no-repeat"
    ));
    assert_eq!(canvas.pixel(35, 65), RED);
    assert_eq!(canvas.pixel(50, 35), LIME);
    assert_eq!(canvas.pixel(50, 55), WHITE);
}

#[test]
fn clip_and_origin_differ_per_layer() {
    let canvas = render(&format!(
        "padding: 10px; background-image: {RED_FILL}, {LIME_FILL}; background-clip: content-box, border-box; background-size: 100% 100%"
    ));
    assert_eq!(canvas.pixel(32, 50), LIME);
    assert_eq!(canvas.pixel(45, 50), RED);
}

#[test]
fn missing_layer_properties_get_initial_values_in_the_shorthand() {
    let canvas = render(&format!(
        "background: {RED_FILL} 0 0 / 10px 10px no-repeat, {LIME_FILL}"
    ));
    assert_eq!(canvas.pixel(32, 32), RED);
    assert_eq!(canvas.pixel(50, 50), LIME);
}

#[test]
fn an_invalid_layer_drops_the_declaration() {
    let canvas = render(&format!(
        "background-image: {RED_FILL}; background-image: {LIME_FILL}, 5px"
    ));
    assert_eq!(canvas.pixel(50, 50), RED);
}

#[test]
fn a_single_value_still_parses_as_before() {
    let sheet = css::parse("p { background-image: url(a.png); background-repeat: no-repeat }");
    let declarations = &sheet.rules[0].declarations;
    assert_eq!(declarations[0].value, Value::Url("a.png".to_string()));
    assert_eq!(
        declarations[1].value,
        Value::Keyword("no-repeat".to_string())
    );
}

#[test]
fn layers_with_images_load_every_url() {
    let directory = std::env::temp_dir().join(format!("foundation-layers-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let mut image = image::RgbaImage::new(2, 2);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = if (x + y) % 2 == 0 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 255, 0, 255])
        };
    }
    image.save(directory.join("t.png")).unwrap();
    fs::write(
        directory.join("index.html"),
        "<html><head><style>body { margin: 0 } div { width: 10px; height: 10px; background: url(t.png) 0 0 no-repeat, url(t.png) 5px 5px no-repeat }</style></head><body><div></div></body></html>",
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(20, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 0), GREEN);
    assert_eq!(canvas.pixel(5, 5), RED);
    assert_eq!(canvas.pixel(6, 5), GREEN);
    assert_eq!(canvas.pixel(3, 3), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}
