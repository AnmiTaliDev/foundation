// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };

fn first_pixel(style: &str, inline: &str) -> Color {
    let html = format!(
        "<html><head><style>div {{ height: 10px; width: 10px }} {}</style></head><body style=\"margin:0\"><div id=\"a\" style=\"{}\"></div></body></html>",
        style, inline
    );
    Page::new(&html).render(20, 20, 0.0).canvas.pixel(0, 0)
}

#[test]
fn important_is_parsed_and_stripped_from_the_value() {
    let sheet = css::parse(
        "a { width: 5px !important; height: 6px; color: red ! IMPORTANT; margin: 1px!important }",
    );
    let rule = &sheet.rules[0];
    assert_eq!(rule.declarations.len(), 1);
    assert_eq!(rule.declarations[0].name, "height");
    let names: Vec<&str> = rule.important.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(
        names,
        [
            "width",
            "color",
            "margin-top",
            "margin-right",
            "margin-bottom",
            "margin-left"
        ]
    );
    assert_eq!(rule.important[0].value, Value::Length(5.0));
}

#[test]
fn important_beats_higher_specificity_and_later_rules() {
    assert_eq!(
        first_pixel(
            "div { background-color: red !important } #a { background-color: blue }",
            ""
        ),
        RED
    );
    assert_eq!(
        first_pixel(
            "div { background-color: red !important } div { background-color: blue !important }",
            ""
        ),
        BLUE
    );
    assert_eq!(
        first_pixel(
            "#a { background-color: red !important } div { background-color: blue !important }",
            ""
        ),
        RED
    );
}

#[test]
fn inline_style_beats_normal_rules_but_not_important_ones() {
    assert_eq!(
        first_pixel("#a { background-color: blue }", "background-color: red"),
        RED
    );
    assert_eq!(
        first_pixel(
            "#a { background-color: blue !important }",
            "background-color: red"
        ),
        BLUE
    );
    assert_eq!(
        first_pixel(
            "#a { background-color: blue !important }",
            "background-color: red !important"
        ),
        RED
    );
}

#[test]
fn shorthand_important_applies_to_every_longhand() {
    let sheet = css::parse("a { background: red !important }");
    assert_eq!(sheet.rules[0].important.len(), 1);
    assert_eq!(
        first_pixel(
            "div { background: red !important } #a { background-color: blue }",
            ""
        ),
        RED
    );
}

#[test]
fn important_reverses_layer_order() {
    assert_eq!(
        first_pixel(
            "@layer one, two; @layer one { div { background-color: red !important } } @layer two { div { background-color: blue !important } }",
            ""
        ),
        RED
    );
    assert_eq!(
        first_pixel(
            "@layer one { div { background-color: red !important } } div { background-color: blue !important }",
            ""
        ),
        RED
    );
    assert_eq!(
        first_pixel(
            "@layer one { div { background-color: red } } div { background-color: blue }",
            ""
        ),
        BLUE
    );
}

#[test]
fn important_declarations_override_the_user_agent_sheet() {
    let html = "<html><head><style>div { display: inline !important }</style></head><body style=\"margin:0\"><div style=\"width:10px;height:10px;background-color:red\">a</div></body></html>";
    let canvas = Page::new(html).render(40, 20, 0.0).canvas;
    assert_eq!(
        canvas.pixel(30, 2),
        Color {
            r: 255,
            g: 255,
            b: 255
        }
    );
}
