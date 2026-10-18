// SPDX-License-Identifier: GPL-3.0-only

use foundation::calc::Linear;
use foundation::css::{self, Color, Value};
use foundation::{html, layout, render, style, text};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn declaration(css_text: &str, name: &str) -> Option<Value> {
    let sheet = css::parse(css_text);
    let mut found = None;
    for declaration in &sheet.rules[0].declarations {
        if declaration.name == name {
            found = Some(declaration.value.clone());
        }
    }
    found
}

fn styled_value(source: &str, viewport: (f32, f32), tag: &str, property: &str) -> Option<Value> {
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled =
        style::style_tree(&root, &sheet, style::Viewport::new(viewport.0, viewport.1)).unwrap();
    find_value(&styled, tag, property)
}

fn find_value(node: &style::StyledNode, tag: &str, property: &str) -> Option<Value> {
    if let style::Content::Element(element) = node.content {
        if element.tag_name == tag {
            return node.value(property).cloned();
        }
    }
    for child in &node.children {
        if let Some(value) = find_value(child, tag, property) {
            return Some(value);
        }
    }
    None
}

#[test]
fn css_parses_new_units() {
    let sheet = "p { a: 2rem; b: 5vw; c: 10vh; d: 3vmin; e: 4vmax }";
    assert_eq!(declaration(sheet, "a"), Some(Value::Rem(2.0)));
    assert_eq!(declaration(sheet, "b"), Some(Value::Vw(5.0)));
    assert_eq!(declaration(sheet, "c"), Some(Value::Vh(10.0)));
    assert_eq!(declaration(sheet, "d"), Some(Value::Vmin(3.0)));
    assert_eq!(declaration(sheet, "e"), Some(Value::Vmax(4.0)));
}

#[test]
fn viewport_and_root_units_resolve_in_style_tree() {
    let source = "<html><style>html { font-size: 10px } p { width: 2rem; height: 50vh; margin-left: 10vw; padding-top: 5vmin; padding-bottom: 5vmax }</style><body><p></p></body></html>";
    let get = |property| styled_value(source, (200.0, 100.0), "p", property);
    assert_eq!(get("width"), Some(Value::Length(20.0)));
    assert_eq!(get("height"), Some(Value::Length(50.0)));
    assert_eq!(get("margin-left"), Some(Value::Length(20.0)));
    assert_eq!(get("padding-top"), Some(Value::Length(5.0)));
    assert_eq!(get("padding-bottom"), Some(Value::Length(10.0)));
}

#[test]
fn rem_in_root_font_size_uses_initial_size() {
    let source = "<html><style>html { font-size: 2rem } p { font-size: 1rem }</style><body><p></p></body></html>";
    assert_eq!(
        styled_value(source, (100.0, 100.0), "html", "font-size"),
        Some(Value::Length(32.0))
    );
    assert_eq!(
        styled_value(source, (100.0, 100.0), "p", "font-size"),
        Some(Value::Length(32.0))
    );
}

#[test]
fn calc_parses_arithmetic() {
    let value = |text: &str| declaration(&format!("p {{ w: {} }}", text), "w");
    assert_eq!(value("calc(10px + 5px)"), Some(Value::Length(15.0)));
    assert_eq!(value("calc(2 * (1px + 3px))"), Some(Value::Length(8.0)));
    assert_eq!(value("calc(100px / 4)"), Some(Value::Length(25.0)));
    assert_eq!(value("calc(1px+2px)"), Some(Value::Length(3.0)));
    assert_eq!(value("calc(calc(1px + 1px) * 3)"), Some(Value::Length(6.0)));
    assert_eq!(
        value("calc(100% - 20px)"),
        Some(Value::Calc(Linear {
            px: -20.0,
            percent: 100.0,
            ..Linear::default()
        }))
    );
    assert_eq!(
        value("calc(1em + 2vw)"),
        Some(Value::Calc(Linear {
            em: 1.0,
            vw: 2.0,
            ..Linear::default()
        }))
    );
}

#[test]
fn calc_rejects_invalid_expressions() {
    for text in [
        "calc(10px + 5)",
        "calc(10px * 5px)",
        "calc(10px / 0)",
        "calc(5 / 10px)",
        "calc(10px +)",
        "calc(10px",
        "calc(10px) + 1px",
        "calc(10qq)",
        "calc()",
    ] {
        assert_eq!(
            declaration(&format!("p {{ w: {} }}", text), "w"),
            None,
            "{}",
            text
        );
    }
}

#[test]
fn calc_resolves_in_layout() {
    let source = "<html><style>body { margin: 0; font-size: 10px } div { width: calc(100% - 20px); margin-left: calc(1em + 2px); height: calc(10vh + 1px) }</style><body><div></div></body></html>";
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(200.0, 100.0)).unwrap();
    let font = text::default_font();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(200.0, 600.0),
        &font,
        &layout::ImageMap::new(),
    );
    let div = &layout_root.children[0].children[0];
    assert_eq!(div.dimensions.content.width, 180.0);
    assert_eq!(div.dimensions.margin.left, 12.0);
    assert_eq!(div.dimensions.content.height, 11.0);
}

#[test]
fn calc_in_font_size_and_shorthand() {
    let source = "<html><style>body { font-size: 20px } p { font-size: calc(1em + 4px); margin: calc(1em + 1px) 3px }</style><body><p></p></body></html>";
    assert_eq!(
        styled_value(source, (100.0, 100.0), "p", "font-size"),
        Some(Value::Length(24.0))
    );
    assert_eq!(
        styled_value(source, (100.0, 100.0), "p", "margin-left"),
        Some(Value::Length(3.0))
    );
    assert_eq!(
        styled_value(source, (100.0, 100.0), "p", "margin-top"),
        Some(Value::Calc(Linear {
            px: 1.0,
            em: 1.0,
            ..Linear::default()
        }))
    );
}

#[test]
fn font_shorthand_sets_size_and_line_height() {
    let sheet = "p { font: italic bold 16px/1.6 system-ui, sans-serif }";
    assert_eq!(declaration(sheet, "font-size"), Some(Value::Length(16.0)));
    assert_eq!(declaration(sheet, "line-height"), Some(Value::Number(1.6)));

    let sheet = "p { font: 700 12px Arial }";
    assert_eq!(declaration(sheet, "font-size"), Some(Value::Length(12.0)));
    assert_eq!(
        declaration(sheet, "line-height"),
        Some(Value::Keyword("normal".to_string()))
    );

    let sheet = "p { font: 1.5em / 20px serif }";
    assert_eq!(declaration(sheet, "font-size"), Some(Value::Em(1.5)));
    assert_eq!(declaration(sheet, "line-height"), Some(Value::Length(20.0)));

    let sheet = "p { font: large serif }";
    assert_eq!(declaration(sheet, "font-size"), Some(Value::Length(18.0)));

    assert_eq!(declaration("p { font: bold serif }", "font-size"), None);
    assert_eq!(
        declaration("p { font: 12px/bogus! serif }", "font-size"),
        None
    );
}

#[test]
fn font_size_keywords_resolve() {
    assert_eq!(
        declaration("p { font-size: x-large }", "font-size"),
        Some(Value::Length(24.0))
    );
    assert_eq!(
        declaration("p { font-size: inherit }", "font-size"),
        Some(Value::Keyword("inherit".to_string()))
    );
}

#[test]
fn background_shorthand_extracts_the_color() {
    let color = Some(Value::Color(Color {
        r: 255,
        g: 255,
        b: 255,
    }));
    assert_eq!(
        declaration(
            "p { background: #fff url(a b.png) no-repeat }",
            "background-color"
        ),
        color
    );
    assert_eq!(
        declaration("p { background: no-repeat white }", "background-color"),
        color
    );
    assert_eq!(
        declaration("p { background: none }", "background-color"),
        None
    );
}

#[test]
fn light_dark_uses_the_light_value() {
    let sheet = "p { color: light-dark(#eee, #222); background: light-dark(red, blue) }";
    assert_eq!(
        declaration(sheet, "color"),
        Some(Value::Color(Color {
            r: 238,
            g: 238,
            b: 238
        }))
    );
    assert_eq!(
        declaration(sheet, "background-color"),
        Some(Value::Color(RED))
    );
    assert_eq!(declaration("p { color: light-dark(#eee) }", "color"), None);
}

#[test]
fn viewport_height_follows_the_render_size() {
    let source = "<html><style>body { margin: 0 } div { height: 50vh; background-color: red }</style><body><div></div></body></html>";
    let canvas = render(source, 10, 20);
    assert_eq!(canvas.pixel(0, 9), RED);
    assert_eq!(canvas.pixel(0, 10), WHITE);
    let taller = render(source, 10, 40);
    assert_eq!(taller.pixel(0, 19), RED);
    assert_eq!(taller.pixel(0, 20), WHITE);
}

#[test]
fn root_background_fills_the_canvas() {
    let source = "<html><style>html { background-color: red } body { margin: 0 } div { height: 5px; background-color: lime }</style><body><div></div></body></html>";
    let canvas = render(source, 10, 20);
    assert_eq!(canvas.pixel(0, 0), Color { r: 0, g: 255, b: 0 });
    assert_eq!(canvas.pixel(0, 10), RED);
    assert_eq!(canvas.pixel(9, 19), RED);
}

#[test]
fn body_background_fills_the_canvas_when_root_has_none() {
    let source = "<html><style>body { margin: 0; background: red }</style><body></body></html>";
    let canvas = render(source, 10, 20);
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(9, 19), RED);
}

#[test]
fn root_background_wins_over_body_background() {
    let source = "<html><style>html { background: lime } body { margin: 0; background: red; height: 5px }</style><body></body></html>";
    let canvas = render(source, 10, 20);
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(0, 10), Color { r: 0, g: 255, b: 0 });
}
