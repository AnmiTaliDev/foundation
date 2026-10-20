// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::dom::NodeType;
use foundation::{entities, html, layout, render, style, text};

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
        "div.a#b, p { margin: 1px 2px; color: #fff; } /* c */ span { width: 10px } a::before { color: red }",
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
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
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
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(100.0, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
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
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(100.0, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
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
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(100.0, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
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

fn first_anonymous<'a, 'b>(
    layout_box: &'b layout::LayoutBox<'a>,
) -> Option<&'b layout::LayoutBox<'a>> {
    if let layout::BoxKind::Anonymous(_) = layout_box.kind {
        return Some(layout_box);
    }
    for child in &layout_box.children {
        if let Some(found) = first_anonymous(child) {
            return Some(found);
        }
    }
    None
}

#[test]
fn inline_runs_share_a_line_and_respect_spaces() {
    let root = html::parse(
        "<html><style>body { margin: 0 }</style><body><p>a<b>b</b> c</p></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let font = text::default_font();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(500.0, 600.0),
        &font,
        &layout::ImageMap::new(),
    );
    let anonymous = first_anonymous(&layout_root).unwrap();
    let fragments = &anonymous.fragments;
    assert_eq!(fragments.len(), 3);
    assert_eq!(fragments[0].baseline, fragments[2].baseline);
    let a_end = fragments[0].x + text::text_width(&font, "a", 16.0);
    assert!((fragments[1].x - a_end).abs() < 0.001);
    let b_end = fragments[1].x + text::text_width(&font, "b", 16.0);
    assert!(fragments[2].x > b_end);
}

#[test]
fn text_wraps_at_container_width() {
    let root = html::parse(
        "<html><style>body { margin: 0 }</style><body><p>one two three four five six</p></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let font = text::default_font();
    let wide = layout::layout_tree(
        &styled,
        style::Viewport::new(1000.0, 600.0),
        &font,
        &layout::ImageMap::new(),
    );
    let narrow = layout::layout_tree(
        &styled,
        style::Viewport::new(60.0, 600.0),
        &font,
        &layout::ImageMap::new(),
    );
    let wide_text = first_anonymous(&wide).unwrap();
    let narrow_text = first_anonymous(&narrow).unwrap();
    assert_eq!(
        wide_text.dimensions.content.height,
        text::line_height(&font, 16.0)
    );
    assert!(narrow_text.dimensions.content.height > wide_text.dimensions.content.height * 2.0);
    for fragment in &narrow_text.fragments {
        assert!(fragment.x + text::text_width(&font, &fragment.text, 16.0) <= 60.0 + 0.001);
    }
}

#[test]
fn text_inherits_color_and_size() {
    let root = html::parse(
        "<html><style>body { margin: 0; color: red; font-size: 40px }</style><body><span>I</span></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(200.0, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
    let fragment = &first_anonymous(&layout_root).unwrap().fragments[0];
    assert_eq!(fragment.color, RED);
    assert_eq!(fragment.font_size, 40.0);
}

#[test]
fn render_draws_text_pixels() {
    let canvas = render(
        "<html><style>body { margin: 0; font-size: 40px }</style><body>I</body></html>",
        60,
        60,
    );
    let mut dark = 0;
    let mut outside = 0;
    for y in 0..60 {
        for x in 0..60 {
            if canvas.pixel(x, y) != WHITE {
                dark += 1;
                if x >= 30 {
                    outside += 1;
                }
            }
        }
    }
    assert!(dark > 20);
    assert_eq!(outside, 0);
}

fn styled_value(source: &str, tag: &str, property: &str) -> Option<Value> {
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
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
fn entities_decode_named_and_numeric() {
    assert_eq!(
        entities::decode("a &amp; b &lt;c&gt; &quot;d&quot;"),
        "a & b <c> \"d\""
    );
    assert_eq!(entities::decode("&#65;&#x42;&#X43;"), "ABC");
    assert_eq!(
        entities::decode("&unknown; &amp &#xZZ; &;"),
        "&unknown; &amp &#xZZ; &;"
    );
    assert_eq!(entities::decode("no entities"), "no entities");
}

#[test]
fn html_decodes_entities_in_text_and_attributes() {
    let root = html::parse("<p title=\"a &amp; b\">x &lt; y</p>");
    let NodeType::Element(element) = &root.node_type else {
        panic!("expected element");
    };
    assert_eq!(
        element.attributes.get("title").map(String::as_str),
        Some("a & b")
    );
    assert_eq!(
        root.children[0].node_type,
        NodeType::Text("x < y".to_string())
    );
}

#[test]
fn html_does_not_decode_entities_in_style() {
    let root = html::parse("<style>a::after { content: \"&amp;\" }</style>");
    assert_eq!(
        root.children[0].node_type,
        NodeType::Text("a::after { content: \"&amp;\" }".to_string())
    );
}

#[test]
fn style_attribute_overrides_rules() {
    let source = "<html><style>#i { width: 10px; height: 4px }</style><p id=\"i\" style=\"width: 20px; color: red\"></p></html>";
    assert_eq!(
        styled_value(source, "p", "width"),
        Some(Value::Length(20.0))
    );
    assert_eq!(
        styled_value(source, "p", "height"),
        Some(Value::Length(4.0))
    );
    assert_eq!(styled_value(source, "p", "color"), Some(Value::Color(RED)));
}

#[test]
fn descendant_and_child_combinators_match_ancestors() {
    let source = "<html><style>div p { width: 1px } div > span { width: 2px } div > b { width: 3px } body div p { height: 5px }</style><body><div><p></p><span></span><section><b></b><p id=\"deep\"></p></section></div></body></html>";
    assert_eq!(styled_value(source, "p", "width"), Some(Value::Length(1.0)));
    assert_eq!(
        styled_value(source, "p", "height"),
        Some(Value::Length(5.0))
    );
    assert_eq!(
        styled_value(source, "span", "width"),
        Some(Value::Length(2.0))
    );
    assert_eq!(styled_value(source, "b", "width"), None);
}

#[test]
fn combinator_selectors_parse_and_sum_specificity() {
    let sheet =
        css::parse("div.a > p#b span { color: red } > p { color: red } div > { color: red }");
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(sheet.rules[0].selectors[0].specificity(), (1, 1, 3));
}

#[test]
fn border_shorthand_sets_width_and_color() {
    let sheet = css::parse("div { border: 2px solid red } p { border: none }");
    let div = &sheet.rules[0].declarations;
    assert_eq!(div.len(), 5);
    assert!(div
        .iter()
        .any(|d| d.name == "border-color" && d.value == Value::Color(RED)));
    assert!(div
        .iter()
        .filter(|d| d.name.ends_with("-width"))
        .all(|d| d.value == Value::Length(2.0)));
    let p = &sheet.rules[1].declarations;
    assert_eq!(p.len(), 4);
    assert!(p.iter().all(|d| d.value == Value::Length(0.0)));
}

#[test]
fn em_and_percent_resolve_in_layout() {
    let root = html::parse(
        "<html><style>body { margin: 0; font-size: 20px } div { width: 50%; margin-left: 1.5em; padding-top: 10%; height: 2em }</style><body><div></div></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(200.0, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
    let div = &layout_root.children[0].children[0];
    assert_eq!(div.dimensions.content.width, 100.0);
    assert_eq!(div.dimensions.margin.left, 30.0);
    assert_eq!(div.dimensions.padding.top, 20.0);
    assert_eq!(div.dimensions.content.height, 40.0);
}

#[test]
fn font_size_em_is_relative_to_parent() {
    let source = "<html><style>body { font-size: 20px } div { font-size: 1.5em } p { font-size: 50% }</style><body><div><p></p></div></body></html>";
    assert_eq!(
        styled_value(source, "div", "font-size"),
        Some(Value::Length(30.0))
    );
    assert_eq!(
        styled_value(source, "p", "font-size"),
        Some(Value::Length(15.0))
    );
}

#[test]
fn page_render_scrolls_and_clamps() {
    let page = foundation::Page::new(
        "<html><style>body { margin: 0 } .a { height: 50px; background-color: red } .b { height: 50px; background-color: lime }</style><body><div class=\"a\"></div><div class=\"b\"></div></body></html>",
    );
    let top = page.render(10, 20, 0.0);
    assert_eq!(top.content_height, 100.0);
    assert_eq!(top.canvas.pixel(0, 0), RED);
    assert_eq!(top.canvas.pixel(0, 19), RED);

    let middle = page.render(10, 20, 40.0);
    assert_eq!(middle.scroll_y, 40.0);
    assert_eq!(middle.canvas.pixel(0, 9), RED);
    assert_eq!(middle.canvas.pixel(0, 10), GREEN);

    let bottom = page.render(10, 20, 1000.0);
    assert_eq!(bottom.scroll_y, 80.0);
    assert_eq!(bottom.canvas.pixel(0, 19), GREEN);

    assert_eq!(page.render(10, 20, -5.0).scroll_y, 0.0);
    assert_eq!(page.render(10, 200, 50.0).scroll_y, 0.0);
}

fn with_layout(source: &str, width: f32, check: impl FnOnce(&layout::LayoutBox)) {
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(width, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
    check(&layout_root);
}

#[test]
fn sibling_margins_collapse() {
    with_layout(
        "<html><style>body { margin: 0 } div { margin: 10px 0; height: 10px }</style><body><div></div><div></div></body></html>",
        100.0,
        |root| {
            let body = &root.children[0];
            assert_eq!(body.dimensions.margin.top, 10.0);
            assert_eq!(body.dimensions.margin.bottom, 10.0);
            assert_eq!(body.children[0].dimensions.content.y, 10.0);
            assert_eq!(body.children[1].dimensions.content.y, 30.0);
            assert_eq!(body.dimensions.content.height, 30.0);
        },
    );
}

#[test]
fn padding_prevents_parent_child_collapse() {
    with_layout(
        "<html><style>body { margin: 0; padding-top: 1px } div { margin-top: 10px; height: 5px }</style><body><div></div></body></html>",
        100.0,
        |root| {
            let body = &root.children[0];
            assert_eq!(body.dimensions.margin.top, 0.0);
            assert_eq!(body.children[0].dimensions.content.y, 11.0);
        },
    );
}

#[test]
fn mixed_sign_margins_are_summed() {
    with_layout(
        "<html><style>body { margin: 0 } .a { height: 10px; margin-bottom: 10px } .b { height: 10px; margin-top: -5px }</style><body><div class=\"a\"></div><div class=\"b\"></div></body></html>",
        100.0,
        |root| {
            let body = &root.children[0];
            assert_eq!(body.children[1].dimensions.content.y, 15.0);
        },
    );
}

#[test]
fn min_and_max_sizes_clamp() {
    with_layout(
        "<html><style>body { margin: 0 } .a { width: 300px; max-width: 100px } .b { min-width: 150px; height: 5px } .c { height: 50px; max-height: 20px } .d { min-height: 30px }</style><body><div class=\"a\"></div><div class=\"b\"></div><div class=\"c\"></div><div class=\"d\"></div></body></html>",
        120.0,
        |root| {
            let body = &root.children[0];
            assert_eq!(body.children[0].dimensions.content.width, 100.0);
            assert_eq!(body.children[1].dimensions.content.width, 150.0);
            assert_eq!(body.children[2].dimensions.content.height, 20.0);
            assert_eq!(body.children[3].dimensions.content.height, 30.0);
        },
    );
}

#[test]
fn text_align_moves_lines() {
    let font = text::default_font();
    let word_width = text::text_width(&font, "abc", 16.0);
    for (align, expected) in [
        ("left", 0.0),
        ("center", (200.0 - word_width) / 2.0),
        ("right", 200.0 - word_width),
    ] {
        let source = format!(
            "<html><style>body {{ margin: 0; text-align: {} }}</style><body><p>abc</p></body></html>",
            align
        );
        with_layout(&source, 200.0, |root| {
            let anonymous = first_anonymous(root).unwrap();
            assert!(
                (anonymous.fragments[0].x - expected).abs() < 0.001,
                "{}",
                align
            );
        });
    }
}

#[test]
fn line_height_sets_line_box_height() {
    for (value, expected) in [("2", 32.0), ("40px", 40.0), ("1.5em", 24.0), ("200%", 32.0)] {
        let source = format!(
            "<html><style>body {{ margin: 0; line-height: {} }}</style><body>text</body></html>",
            value
        );
        with_layout(&source, 200.0, |root| {
            let anonymous = first_anonymous(root).unwrap();
            assert!(
                (anonymous.dimensions.content.height - expected).abs() < 0.001,
                "{}",
                value
            );
        });
    }
}

#[test]
fn css_keeps_unitless_numbers() {
    let sheet = css::parse("p { line-height: 1.5; width: 5; height: -infpx; margin: 0 }");
    let names: Vec<&str> = sheet.rules[0]
        .declarations
        .iter()
        .map(|d| d.name.as_str())
        .collect();
    assert_eq!(sheet.rules[0].declarations[0].value, Value::Number(1.5));
    assert_eq!(sheet.rules[0].declarations[1].value, Value::Number(5.0));
    assert!(!names.contains(&"height"));
}

#[test]
fn long_words_break_across_lines() {
    let word = "abcdefghijklmnopqrstuvwxyz";
    let source = format!(
        "<html><style>body {{ margin: 0 }}</style><body>{}</body></html>",
        word
    );
    let font = text::default_font();
    with_layout(&source, 60.0, |root| {
        let anonymous = first_anonymous(root).unwrap();
        let joined: String = anonymous
            .fragments
            .iter()
            .map(|f| f.text.as_str())
            .collect();
        assert_eq!(joined, word);
        assert!(anonymous.fragments.len() > 2);
        for fragment in &anonymous.fragments {
            assert!(text::text_width(&font, &fragment.text, 16.0) <= 60.0 + 0.001);
            assert_eq!(fragment.x, 0.0);
        }
    });
}

#[test]
fn inline_background_covers_words_and_gap() {
    let source = "<html><style>body { margin: 0 } span { background-color: lime }</style><body>x <span>ab cd</span></body></html>";
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(800.0, 600.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(200.0, 600.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
    let anonymous = first_anonymous(&layout_root).unwrap();
    let fragments = &anonymous.fragments;
    assert_eq!(fragments.len(), 3);
    assert_eq!(anonymous.inline_boxes.len(), 1);
    let inline_box = &anonymous.inline_boxes[0];
    let first = inline_box.rect;
    assert_eq!(inline_box.background, Some(GREEN));
    assert!((first.x - fragments[1].x).abs() < 0.001);
    let font = text::default_font();
    let end = fragments[2].x + text::text_width(&font, "cd", 16.0);
    assert!((first.x + first.width - end).abs() < 0.001);
    assert!(first.x > fragments[0].x);
    assert!(first.height > 0.0);

    let canvas = render(source, 200, 40);
    let x = (first.x + 1.0) as usize;
    let y = (first.y + 1.0).round() as usize;
    assert_eq!(canvas.pixel(x, y), GREEN);
}

#[test]
fn text_before_a_block_keeps_the_parents_alignment() {
    let source = "<html><style>body { margin: 0 } .c { text-align: center }</style><body>left text<div class=\"c\">centered</div></body></html>";
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(200.0, 100.0)).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        style::Viewport::new(200.0, 100.0),
        &text::default_font(),
        &layout::ImageMap::new(),
    );
    let first = first_anonymous(&layout_root).unwrap();
    assert_eq!(first.fragments[0].x, 0.0);
    let centered = &layout_root.children[0].children[1].children[0];
    assert!(centered.fragments[0].x > 0.0);
}
