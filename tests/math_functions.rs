// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn value(text: &str) -> Option<Value> {
    css::parse(&format!("a {{ width: {} }}", text)).rules[0]
        .declarations
        .first()
        .map(|declaration| declaration.value.clone())
}

fn render(width: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: {}; height: 10px; background: red\"></div></body></html>",
        width
    );
    Page::new(&html).render(200, 20, 0.0).canvas
}

fn filled_width(canvas: &Canvas) -> usize {
    (0..200).filter(|x| canvas.pixel(*x, 5) == RED).count()
}

#[test]
fn absolute_arguments_fold_to_a_length() {
    assert_eq!(value("min(10px, 20px)"), Some(Value::Length(10.0)));
    assert_eq!(value("max(10px, 20px)"), Some(Value::Length(20.0)));
    assert_eq!(value("clamp(10px, 5px, 20px)"), Some(Value::Length(10.0)));
    assert_eq!(value("clamp(10px, 15px, 20px)"), Some(Value::Length(15.0)));
    assert_eq!(value("clamp(10px, 25px, 20px)"), Some(Value::Length(20.0)));
}

#[test]
fn min_picks_the_smaller_resolved_value() {
    assert_eq!(filled_width(&render("min(50%, 150px)")), 100);
    assert_eq!(filled_width(&render("min(90%, 150px)")), 150);
}

#[test]
fn max_picks_the_larger_resolved_value() {
    assert_eq!(filled_width(&render("max(10%, 30px)")), 30);
    assert_eq!(filled_width(&render("max(50%, 30px)")), 100);
}

#[test]
fn clamp_bounds_the_middle_value() {
    assert_eq!(filled_width(&render("clamp(40px, 10%, 120px)")), 40);
    assert_eq!(filled_width(&render("clamp(40px, 40%, 120px)")), 80);
    assert_eq!(filled_width(&render("clamp(40px, 90%, 120px)")), 120);
}

#[test]
fn functions_nest_and_combine_with_calc() {
    assert_eq!(filled_width(&render("calc(min(50%, 150px) + 10px)")), 110);
    assert_eq!(filled_width(&render("min(calc(100% - 20px), 500px)")), 180);
    assert_eq!(filled_width(&render("calc(max(10px, 5px) * 2)")), 20);
    assert_eq!(filled_width(&render("calc(min(40px, 60px) / 2)")), 20);
}

#[test]
fn viewport_units_resolve_inside_functions() {
    assert_eq!(filled_width(&render("min(50vw, 150px)")), 100);
    assert_eq!(filled_width(&render("max(10vw, 5px)")), 20);
}

#[test]
fn em_units_use_the_element_font_size() {
    let canvas = Page::new(
        "<html><head><style>body { margin: 0 }</style></head><body><div style=\"font-size: 10px; width: min(5em, 80px); height: 10px; background: red\"></div></body></html>",
    )
    .render(200, 20, 0.0)
    .canvas;
    assert_eq!(filled_width(&canvas), 50);
}

#[test]
fn applies_to_other_properties() {
    let canvas = Page::new(
        "<html><head><style>body { margin: 0 }</style></head><body><div style=\"width: 150px; max-width: min(40%, 100px); height: clamp(5px, 1px, 8px); background: red\"></div></body></html>",
    )
    .render(200, 20, 0.0)
    .canvas;
    assert_eq!((0..200).filter(|x| canvas.pixel(*x, 2) == RED).count(), 80);
    assert_eq!(canvas.pixel(5, 4), RED);
    assert_eq!(canvas.pixel(5, 5), WHITE);
}

#[test]
fn invalid_expressions_are_dropped() {
    assert_eq!(value("min()"), None);
    assert_eq!(value("min(10px,)"), None);
    assert_eq!(value("clamp(1px, 2px)"), None);
    assert_eq!(value("clamp(1px, 2px, 3px, 4px)"), None);
    assert_eq!(value("min(10px 20px)"), None);
    assert_eq!(value("min(1, 2px)"), None);
    assert_eq!(value("min(10px, 20px) * 2"), None);
}

#[test]
fn supports_accepts_the_functions() {
    assert!(foundation::supports::evaluate("(width: min(10px, 50%))"));
    assert!(foundation::supports::evaluate(
        "(width: clamp(1px, 2%, 3px))"
    ));
}
