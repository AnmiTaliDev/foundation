// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn declarations(text: &str) -> Vec<(String, Value)> {
    css::parse(&format!("a {{ {} }}", text)).rules[0]
        .declarations
        .iter()
        .map(|declaration| (declaration.name.clone(), declaration.value.clone()))
        .collect()
}

fn names(text: &str) -> Vec<String> {
    declarations(text)
        .into_iter()
        .map(|(name, _)| name)
        .collect()
}

fn render(style: &str) -> foundation::paint::Canvas {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ position: relative; width: 80px; height: 80px }} #b {{ background: red; {} }}</style></head><body><div id=a><div id=b></div></div></body></html>",
        style
    ))
    .render(100, 100, 0.0)
    .canvas
}

#[test]
fn single_properties_map_to_physical_sides() {
    let cases = [
        ("margin-inline-start", "margin-left"),
        ("margin-inline-end", "margin-right"),
        ("margin-block-start", "margin-top"),
        ("margin-block-end", "margin-bottom"),
        ("padding-inline-start", "padding-left"),
        ("padding-inline-end", "padding-right"),
        ("padding-block-start", "padding-top"),
        ("padding-block-end", "padding-bottom"),
        ("inset-inline-start", "left"),
        ("inset-inline-end", "right"),
        ("inset-block-start", "top"),
        ("inset-block-end", "bottom"),
        ("inline-size", "width"),
        ("block-size", "height"),
        ("min-inline-size", "min-width"),
        ("max-block-size", "max-height"),
        ("overflow-inline", "overflow-x"),
        ("border-inline-start-width", "border-left-width"),
        ("border-block-end-style", "border-bottom-style"),
        ("border-inline-end-color", "border-right-color"),
        ("scroll-margin-block-start", "scroll-margin-top"),
        ("scroll-padding-inline-end", "scroll-padding-right"),
    ];
    for (logical, physical) in cases {
        let value = if logical.ends_with("-style") {
            "solid"
        } else if logical.ends_with("-color") {
            "red"
        } else if logical.starts_with("overflow") {
            "hidden"
        } else {
            "5px"
        };
        assert_eq!(
            names(&format!("{}: {}", logical, value)),
            vec![physical.to_string()],
            "{}",
            logical
        );
    }
}

#[test]
fn values_are_kept() {
    let found = declarations("margin-inline-start: 7px");
    assert_eq!(found, vec![("margin-left".to_string(), Value::Length(7.0))]);
}

#[test]
fn pairs_take_one_or_two_values() {
    let one = declarations("margin-inline: 4px");
    assert_eq!(
        one,
        vec![
            ("margin-left".to_string(), Value::Length(4.0)),
            ("margin-right".to_string(), Value::Length(4.0))
        ]
    );
    let two = declarations("padding-block: 1px 2px");
    assert_eq!(
        two,
        vec![
            ("padding-top".to_string(), Value::Length(1.0)),
            ("padding-bottom".to_string(), Value::Length(2.0))
        ]
    );
    assert_eq!(names("inset-inline: 0 5px"), vec!["left", "right"]);
    assert_eq!(names("inset-block: 0"), vec!["top", "bottom"]);
}

#[test]
fn border_pairs_map_every_part() {
    assert_eq!(
        names("border-inline-width: 1px 2px"),
        vec!["border-left-width", "border-right-width"]
    );
    assert_eq!(
        names("border-block-style: solid dashed"),
        vec!["border-top-style", "border-bottom-style"]
    );
    assert_eq!(
        names("border-inline-color: red blue"),
        vec!["border-left-color", "border-right-color"]
    );
}

#[test]
fn border_shorthands_apply_to_the_physical_sides() {
    let start = names("border-inline-start: 2px solid red");
    assert!(
        start.iter().all(|name| name.contains("left")),
        "{:?}",
        start
    );
    assert_eq!(start.len(), 3);
    let both = names("border-block: 2px solid red");
    assert_eq!(both.iter().filter(|n| n.contains("top")).count(), 3);
    assert_eq!(both.iter().filter(|n| n.contains("bottom")).count(), 3);
    let inline = names("border-inline: 1px dotted blue");
    assert_eq!(inline.iter().filter(|n| n.contains("left")).count(), 3);
    assert_eq!(inline.iter().filter(|n| n.contains("right")).count(), 3);
}

#[test]
fn radii_map_to_corners() {
    assert_eq!(
        names("border-start-start-radius: 4px"),
        vec!["border-top-left-radius-x", "border-top-left-radius-y"]
    );
    assert_eq!(
        names("border-end-end-radius: 4px"),
        vec![
            "border-bottom-right-radius-x",
            "border-bottom-right-radius-y"
        ]
    );
}

#[test]
fn invalid_input_is_dropped() {
    assert!(names("margin-inline: 1px 2px 3px").is_empty());
    assert!(names("margin-inline: ???").is_empty());
    assert!(names("margin-inline: 1px ???").is_empty());
    assert!(names("padding-inline-start: ???").is_empty());
    assert!(names("border-inline-style: solid nonsense").is_empty());
}

#[test]
fn margins_move_the_box() {
    let canvas =
        render("margin-inline-start: 10px; margin-block-start: 20px; width: 20px; height: 20px");
    assert_eq!(canvas.pixel(9, 30), WHITE);
    assert_eq!(canvas.pixel(10, 30), RED);
    assert_eq!(canvas.pixel(30, 30), WHITE);
    assert_eq!(canvas.pixel(15, 19), WHITE);
    assert_eq!(canvas.pixel(15, 20), RED);
}

#[test]
fn sizes_use_the_physical_axes() {
    let canvas = render("inline-size: 30px; block-size: 10px");
    assert_eq!(canvas.pixel(29, 5), RED);
    assert_eq!(canvas.pixel(30, 5), WHITE);
    assert_eq!(canvas.pixel(5, 9), RED);
    assert_eq!(canvas.pixel(5, 10), WHITE);
}

#[test]
fn padding_and_border_are_drawn_on_the_right_sides() {
    let canvas = render(
        "width: 20px; height: 20px; border-inline-start: 4px solid blue; padding-inline-start: 6px",
    );
    assert_eq!(canvas.pixel(2, 10), BLUE);
    assert_eq!(canvas.pixel(5, 10), RED);
    assert_eq!(canvas.pixel(29, 10), RED);
    assert_eq!(canvas.pixel(30, 10), WHITE);
}

#[test]
fn inset_properties_position_boxes() {
    let canvas = Page::new(
        "<html><head><style>body { margin: 0 } #a { position: relative; width: 80px; height: 80px } #b { position: absolute; background: red; width: 10px; height: 10px; inset-inline-start: 20px; inset-block-start: 30px }</style></head><body><div id=a><div id=b></div></div></body></html>",
    )
    .render(100, 100, 0.0)
    .canvas;
    assert_eq!(canvas.pixel(19, 35), WHITE);
    assert_eq!(canvas.pixel(20, 35), RED);
    assert_eq!(canvas.pixel(25, 29), WHITE);
    assert_eq!(canvas.pixel(25, 30), RED);
}

#[test]
fn logical_longhands_override_earlier_physical_ones() {
    let canvas = render("margin-left: 30px; margin-inline-start: 10px; width: 20px; height: 20px");
    assert_eq!(canvas.pixel(9, 5), WHITE);
    assert_eq!(canvas.pixel(10, 5), RED);
}

#[test]
fn important_applies_to_logical_properties() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } #b { margin-inline-start: 10px !important; margin-left: 30px; width: 20px; height: 20px; background: red }</style></head><body><div id=b></div></body></html>",
    );
    let canvas = page.render(100, 40, 0.0).canvas;
    assert_eq!(canvas.pixel(9, 5), WHITE);
    assert_eq!(canvas.pixel(10, 5), RED);
}

#[test]
fn logical_properties_are_supported() {
    assert!(foundation::supports::evaluate("(margin-inline: 4px)"));
    assert!(foundation::supports::evaluate("(inline-size: 10px)"));
    assert!(foundation::supports::evaluate(
        "(border-inline-start: 1px solid red)"
    ));
}
