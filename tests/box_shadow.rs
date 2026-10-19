// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::paint::Canvas;
use foundation::shadow;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const BLACK: Color = Color { r: 0, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(body: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 30px }} .b {{ width: 40px; height: 40px; background-color: red }}</style></head><body>{}</body></html>",
        body
    );
    Page::new(&html).render(120, 120, 0.0).canvas
}

fn near(actual: Color, expected: (u8, u8, u8)) -> bool {
    let close = |a: u8, b: u8| (a as i32 - b as i32).abs() <= 2;
    close(actual.r, expected.0) && close(actual.g, expected.1) && close(actual.b, expected.2)
}

fn parsed(text: &str) -> Vec<shadow::Shadow> {
    match css::parse_declarations(text)
        .into_iter()
        .next()
        .unwrap()
        .value
    {
        Value::Shadows(shadows) => shadows,
        other => panic!("{:?}", other),
    }
}

#[test]
fn a_hard_shadow_is_offset_behind_the_box() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: 10px 10px blue\"></div>");
    assert_eq!(canvas.pixel(50, 50), RED);
    assert_eq!(canvas.pixel(75, 75), BLUE);
    assert_eq!(canvas.pixel(75, 35), WHITE);
    assert_eq!(canvas.pixel(35, 75), WHITE);
    assert_eq!(canvas.pixel(85, 85), WHITE);
}

#[test]
fn the_shadow_is_not_painted_under_a_transparent_box() {
    let canvas =
        render("<div style=\"width: 40px; height: 40px; box-shadow: 0 0 0 10px blue\"></div>");
    assert_eq!(canvas.pixel(50, 50), WHITE);
    assert_eq!(canvas.pixel(25, 50), BLUE);
}

#[test]
fn spread_grows_the_shadow() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: 0 0 0 5px blue\"></div>");
    assert_eq!(canvas.pixel(27, 50), BLUE);
    assert_eq!(canvas.pixel(24, 50), WHITE);
    assert_eq!(canvas.pixel(50, 73), BLUE);
}

#[test]
fn a_negative_spread_shrinks_it() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: 0 10px 0 -5px blue\"></div>");
    assert_eq!(canvas.pixel(50, 72), BLUE);
    assert_eq!(canvas.pixel(50, 76), WHITE);
    assert_eq!(canvas.pixel(33, 72), WHITE);
}

#[test]
fn blur_fades_the_edge() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: 0 0 20px black\"></div>");
    let near_edge = canvas.pixel(28, 50);
    let far = canvas.pixel(15, 50);
    assert!(near_edge.r < 200 && near_edge.r > 60, "{:?}", near_edge);
    assert!(far.r > near_edge.r);
    assert_eq!(canvas.pixel(2, 50), WHITE);
    assert_eq!(canvas.pixel(50, 50), RED);
}

#[test]
fn blur_is_half_covered_at_the_shape_edge() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: 0 0 20px black\"></div>");
    let edge = canvas.pixel(29, 50);
    assert!(edge.r > 100 && edge.r < 160, "{:?}", edge);
}

#[test]
fn alpha_colors_blend() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: 10px 10px rgba(0,0,0,0.5)\"></div>");
    assert!(
        near(canvas.pixel(75, 75), (128, 128, 128)),
        "{:?}",
        canvas.pixel(75, 75)
    );
}

#[test]
fn the_default_color_is_the_text_color() {
    let canvas = render("<div class=\"b\" style=\"color: blue; box-shadow: 10px 10px\"></div>");
    assert_eq!(canvas.pixel(75, 75), BLUE);
    let canvas =
        render("<div class=\"b\" style=\"color: blue; box-shadow: 10px 10px currentcolor\"></div>");
    assert_eq!(canvas.pixel(75, 75), BLUE);
}

#[test]
fn several_shadows_stack_with_the_first_on_top() {
    let canvas =
        render("<div class=\"b\" style=\"box-shadow: 10px 10px blue, 10px 10px black\"></div>");
    assert_eq!(canvas.pixel(75, 75), BLUE);
    let canvas =
        render("<div class=\"b\" style=\"box-shadow: 10px 10px blue, 20px 20px black\"></div>");
    assert_eq!(canvas.pixel(85, 85), BLACK);
    assert_eq!(canvas.pixel(75, 75), BLUE);
}

#[test]
fn the_shadow_follows_the_border_radius() {
    let canvas = render(
        "<div class=\"b\" style=\"border-radius: 20px; box-shadow: 0 0 0 10px blue\"></div>",
    );
    assert_eq!(canvas.pixel(25, 25), WHITE);
    assert_eq!(canvas.pixel(50, 25), BLUE);
    assert_eq!(canvas.pixel(25, 50), BLUE);
}

#[test]
fn inset_shadows_are_inside_the_padding_box() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: inset 0 0 0 5px blue\"></div>");
    assert_eq!(canvas.pixel(32, 50), BLUE);
    assert_eq!(canvas.pixel(50, 50), RED);
    assert_eq!(canvas.pixel(25, 50), WHITE);
}

#[test]
fn an_inset_offset_shifts_the_hole() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: inset 10px 0 blue\"></div>");
    assert_eq!(canvas.pixel(35, 50), BLUE);
    assert_eq!(canvas.pixel(65, 50), RED);
}

#[test]
fn an_inset_shadow_is_drawn_over_the_background() {
    let canvas = render("<div class=\"b\" style=\"box-shadow: inset 0 0 0 40px blue\"></div>");
    assert_eq!(canvas.pixel(50, 50), BLUE);
}

#[test]
fn none_removes_shadows_set_earlier() {
    let html = "<html><head><style>body { margin: 30px } .b { width: 40px; height: 40px; box-shadow: 10px 10px blue } .b { box-shadow: none }</style></head><body><div class=\"b\"></div></body></html>";
    let canvas = Page::new(html).render(120, 120, 0.0).canvas;
    assert_eq!(canvas.pixel(75, 75), WHITE);
}

#[test]
fn the_shadow_scrolls_with_the_page() {
    let html = "<html><head><style>body { margin: 0 }</style></head><body><div style=\"height: 60px\"></div><div style=\"width: 20px; height: 20px; box-shadow: 10px 10px blue\"></div><div style=\"height: 500px\"></div></body></html>";
    let canvas = Page::new(html).render(120, 120, 30.0).canvas;
    assert_eq!(canvas.pixel(25, 45), BLUE);
    assert_eq!(canvas.pixel(25, 75), WHITE);
}

#[test]
fn the_shadow_is_clipped_by_an_overflow_ancestor() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 50px; height: 50px\"><div class=\"b\" style=\"box-shadow: 30px 0 blue\"></div></div>",
    );
    assert_eq!(canvas.pixel(75, 50), BLUE);
    assert_eq!(canvas.pixel(85, 50), WHITE);
}

#[test]
fn shadows_work_inside_opacity_and_transform() {
    let canvas = render("<div class=\"b\" style=\"opacity: 1; box-shadow: 10px 10px blue\"></div>");
    assert_eq!(canvas.pixel(75, 75), BLUE);
    let canvas = render(
        "<div class=\"b\" style=\"transform: translate(10px, 0); box-shadow: 10px 10px blue\"></div>",
    );
    assert_eq!(canvas.pixel(85, 75), BLUE);
}

#[test]
fn parsing_reads_offsets_blur_spread_and_color() {
    let shadows = parsed("box-shadow: 1px 2px 3px 4px red");
    assert_eq!(shadows.len(), 1);
    let one = &shadows[0];
    assert_eq!((one.offset_x.px, one.offset_y.px), (1.0, 2.0));
    assert_eq!((one.blur.px, one.spread.px), (3.0, 4.0));
    assert_eq!(one.color.unwrap().color, RED);
    assert!(!one.inset);
}

#[test]
fn parsing_accepts_any_order_inset_em_and_lists() {
    let shadows = parsed("box-shadow: red inset 0 0 .5em, 1px 1px black");
    assert_eq!(shadows.len(), 2);
    assert!(shadows[0].inset);
    assert_eq!(shadows[0].blur.em, 0.5);
    assert!(!shadows[1].inset);
}

#[test]
fn parsing_rejects_invalid_values() {
    assert!(css::parse_declarations("box-shadow: 1px").is_empty());
    assert!(css::parse_declarations("box-shadow: 1px 2px -3px red").is_empty());
    assert!(css::parse_declarations("box-shadow: 1px 2px 3px 4px 5px").is_empty());
    assert!(css::parse_declarations("box-shadow: 1px 2px red blue").is_empty());
    assert!(css::parse_declarations("box-shadow: 1px 2px bogus").is_empty());
    assert!(css::parse_declarations("box-shadow: 1px 2px 1rem").is_empty());
}

#[test]
fn blurred_coverage_is_monotonic() {
    let mut last = 1.0;
    for step in -20..=20 {
        let value = shadow::blurred_coverage(step as f32, 10.0);
        assert!(value <= last + 0.0001);
        last = value;
    }
    assert!((shadow::blurred_coverage(0.0, 10.0) - 0.5).abs() < 0.001);
    assert_eq!(shadow::blurred_coverage(-5.0, 0.0), 1.0);
    assert_eq!(shadow::blurred_coverage(5.0, 0.0), 0.0);
}

#[test]
fn supports_recognizes_box_shadow() {
    assert!(foundation::supports::evaluate("(box-shadow: 0 0 4px red)"));
}
