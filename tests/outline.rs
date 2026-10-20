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

fn render(body: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    );
    Page::new(&html).render(120, 120, 0.0).canvas
}

fn boxed(style: &str) -> Canvas {
    render(&format!(
        "<div style=\"margin: 30px; width: 40px; height: 40px; background-color: lime; {}\"></div>",
        style
    ))
}

#[test]
fn outline_surrounds_the_border_box() {
    let canvas = boxed("outline: 4px solid red");
    assert_eq!(canvas.pixel(28, 50), RED);
    assert_eq!(canvas.pixel(71, 50), RED);
    assert_eq!(canvas.pixel(50, 28), RED);
    assert_eq!(canvas.pixel(50, 71), RED);
    assert_eq!(canvas.pixel(26, 50), RED);
    assert_eq!(canvas.pixel(25, 50), WHITE);
    assert_eq!(canvas.pixel(74, 50), WHITE);
    assert_eq!(canvas.pixel(50, 50), LIME);
}

#[test]
fn outline_covers_the_corners() {
    let canvas = boxed("outline: 4px solid red");
    assert_eq!(canvas.pixel(26, 26), RED);
    assert_eq!(canvas.pixel(73, 73), RED);
}

#[test]
fn outline_does_not_affect_layout() {
    let with = render(
        "<div style=\"width: 40px; height: 20px; outline: 6px solid red\"></div><div style=\"width: 40px; height: 20px; background-color: lime\"></div>",
    );
    let without = render(
        "<div style=\"width: 40px; height: 20px\"></div><div style=\"width: 40px; height: 20px; background-color: lime\"></div>",
    );
    assert_eq!(with.pixel(10, 25), LIME);
    assert_eq!(without.pixel(10, 25), LIME);
    assert_eq!(with.pixel(43, 10), RED);
    assert_eq!(without.pixel(43, 10), WHITE);
}

#[test]
fn outline_offset_moves_the_ring_outward() {
    let canvas = boxed("outline: 4px solid red; outline-offset: 5px");
    assert_eq!(canvas.pixel(22, 50), RED);
    assert_eq!(canvas.pixel(27, 50), WHITE);
    assert_eq!(canvas.pixel(28, 50), WHITE);
    assert_eq!(canvas.pixel(20, 50), WHITE);
}

#[test]
fn negative_offset_draws_over_the_box() {
    let canvas = boxed("outline: 4px solid red; outline-offset: -2px");
    assert_eq!(canvas.pixel(29, 50), RED);
    assert_eq!(canvas.pixel(31, 50), RED);
    assert_eq!(canvas.pixel(33, 50), LIME);
}

#[test]
fn style_none_or_missing_draws_nothing() {
    for style in [
        "outline: none",
        "outline-width: 4px",
        "outline: 4px red",
        "outline: 4px hidden red",
        "outline-width: 4px; outline-color: red",
    ] {
        let canvas = boxed(style);
        assert_eq!(canvas.pixel(28, 50), WHITE, "{}", style);
    }
}

#[test]
fn zero_width_draws_nothing() {
    let canvas = boxed("outline: 0 solid red");
    assert_eq!(canvas.pixel(29, 50), WHITE);
}

#[test]
fn color_defaults_to_the_text_color() {
    let canvas = boxed("color: blue; outline: 2px solid");
    assert_eq!(canvas.pixel(28, 50), BLUE);
    let canvas = boxed("color: blue; outline: 2px solid currentcolor");
    assert_eq!(canvas.pixel(28, 50), BLUE);
}

#[test]
fn width_defaults_to_medium() {
    let canvas = boxed("outline: solid red");
    assert_eq!(canvas.pixel(27, 50), RED);
    assert_eq!(canvas.pixel(26, 50), WHITE);
}

#[test]
fn width_keywords() {
    let thin = boxed("outline: thin solid red");
    assert_eq!(thin.pixel(29, 50), RED);
    assert_eq!(thin.pixel(28, 50), WHITE);
    let thick = boxed("outline: thick solid red");
    assert_eq!(thick.pixel(25, 50), RED);
    assert_eq!(thick.pixel(24, 50), WHITE);
}

#[test]
fn em_widths_use_the_element_font_size() {
    let canvas = boxed("font-size: 10px; outline: 0.5em solid red; outline-offset: 0.2em");
    assert_eq!(canvas.pixel(23, 50), RED);
    assert_eq!(canvas.pixel(22, 50), WHITE);
    assert_eq!(canvas.pixel(27, 50), RED);
}

#[test]
fn longhands_work_together() {
    let canvas = boxed("outline-style: solid; outline-color: red; outline-width: 2px");
    assert_eq!(canvas.pixel(28, 50), RED);
    assert_eq!(canvas.pixel(27, 50), WHITE);
}

#[test]
fn shorthand_resets_the_other_longhands() {
    let canvas =
        boxed("outline-style: solid; outline-color: red; outline-width: 6px; outline: none");
    assert_eq!(canvas.pixel(28, 50), WHITE);
}

#[test]
fn outline_is_not_inherited() {
    let canvas = render(
        "<div style=\"outline: 4px solid red; margin: 30px\"><div style=\"width: 20px; height: 20px; margin: 10px\"></div></div>",
    );
    assert_eq!(canvas.pixel(38, 40), WHITE);
    assert_eq!(canvas.pixel(28, 40), RED);
}

#[test]
fn outline_follows_the_border_box() {
    let canvas = render(
        "<div style=\"margin: 30px; width: 30px; height: 30px; border: 5px solid blue; outline: 3px solid red\"></div>",
    );
    assert_eq!(canvas.pixel(27, 50), RED);
    assert_eq!(canvas.pixel(29, 50), RED);
    assert_eq!(canvas.pixel(30, 50), BLUE);
    assert_eq!(canvas.pixel(34, 50), BLUE);
}

#[test]
fn outline_follows_rounded_corners() {
    let canvas = boxed("border-radius: 10px; outline: 4px solid red");
    assert_eq!(canvas.pixel(26, 26), WHITE);
    assert_eq!(canvas.pixel(50, 28), RED);
    assert_eq!(canvas.pixel(28, 50), RED);
    assert_eq!(canvas.pixel(50, 50), LIME);
}

#[test]
fn outline_is_not_clipped_by_the_elements_own_overflow() {
    let canvas = boxed("overflow: hidden; outline: 4px solid red");
    assert_eq!(canvas.pixel(28, 50), RED);
}

#[test]
fn outline_is_clipped_by_an_ancestor() {
    let canvas = render(
        "<div style=\"width: 50px; height: 50px; overflow: hidden\"><div style=\"width: 50px; height: 50px; outline: 10px solid red\"></div></div>",
    );
    assert_eq!(canvas.pixel(25, 25), WHITE);
    assert_eq!(canvas.pixel(60, 25), WHITE);
}

#[test]
fn outline_paints_over_the_content() {
    let canvas = render(
        "<div style=\"margin: 30px; width: 40px; height: 40px; outline: 4px solid red; outline-offset: -4px\"><div style=\"width: 40px; height: 40px; background-color: lime\"></div></div>",
    );
    assert_eq!(canvas.pixel(31, 50), RED);
    assert_eq!(canvas.pixel(50, 50), LIME);
}

#[test]
fn positioned_elements_get_an_outline() {
    let canvas = render(
        "<div style=\"position: absolute; left: 30px; top: 30px; width: 40px; height: 40px; background-color: lime; outline: 4px solid red\"></div>",
    );
    assert_eq!(canvas.pixel(28, 50), RED);
}

#[test]
fn opacity_applies_to_the_outline() {
    let canvas = boxed("opacity: 0.5; outline: 4px solid red");
    let pixel = canvas.pixel(28, 50);
    assert_ne!(pixel, RED);
    assert_ne!(pixel, WHITE);
}

#[test]
fn alpha_color_blends() {
    let canvas = boxed("outline: 4px solid rgba(255, 0, 0, 0.5)");
    let pixel = canvas.pixel(28, 50);
    assert_ne!(pixel, RED);
    assert_ne!(pixel, WHITE);
    assert!(pixel.r > pixel.g);
}

#[test]
fn shorthand_expands_to_longhands() {
    let declarations = css::parse_declarations("outline: 2px dashed red");
    let names: Vec<&str> = declarations.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["outline-width", "outline-style", "outline-color"]);
    assert_eq!(declarations[0].value, Value::Length(2.0));
    assert_eq!(declarations[1].value, Value::Keyword("dashed".into()));
}

#[test]
fn shorthand_order_is_free() {
    let declarations = css::parse_declarations("outline: red solid 2px");
    assert_eq!(declarations[0].value, Value::Length(2.0));
    assert_eq!(declarations[1].value, Value::Keyword("solid".into()));
}

#[test]
fn invalid_shorthand_is_dropped() {
    for value in ["2px wavy red", "2px 3px 4px nonsense", "foo"] {
        assert!(
            css::parse_declarations(&format!("outline: {}", value)).is_empty(),
            "{}",
            value
        );
    }
}

#[test]
fn supports_the_outline_properties() {
    for property in [
        "outline: 1px solid red",
        "outline-offset: 2px",
        "outline-width: 2px",
        "outline-style: solid",
        "outline-color: red",
    ] {
        assert!(
            foundation::supports::evaluate(&format!("({})", property)),
            "{}",
            property
        );
    }
}
