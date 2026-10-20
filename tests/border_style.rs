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

fn render(style: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"margin: 10px; width: 60px; height: 60px; {}\"></div></body></html>",
        style
    );
    Page::new(&html).render(100, 100, 0.0).canvas
}

fn row(canvas: &Canvas, y: usize, from: usize, to: usize) -> Vec<bool> {
    (from..to).map(|x| canvas.pixel(x, y) == RED).collect()
}

#[test]
fn unset_style_keeps_the_border_solid() {
    let canvas = render("border-width: 3px; border-color: red");
    assert_eq!(canvas.pixel(11, 40), RED);
    assert_eq!(canvas.pixel(40, 11), RED);
}

#[test]
fn none_removes_the_border_and_its_width() {
    let canvas = render("border: 3px solid red; border-style: none");
    assert_eq!(canvas.pixel(11, 40), WHITE);
    assert_eq!(canvas.pixel(10, 10), WHITE);
    let shifted = render("border: 3px solid red; border-style: none; background-color: red");
    assert_eq!(shifted.pixel(10, 40), RED);
    assert_eq!(shifted.pixel(9, 40), WHITE);
}

#[test]
fn hidden_behaves_like_none() {
    let canvas = render("border: 3px solid red; border-style: hidden");
    assert_eq!(canvas.pixel(11, 40), WHITE);
}

#[test]
fn style_applies_per_side() {
    let canvas = render("border: 3px solid red; border-style: none solid");
    assert_eq!(canvas.pixel(40, 11), WHITE);
    assert_eq!(canvas.pixel(11, 40), RED);
    assert_eq!(canvas.pixel(40, 11 + 62), WHITE);
}

#[test]
fn longhand_sets_one_side() {
    let canvas = render("border: 3px solid red; border-left-style: none");
    assert_eq!(canvas.pixel(11, 40), WHITE);
    assert_eq!(canvas.pixel(40, 11), RED);
}

#[test]
fn dashed_alternates_dashes_and_gaps() {
    let canvas = render("border: 2px dashed red");
    let top = row(&canvas, 10, 10, 40);
    assert!(top[0] && top[5]);
    assert!(!top[6] && !top[11]);
    assert!(top[12]);
    assert_eq!(canvas.pixel(10, 10 + 6 + 3), WHITE);
}

#[test]
fn dotted_alternates_single_width_dots() {
    let canvas = render("border: 2px dotted red");
    let top = row(&canvas, 10, 10, 30);
    assert!(top[0] && top[1]);
    assert!(!top[2] && !top[3]);
    assert!(top[4]);
}

#[test]
fn double_leaves_a_gap_between_two_lines() {
    let canvas = render("border: 9px double red");
    assert_eq!(canvas.pixel(40, 10), RED);
    assert_eq!(canvas.pixel(40, 12), RED);
    assert_eq!(canvas.pixel(40, 13), WHITE);
    assert_eq!(canvas.pixel(40, 15), WHITE);
    assert_eq!(canvas.pixel(40, 16), RED);
    assert_eq!(canvas.pixel(40, 18), RED);
}

#[test]
fn thin_double_falls_back_to_solid() {
    let canvas = render("border: 2px double red");
    assert_eq!(canvas.pixel(40, 10), RED);
    assert_eq!(canvas.pixel(40, 11), RED);
}

#[test]
fn groove_and_inset_draw_solid() {
    let canvas = render("border: 3px groove red");
    assert_eq!(canvas.pixel(40, 11), RED);
    let canvas = render("border: 3px inset red");
    assert_eq!(canvas.pixel(40, 11), RED);
}

#[test]
fn shorthand_none_clears_style_and_width() {
    let sheet = css::parse("p { border: none } q { border: hidden }");
    for rule in &sheet.rules {
        assert!(rule
            .declarations
            .iter()
            .any(|d| d.name == "border-top-style" && matches!(&d.value, Value::Keyword(_))));
    }
}

#[test]
fn invalid_style_is_dropped() {
    let sheet =
        css::parse("p { border-style: wide; border-top-style: 5px; border-style: solid dotted }");
    let names: Vec<&str> = sheet.rules[0]
        .declarations
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(names.len(), 4);
}

#[test]
fn outline_honors_dashed_and_dotted() {
    let canvas = render("outline: 2px dashed red");
    let top = row(&canvas, 8, 8, 40);
    assert!(top[0] && top[5]);
    assert!(!top[6] && !top[11]);
    assert!(top[12]);
}

#[test]
fn supports_the_new_properties() {
    assert!(foundation::supports::evaluate("(border-style: solid)"));
    assert!(foundation::supports::evaluate("(border-top-style: dashed)"));
}
