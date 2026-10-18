// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::gradient::{Direction, Kind};
use foundation::paint::Canvas;
use foundation::Page;

fn value_of(declaration: &str) -> Option<Value> {
    css::parse_declarations(declaration)
        .into_iter()
        .next()
        .map(|d| d.value)
}

fn color(r: u8, g: u8, b: u8) -> Value {
    Value::Color(Color { r, g, b })
}

#[test]
fn color_functions_are_parsed() {
    assert_eq!(value_of("color: rgb(255, 0, 0)"), Some(color(255, 0, 0)));
    assert_eq!(value_of("color: rgb(100%, 0%, 0%)"), Some(color(255, 0, 0)));
    assert_eq!(value_of("color: rgb(300, 0, 0)"), Some(color(255, 0, 0)));
    assert_eq!(value_of("color: RGB(0 128 255)"), Some(color(0, 128, 255)));
    assert_eq!(
        value_of("color: rgb(255 0 0 / 50%)"),
        Some(Value::Rgba(Color { r: 255, g: 0, b: 0 }, 128))
    );
    assert_eq!(
        value_of("color: rgba(0, 0, 0, 0.5)"),
        Some(Value::Rgba(Color { r: 0, g: 0, b: 0 }, 128))
    );
    assert_eq!(value_of("color: rgba(1, 2, 3, 1)"), Some(color(1, 2, 3)));
    assert_eq!(
        value_of("color: hsl(120, 100%, 50%)"),
        Some(color(0, 255, 0))
    );
    assert_eq!(value_of("color: hsl(0 100% 50%)"), Some(color(255, 0, 0)));
    assert_eq!(
        value_of("color: hsl(0.5turn, 100%, 50%)"),
        Some(color(0, 255, 255))
    );
    assert_eq!(
        value_of("color: hsl(0, 0%, 50%)"),
        Some(color(128, 128, 128))
    );
    assert_eq!(
        value_of("color: hsla(240, 100%, 50%, 0.25)"),
        Some(Value::Rgba(Color { r: 0, g: 0, b: 255 }, 64))
    );
    assert_eq!(
        value_of("color: #f008"),
        Some(Value::Rgba(Color { r: 255, g: 0, b: 0 }, 136))
    );
    assert_eq!(
        value_of("color: #ff000080"),
        Some(Value::Rgba(Color { r: 255, g: 0, b: 0 }, 128))
    );
    assert_eq!(
        value_of("color: transparent"),
        Some(Value::Rgba(Color { r: 0, g: 0, b: 0 }, 0))
    );
    assert_eq!(value_of("color: rgb(1, 2)"), None);
    assert_eq!(value_of("color: rgb(a, b, c)"), None);
    assert_eq!(value_of("color: #12345"), None);
}

const BOX: &str = "body { margin: 0 } div { width: 100px; height: 20px }";

fn render(rule: &str) -> Canvas {
    let html = format!(
        "<html><head><style>{} {}</style></head><body><div></div></body></html>",
        BOX, rule
    );
    Page::new(&html).render(120, 120, 0.0).canvas
}

fn near(actual: Color, expected: (u8, u8, u8), tolerance: i32) -> bool {
    let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= tolerance;
    close(actual.r, expected.0) && close(actual.g, expected.1) && close(actual.b, expected.2)
}

fn check(canvas: &Canvas, x: usize, y: usize, expected: (u8, u8, u8), tolerance: i32) {
    let actual = canvas.pixel(x, y);
    assert!(
        near(actual, expected, tolerance),
        "({}, {}) is {:?}, expected {:?}",
        x,
        y,
        actual,
        expected
    );
}

#[test]
fn translucent_colors_blend_with_the_backdrop() {
    let canvas = render("div { background-color: rgba(255, 0, 0, 0.5) }");
    check(&canvas, 5, 5, (255, 128, 128), 1);
    let canvas = render("div { background-color: transparent }");
    check(&canvas, 5, 5, (255, 255, 255), 0);
    let canvas =
        render("div { border: 4px solid rgba(0, 0, 255, 0.5); width: 92px; height: 12px }");
    check(&canvas, 1, 1, (128, 128, 255), 1);
    check(&canvas, 50, 10, (255, 255, 255), 0);
    let canvas = render("body { background-color: rgba(0, 0, 0, 0.5) }");
    check(&canvas, 110, 30, (128, 128, 128), 1);
}

#[test]
fn translucent_text_is_lighter() {
    let page = |color: &str| {
        let html = format!(
            "<html><body style=\"margin:0;font-size:20px;color:{}\">HHHH</body></html>",
            color
        );
        Page::new(&html).render(120, 120, 0.0).canvas
    };
    let solid = page("black");
    let faded = page("rgba(0, 0, 0, 0.5)");
    let darkest = |canvas: &Canvas| {
        (0..30)
            .flat_map(|y| (0..100).map(move |x| (x, y)))
            .map(|(x, y)| canvas.pixel(x, y).r)
            .min()
            .unwrap()
    };
    assert_eq!(darkest(&solid), 0);
    assert!(
        darkest(&faded) >= 120 && darkest(&faded) <= 135,
        "{}",
        darkest(&faded)
    );
}

#[test]
fn linear_gradient_runs_along_the_direction() {
    let canvas = render("div { background: linear-gradient(to right, red, blue) }");
    check(&canvas, 0, 5, (255, 0, 0), 4);
    check(&canvas, 50, 5, (128, 0, 128), 4);
    check(&canvas, 99, 5, (0, 0, 255), 4);
    check(&canvas, 50, 0, (128, 0, 128), 4);

    let canvas = render("div { background: linear-gradient(red, blue) }");
    check(&canvas, 5, 0, (249, 0, 6), 3);
    check(&canvas, 5, 19, (6, 0, 249), 3);
    check(&canvas, 5, 10, (128, 0, 128), 8);

    let canvas = render("div { background: linear-gradient(0deg, red, blue) }");
    check(&canvas, 5, 0, (6, 0, 249), 3);
    check(&canvas, 5, 19, (249, 0, 6), 3);

    let canvas = render("div { background: linear-gradient(90deg, red, blue) }");
    check(&canvas, 0, 5, (255, 0, 0), 4);
    check(&canvas, 99, 5, (0, 0, 255), 4);

    let canvas = render("div { background: linear-gradient(0.25turn, red, blue) }");
    check(&canvas, 0, 5, (255, 0, 0), 4);
}

#[test]
fn corner_directions_cross_the_other_corners_at_the_middle() {
    let canvas = render(
        "div { width: 40px; height: 40px; background: linear-gradient(to bottom right, red, blue) }",
    );
    check(&canvas, 0, 0, (255, 0, 0), 12);
    check(&canvas, 39, 39, (0, 0, 255), 12);
    check(&canvas, 39, 0, (128, 0, 128), 12);
    check(&canvas, 0, 39, (128, 0, 128), 12);
}

#[test]
fn color_stops_can_be_positioned() {
    let canvas = render("div { background: linear-gradient(to right, red 50%, blue 50%) }");
    check(&canvas, 25, 5, (255, 0, 0), 0);
    check(&canvas, 75, 5, (0, 0, 255), 0);

    let canvas = render("div { background: linear-gradient(to right, red 25%, blue 75%) }");
    check(&canvas, 10, 5, (255, 0, 0), 0);
    check(&canvas, 90, 5, (0, 0, 255), 0);
    check(&canvas, 50, 5, (128, 0, 128), 4);

    let canvas = render("div { background: linear-gradient(to right, red, yellow, blue) }");
    check(&canvas, 50, 5, (255, 255, 0), 8);

    let canvas = render("div { background: linear-gradient(to right, red 20px, blue 60px) }");
    check(&canvas, 10, 5, (255, 0, 0), 0);
    check(&canvas, 70, 5, (0, 0, 255), 0);

    let canvas = render("div { background: linear-gradient(to right, red 0 50%, blue 50% 100%) }");
    check(&canvas, 25, 5, (255, 0, 0), 0);
    check(&canvas, 75, 5, (0, 0, 255), 0);
}

#[test]
fn transparent_stops_interpolate_with_premultiplied_alpha() {
    let canvas = render("div { background: linear-gradient(to right, red, transparent) }");
    check(&canvas, 50, 5, (255, 128, 128), 4);
    check(&canvas, 99, 5, (255, 255, 255), 6);

    let canvas = render("div { background: linear-gradient(rgba(0,0,0,0), rgba(0,0,0,1)) }");
    check(&canvas, 5, 0, (242, 242, 242), 8);
    check(&canvas, 5, 10, (128, 128, 128), 8);
    check(&canvas, 5, 19, (13, 13, 13), 8);
}

#[test]
fn repeating_gradients_tile_their_stop_range() {
    let canvas = render(
        "div { background: repeating-linear-gradient(to right, red 0, red 10px, blue 10px, blue 20px) }",
    );
    check(&canvas, 5, 5, (255, 0, 0), 0);
    check(&canvas, 15, 5, (0, 0, 255), 0);
    check(&canvas, 25, 5, (255, 0, 0), 0);
    check(&canvas, 35, 5, (0, 0, 255), 0);
}

#[test]
fn radial_gradients_grow_from_the_center() {
    let canvas = render(
        "div { width: 100px; height: 100px; background: radial-gradient(circle, red, blue) }",
    );
    check(&canvas, 50, 50, (255, 0, 0), 12);
    check(&canvas, 0, 0, (0, 0, 255), 12);
    check(&canvas, 99, 99, (0, 0, 255), 12);

    let canvas = render(
        "div { width: 100px; height: 100px; background: radial-gradient(circle closest-side at 50% 50%, red, blue) }",
    );
    check(&canvas, 50, 50, (255, 0, 0), 12);
    check(&canvas, 99, 50, (0, 0, 255), 12);
    check(&canvas, 0, 0, (0, 0, 255), 0);

    let canvas = render(
        "div { width: 100px; height: 100px; background: radial-gradient(circle at left top, red, blue) }",
    );
    check(&canvas, 0, 0, (255, 0, 0), 12);
    check(&canvas, 99, 99, (0, 0, 255), 12);

    let canvas = render(
        "div { width: 100px; height: 100px; background: radial-gradient(ellipse farthest-side, red, blue) }",
    );
    check(&canvas, 50, 50, (255, 0, 0), 12);
    check(&canvas, 99, 50, (0, 0, 255), 12);
}

#[test]
fn gradients_tile_with_background_size_and_repeat() {
    let canvas = render(
        "div { background: linear-gradient(to right, red, blue); background-size: 50px 20px }",
    );
    check(&canvas, 0, 5, (255, 0, 0), 6);
    check(&canvas, 49, 5, (0, 0, 255), 6);
    check(&canvas, 50, 5, (255, 0, 0), 6);

    let canvas = render(
        "div { background: linear-gradient(to right, red, blue) no-repeat; background-size: 50px 20px }",
    );
    check(&canvas, 49, 5, (0, 0, 255), 6);
    check(&canvas, 60, 5, (255, 255, 255), 0);
}

#[test]
fn gradient_sits_above_the_background_color_and_below_text() {
    let canvas =
        render("div { background: yellow linear-gradient(to right, rgba(255,0,0,0), red) }");
    check(&canvas, 0, 5, (255, 255, 0), 6);
    check(&canvas, 99, 5, (255, 0, 0), 6);
}

#[test]
fn only_the_first_background_layer_is_used() {
    let canvas = render(
        "div { background: linear-gradient(to right, red, red), linear-gradient(to right, blue, blue) }",
    );
    check(&canvas, 50, 5, (255, 0, 0), 0);
    let canvas =
        render("div { background-image: linear-gradient(to right, red, red), url(x.png) }");
    check(&canvas, 50, 5, (255, 0, 0), 0);
}

#[test]
fn the_final_layer_supplies_the_background_color() {
    let canvas = render(
        "div { background: linear-gradient(to right, rgba(0,0,0,0), rgba(0,0,0,0)), yellow }",
    );
    check(&canvas, 50, 5, (255, 255, 0), 0);
}

#[test]
fn gradients_are_parsed_into_structures() {
    let Some(Value::Gradient(gradient)) =
        value_of("background-image: linear-gradient(to top left, rgba(0, 0, 0, .5) 10%, #fff)")
    else {
        panic!("not a gradient");
    };
    assert_eq!(gradient.kind, Kind::Linear(Direction::Corner(-1, -1)));
    assert_eq!(gradient.stops.len(), 2);
    assert_eq!(gradient.stops[0].alpha, 128);
    assert!(!gradient.repeating);
    assert!(css::parse_declarations("background-image: linear-gradient(red)").is_empty());
    assert!(
        css::parse_declarations("background-image: linear-gradient(to nowhere, red, blue)")
            .is_empty()
    );
    assert!(
        css::parse_declarations("background-image: linear-gradient(red, 50%, blue)").is_empty()
    );
    assert!(css::parse_declarations(
        "background-image: radial-gradient(circle 5px 6px, red, blue)"
    )
    .is_empty());
}
