// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Value};
use foundation::paint::Canvas;
use foundation::Page;

const RED: foundation::css::Color = foundation::css::Color { r: 255, g: 0, b: 0 };

const CHILDREN: &str = "<span style=\"display: inline-block; width: 30px; height: 10px\"></span><span style=\"display: inline-block; width: 40px; height: 10px\"></span>";

fn render(outer: &str, inner: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: 100px; {}\"><div style=\"background: red; {}\">{}</div></div></body></html>",
        outer, inner, CHILDREN
    );
    Page::new(&html).render(200, 40, 0.0).canvas
}

fn filled(canvas: &Canvas) -> usize {
    (0..200).filter(|x| canvas.pixel(*x, 2) == RED).count()
}

fn width(style: &str) -> usize {
    filled(&render("", style))
}

#[test]
fn limit_between_min_and_max_content_is_used() {
    assert_eq!(width("width: fit-content(60px)"), 60);
}

#[test]
fn limit_above_max_content_is_capped() {
    assert_eq!(width("width: fit-content(90px)"), 70);
}

#[test]
fn limit_below_min_content_is_raised() {
    assert_eq!(width("width: fit-content(10px)"), 40);
}

#[test]
fn percent_and_em_limits_resolve() {
    assert_eq!(width("width: fit-content(50%)"), 50);
    assert_eq!(width("font-size: 10px; width: fit-content(5em)"), 50);
}

#[test]
fn calc_and_math_limits_resolve() {
    assert_eq!(width("width: fit-content(calc(10px + 40px))"), 50);
    assert_eq!(width("width: fit-content(min(55px, 80px))"), 55);
}

#[test]
fn viewport_units_resolve_in_the_limit() {
    assert_eq!(width("width: fit-content(25vw)"), 50);
}

#[test]
fn works_in_max_width() {
    assert_eq!(width("width: 100%; max-width: fit-content(50px)"), 50);
}

#[test]
fn works_on_floats_and_flex_items() {
    assert_eq!(width("float: left; width: fit-content(60px)"), 60);
    assert_eq!(
        filled(&render(
            "display: flex",
            "flex: none; width: fit-content(60px)"
        )),
        60
    );
}

#[test]
fn invalid_limits_are_dropped() {
    for text in [
        "fit-content()",
        "fit-content(auto)",
        "fit-content(1)",
        "fit-content(red)",
    ] {
        let sheet = css::parse(&format!("a {{ width: {} }}", text));
        assert!(sheet.rules[0].declarations.is_empty(), "{}", text);
    }
    let sheet = css::parse("a { width: fit-content(10px) }");
    assert!(matches!(
        sheet.rules[0].declarations[0].value,
        Value::FitContent(_)
    ));
}

fn column(tracks: &str) -> usize {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"display: grid; width: 200px; grid-template-columns: {}\"><div style=\"background: red\">{}</div><div></div></div></body></html>",
        tracks, CHILDREN
    );
    filled(&Page::new(&html).render(200, 40, 0.0).canvas)
}

#[test]
fn grid_track_grows_up_to_the_limit() {
    assert_eq!(column("fit-content(60px) 20px"), 60);
}

#[test]
fn grid_track_stops_at_max_content() {
    assert_eq!(column("fit-content(150px) 20px"), 70);
}

#[test]
fn grid_track_keeps_min_content() {
    assert_eq!(column("fit-content(10px) 20px"), 40);
}

#[test]
fn grid_percent_limit_uses_the_container() {
    assert_eq!(column("fit-content(25%) 20px"), 50);
}
