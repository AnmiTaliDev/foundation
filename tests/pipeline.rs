// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::dom::NodeType;
use foundation::{html, layout, render, style};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn tag_name(node: &foundation::dom::Node) -> &str {
    match &node.node_type {
        NodeType::Element(element) => &element.tag_name,
        NodeType::Text(_) => "#text",
    }
}

#[test]
fn html_builds_tree_with_attributes_and_void_elements() {
    let root = html::parse(
        "<!DOCTYPE html><html><body><div id=\"a\" class='x y'>hi</div><br><p>t</p><!-- c --></body></html>",
    );
    assert_eq!(tag_name(&root), "html");
    let body = &root.children[0];
    assert_eq!(tag_name(body), "body");
    assert_eq!(body.children.len(), 3);
    let NodeType::Element(div) = &body.children[0].node_type else {
        panic!("expected element");
    };
    assert_eq!(div.id().map(String::as_str), Some("a"));
    assert!(div.classes().contains("x") && div.classes().contains("y"));
    assert_eq!(body.children[0].children.len(), 1);
    assert_eq!(tag_name(&body.children[1]), "br");
    assert!(body.children[1].children.is_empty());
    assert_eq!(tag_name(&body.children[2]), "p");
}

#[test]
fn html_keeps_style_content_as_raw_text() {
    let root = html::parse("<html><style>a > b { color: red; }</style><p>x</p></html>");
    assert_eq!(tag_name(&root.children[0]), "style");
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(tag_name(&root.children[1]), "p");
}

#[test]
fn html_tolerates_unclosed_and_stray_tags() {
    let root = html::parse("<div><p>one</span><p>two");
    assert_eq!(tag_name(&root), "div");
}

#[test]
fn css_parses_selectors_and_expands_shorthand() {
    let sheet = css::parse(
        "div.a#b, p { margin: 1px 2px; color: #fff; } /* c */ span { width: 10px } a > b { color: red }",
    );
    assert_eq!(sheet.rules.len(), 2);
    let first = &sheet.rules[0];
    assert_eq!(first.selectors.len(), 2);
    assert_eq!(first.selectors[0].tag_name.as_deref(), Some("div"));
    assert_eq!(first.selectors[0].id.as_deref(), Some("b"));
    assert_eq!(first.selectors[0].classes, vec!["a".to_string()]);
    assert_eq!(first.selectors[0].specificity(), (1, 1, 1));
    assert_eq!(first.declarations.len(), 5);
    assert_eq!(first.declarations[0].name, "margin-top");
    assert_eq!(first.declarations[1].value, Value::Length(2.0));
    assert_eq!(
        first.declarations[4].value,
        Value::Color(Color {
            r: 255,
            g: 255,
            b: 255
        })
    );
}

#[test]
fn css_skips_malformed_declarations() {
    let sheet = css::parse("p { width: ; height: 5px; color: #12; : 3px }");
    assert_eq!(sheet.rules[0].declarations.len(), 1);
    assert_eq!(sheet.rules[0].declarations[0].name, "height");
}

#[test]
fn style_applies_specificity_and_source_order() {
    let root = html::parse(
        "<html><style>p { width: 1px; } .c { width: 2px; } p { height: 3px; } #i { width: 4px; }</style><p class=\"c\" id=\"i\"></p><p class=\"c\"></p></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet).unwrap();
    let with_id = &styled.children[1];
    let without_id = &styled.children[2];
    assert_eq!(with_id.value("width"), Some(&Value::Length(4.0)));
    assert_eq!(without_id.value("width"), Some(&Value::Length(2.0)));
    assert_eq!(without_id.value("height"), Some(&Value::Length(3.0)));
}

#[test]
fn layout_fills_container_width_with_auto() {
    let root = html::parse(
        "<html><style>body { margin: 0 } div { margin: 10px; height: 5px; }</style><body><div></div></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet).unwrap();
    let layout_root = layout::layout_tree(&styled, 100.0);
    let div = &layout_root.children[0].children[0];
    assert_eq!(div.dimensions.content.x, 10.0);
    assert_eq!(div.dimensions.content.y, 10.0);
    assert_eq!(div.dimensions.content.width, 80.0);
    assert_eq!(div.dimensions.content.height, 5.0);
}

#[test]
fn layout_centers_with_auto_margins() {
    let root = html::parse(
        "<html><style>body { margin: 0 } div { width: 50px; margin-left: auto; margin-right: auto; }</style><body><div></div></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet).unwrap();
    let layout_root = layout::layout_tree(&styled, 100.0);
    let div = &layout_root.children[0].children[0];
    assert_eq!(div.dimensions.content.x, 25.0);
    assert_eq!(div.dimensions.content.width, 50.0);
}

#[test]
fn layout_skips_display_none() {
    let root = html::parse(
        "<html><style>body { margin: 0 } .h { display: none; height: 50px } .v { height: 7px }</style><body><div class=\"h\"></div><div class=\"v\"></div></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet).unwrap();
    let layout_root = layout::layout_tree(&styled, 100.0);
    let body = &layout_root.children[0];
    assert_eq!(body.children.len(), 1);
    assert_eq!(body.dimensions.content.height, 7.0);
}

#[test]
fn render_paints_backgrounds_in_position() {
    let canvas = render(
        "<html><style>body { margin: 0 } .a { height: 10px; background-color: #ff0000 } .b { width: 10px; height: 10px; margin: 5px; background: lime }</style><body><div class=\"a\"></div><div class=\"b\"></div></body></html>",
        30,
        30,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(29, 9), RED);
    assert_eq!(canvas.pixel(0, 10), WHITE);
    assert_eq!(canvas.pixel(4, 15), WHITE);
    assert_eq!(canvas.pixel(5, 15), GREEN);
    assert_eq!(canvas.pixel(14, 24), GREEN);
    assert_eq!(canvas.pixel(15, 15), WHITE);
}

#[test]
fn render_paints_borders() {
    let canvas = render(
        "<html><style>body { margin: 0 } div { width: 6px; height: 6px; border-width: 2px; border-color: red; background-color: lime }</style><body><div></div></body></html>",
        20,
        20,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 5), RED);
    assert_eq!(canvas.pixel(2, 2), GREEN);
    assert_eq!(canvas.pixel(9, 9), RED);
    assert_eq!(canvas.pixel(10, 10), WHITE);
}

#[test]
fn render_clips_content_outside_canvas() {
    let canvas = render(
        "<html><style>body { margin: 0 } div { height: 500px; width: 500px; background-color: red }</style><body><div></div></body></html>",
        10,
        10,
    );
    assert_eq!(canvas.pixel(9, 9), RED);
    assert_eq!(canvas.to_ppm().len(), "P6\n10 10\n255\n".len() + 300);
}
