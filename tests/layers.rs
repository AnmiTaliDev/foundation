// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color};
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };

fn first_pixel(style: &str) -> Color {
    let html = format!(
        "<html><head><style>{}</style></head><body style=\"margin:0\"><div id=\"a\" class=\"b\"></div></body></html>",
        style
    );
    Page::new(&html).render(20, 20, 0.0).canvas.pixel(0, 0)
}

const BOX: &str = "div { height: 10px; width: 10px }";

#[test]
fn unlayered_rules_beat_layered_rules_of_any_specificity() {
    assert_eq!(
        first_pixel(&format!(
            "{} @layer base {{ #a {{ background-color: blue }} }} div {{ background-color: red }}",
            BOX
        )),
        RED
    );
    assert_eq!(
        first_pixel(&format!(
            "{} div {{ background-color: red }} @layer base {{ #a {{ background-color: blue }} }}",
            BOX
        )),
        RED
    );
}

#[test]
fn later_layers_beat_earlier_layers() {
    assert_eq!(
        first_pixel(&format!(
            "{} @layer one {{ div {{ background-color: red }} }} @layer two {{ div {{ background-color: blue }} }}",
            BOX
        )),
        BLUE
    );
    assert_eq!(
        first_pixel(&format!(
            "{} @layer two {{ div {{ background-color: blue }} }} @layer one {{ #a {{ background-color: red }} }}",
            BOX
        )),
        RED
    );
}

#[test]
fn a_layer_statement_fixes_the_order_up_front() {
    assert_eq!(
        first_pixel(&format!(
            "{} @layer one, two; @layer two {{ div {{ background-color: blue }} }} @layer one {{ #a {{ background-color: red }} }}",
            BOX
        )),
        BLUE
    );
}

#[test]
fn reopening_a_layer_keeps_its_original_position() {
    assert_eq!(
        first_pixel(&format!(
            "{} @layer one {{ div {{ height: 10px }} }} @layer two {{ div {{ background-color: blue }} }} @layer one {{ #a {{ background-color: red }} }}",
            BOX
        )),
        BLUE
    );
}

#[test]
fn layers_beat_the_user_agent_stylesheet() {
    let html = "<html><head><style>@layer l { div { margin-top: 5px; background-color: red; height: 10px; width: 10px } }</style></head><body style=\"margin:0\"><div></div></body></html>";
    let canvas = Page::new(html).render(20, 20, 0.0).canvas;
    assert_ne!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(0, 5), RED);
}

#[test]
fn nested_layers_are_registered_with_dotted_paths() {
    let sheet = css::parse("@layer a { @layer b { x { y: 1px } } } @layer a.c, d;");
    assert_eq!(sheet.layers, ["a", "a.b", "a.c", "d"]);
    assert_eq!(sheet.rules[0].layer.as_deref(), Some("a.b"));
}

#[test]
fn anonymous_layers_are_unique() {
    let sheet = css::parse("@layer { x { y: 1px } } @layer { z { y: 1px } }");
    assert_eq!(sheet.layers.len(), 2);
    assert_ne!(sheet.rules[0].layer, sheet.rules[1].layer);
}
