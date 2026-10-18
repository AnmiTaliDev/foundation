// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::paint::Canvas;
use foundation::transform::{self, Amount, Function};
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(body: &str, scroll: f32) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }} .b {{ width: 10px; height: 10px; background-color: red }}</style></head><body>{}</body></html>",
        body
    );
    Page::new(&html).render(60, 40, scroll).canvas
}

fn near(actual: Color, expected: (u8, u8, u8)) -> bool {
    let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 1;
    close(actual.r, expected.0) && close(actual.g, expected.1) && close(actual.b, expected.2)
}

#[test]
fn opacity_blends_with_the_backdrop() {
    let canvas = render("<div class=\"b\" style=\"opacity: 0.5\"></div>", 0.0);
    assert!(
        near(canvas.pixel(5, 5), (255, 128, 128)),
        "{:?}",
        canvas.pixel(5, 5)
    );
    assert_eq!(canvas.pixel(20, 5), WHITE);
    let canvas = render("<div class=\"b\" style=\"opacity: 0\"></div>", 0.0);
    assert_eq!(canvas.pixel(5, 5), WHITE);
    let canvas = render("<div class=\"b\" style=\"opacity: 1\"></div>", 0.0);
    assert_eq!(canvas.pixel(5, 5), RED);
    let canvas = render("<div class=\"b\" style=\"opacity: 50%\"></div>", 0.0);
    assert!(near(canvas.pixel(5, 5), (255, 128, 128)));
}

#[test]
fn opacity_applies_to_the_group_as_a_whole() {
    let canvas = render(
        "<div style=\"opacity: 0.5; width: 20px; height: 20px; background-color: red\"><div style=\"width: 10px; height: 10px; background-color: blue\"></div></div>",
        0.0,
    );
    assert!(
        near(canvas.pixel(5, 5), (128, 128, 255)),
        "{:?}",
        canvas.pixel(5, 5)
    );
    assert!(near(canvas.pixel(15, 15), (255, 128, 128)));
}

#[test]
fn nested_opacity_multiplies() {
    let canvas = render(
        "<div style=\"opacity: 0.5\"><div class=\"b\" style=\"opacity: 0.5\"></div></div>",
        0.0,
    );
    assert!(
        near(canvas.pixel(5, 5), (255, 191, 191)),
        "{:?}",
        canvas.pixel(5, 5)
    );
}

#[test]
fn translate_moves_the_painted_box_only() {
    let canvas = render(
        "<div class=\"b\" style=\"transform: translate(10px, 5px)\"></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(15, 10), RED);
    assert_eq!(canvas.pixel(2, 2), WHITE);
    assert_eq!(canvas.pixel(25, 10), WHITE);
}

#[test]
fn translate_percentages_use_the_border_box() {
    let canvas = render(
        "<div class=\"b\" style=\"transform: translateX(100%)\"></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(15, 5), RED);
    assert_eq!(canvas.pixel(5, 5), WHITE);
}

#[test]
fn scale_grows_around_the_center_by_default() {
    let canvas = render(
        "<div style=\"margin: 10px 0 0 10px\"><div class=\"b\" style=\"transform: scale(2)\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(6, 6), RED);
    assert_eq!(canvas.pixel(23, 23), RED);
    assert_eq!(canvas.pixel(3, 3), WHITE);
    assert_eq!(canvas.pixel(27, 27), WHITE);
}

#[test]
fn transform_origin_moves_the_pivot() {
    let canvas = render(
        "<div style=\"margin: 10px 0 0 10px\"><div class=\"b\" style=\"transform: scale(2); transform-origin: 0 0\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(28, 28), RED);
    assert_eq!(canvas.pixel(8, 8), WHITE);
    let canvas = render(
        "<div style=\"margin: 10px 0 0 10px\"><div class=\"b\" style=\"transform: scale(2); transform-origin: right bottom\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(1, 1), RED);
    assert_eq!(canvas.pixel(18, 18), RED);
    assert_eq!(canvas.pixel(22, 22), WHITE);
}

#[test]
fn rotate_turns_the_box() {
    let canvas = render(
        "<div style=\"margin: 20px 0 0 20px\"><div style=\"width: 20px; height: 10px; background-color: red; transform: rotate(90deg)\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(30, 32), RED);
    assert_eq!(canvas.pixel(30, 17), RED);
    assert_eq!(canvas.pixel(22, 25), WHITE);
    assert_eq!(canvas.pixel(38, 25), WHITE);
}

#[test]
fn transform_respects_scrolling() {
    let canvas = render(
        "<div style=\"height: 10px\"></div><div class=\"b\" style=\"transform: translateY(10px)\"></div><div style=\"height: 100px\"></div>",
        5.0,
    );
    assert_eq!(canvas.pixel(5, 16), RED);
    assert_eq!(canvas.pixel(5, 12), WHITE);
    assert_eq!(canvas.pixel(5, 26), WHITE);
}

#[test]
fn transform_and_opacity_combine_and_nest() {
    let canvas = render(
        "<div style=\"transform: translateX(20px); opacity: 0.5\"><div class=\"b\"></div></div>",
        0.0,
    );
    assert!(
        near(canvas.pixel(25, 5), (255, 128, 128)),
        "{:?}",
        canvas.pixel(25, 5)
    );
    assert_eq!(canvas.pixel(5, 5), WHITE);
    let canvas = render(
        "<div style=\"transform: translateX(10px)\"><div class=\"b\" style=\"transform: translateX(10px)\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(25, 5), RED);
    assert_eq!(canvas.pixel(5, 5), WHITE);
}

#[test]
fn transformed_text_is_drawn() {
    let canvas = render(
        "<div style=\"transform: translateY(20px); height: 10px\">Hello</div>",
        0.0,
    );
    let inked = |top: usize, bottom: usize| {
        (top..bottom).any(|y| (0..40).any(|x| canvas.pixel(x, y) != WHITE))
    };
    assert!(inked(20, 32));
    assert!(!inked(0, 18));
}

#[test]
fn transform_values_are_parsed() {
    let declarations = css::parse_declarations(
        "transform: translate(10px, 50%) rotate(45deg) scale(2) skewX(10deg) matrix(1,0,0,1,5,6)",
    );
    let Value::Transform(functions) = &declarations[0].value else {
        panic!("{:?}", declarations);
    };
    assert_eq!(functions.len(), 5);
    assert_eq!(
        functions[0],
        Function::Translate(Amount::Px(10.0), Amount::Percent(50.0))
    );
    assert_eq!(functions[2], Function::Scale(2.0, 2.0));
    assert_eq!(
        functions[4],
        Function::Matrix([1.0, 0.0, 0.0, 1.0, 5.0, 6.0])
    );
    assert!(css::parse_declarations("transform: rotate(foo)").is_empty());
    assert!(css::parse_declarations("transform: translate3d(1px,2px,3px)").is_empty());
    assert_eq!(
        css::parse_declarations("transform: none")[0].value,
        Value::Keyword("none".to_string())
    );
    let origin = css::parse_declarations("transform-origin: left top");
    assert_eq!(origin[0].name, "transform-origin-x");
    assert_eq!(origin[0].value, Value::Percent(0.0));
}

#[test]
fn matrices_compose_and_invert() {
    let matrix = transform::compose(
        &[
            Function::Translate(Amount::Px(10.0), Amount::Px(0.0)),
            Function::Scale(2.0, 2.0),
        ],
        (0.0, 0.0),
        0.0,
        0.0,
        16.0,
    );
    assert_eq!(transform::apply(&matrix, 1.0, 1.0), (12.0, 2.0));
    let inverse = transform::invert(&matrix).unwrap();
    let (x, y) = transform::apply(&inverse, 12.0, 2.0);
    assert!((x - 1.0).abs() < 1e-5 && (y - 1.0).abs() < 1e-5);
    assert!(transform::invert(&[0.0, 0.0, 0.0, 0.0, 1.0, 1.0]).is_none());
}
