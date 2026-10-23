// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::{css, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn names(declarations: &str) -> Vec<String> {
    css::parse(&format!("a {{ {} }}", declarations)).rules[0]
        .declarations
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect()
}

fn render(style: &str) -> foundation::paint::Canvas {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ position: relative; width: 60px; height: 60px }} #b {{ position: absolute; background: red; {} }}</style></head><body><div id=a><div id=b></div></div></body></html>",
        style
    ))
    .render(80, 80, 0.0)
    .canvas
}

#[test]
fn inset_expands_like_margin() {
    assert_eq!(names("inset: 1px"), vec!["top", "right", "bottom", "left"]);
    assert_eq!(names("inset: 1px 2px 3px 4px").len(), 4);
    assert!(names("inset: 1px 2px 3px 4px 5px").is_empty());
}

#[test]
fn inset_stretches_an_absolute_box() {
    let canvas = render("inset: 10px");
    assert_eq!(canvas.pixel(9, 30), WHITE);
    assert_eq!(canvas.pixel(10, 30), RED);
    assert_eq!(canvas.pixel(49, 30), RED);
    assert_eq!(canvas.pixel(50, 30), WHITE);
    assert_eq!(canvas.pixel(30, 9), WHITE);
    assert_eq!(canvas.pixel(30, 10), RED);
    assert_eq!(canvas.pixel(30, 49), RED);
    assert_eq!(canvas.pixel(30, 50), WHITE);
}

#[test]
fn inset_values_map_to_their_sides() {
    let canvas = render("inset: 5px 10px 15px 20px");
    assert_eq!(canvas.pixel(19, 30), WHITE);
    assert_eq!(canvas.pixel(20, 30), RED);
    assert_eq!(canvas.pixel(49, 30), RED);
    assert_eq!(canvas.pixel(50, 30), WHITE);
    assert_eq!(canvas.pixel(30, 4), WHITE);
    assert_eq!(canvas.pixel(30, 5), RED);
    assert_eq!(canvas.pixel(30, 44), RED);
    assert_eq!(canvas.pixel(30, 45), WHITE);
}

#[test]
fn inset_accepts_auto_and_percentages() {
    let canvas = render("inset: 0 auto auto 0; width: 10px; height: 10px");
    assert_eq!(canvas.pixel(5, 5), RED);
    assert_eq!(canvas.pixel(15, 5), WHITE);
    let canvas = render("inset: 50% auto auto 50%; width: 10px; height: 10px");
    assert_eq!(canvas.pixel(29, 29), WHITE);
    assert_eq!(canvas.pixel(30, 30), RED);
}

#[test]
fn later_longhands_override_inset() {
    let canvas = render("inset: 10px; left: 30px");
    assert_eq!(canvas.pixel(29, 30), WHITE);
    assert_eq!(canvas.pixel(30, 30), RED);
}

#[test]
fn inset_is_supported() {
    assert!(foundation::supports::evaluate("(inset: 0)"));
}
