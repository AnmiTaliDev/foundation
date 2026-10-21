// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Value};
use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::style::Content;
use foundation::{html, style, text};

fn find(layout_box: &LayoutBox, id: &str, found: &mut Option<(f32, f32)>) {
    if let BoxKind::Block(node) = layout_box.kind {
        if let Content::Element(element) = node.content {
            if element.attributes.get("id").map(String::as_str) == Some(id) {
                let border_box = layout_box.dimensions.border_box();
                *found = Some((border_box.width, border_box.height));
            }
        }
    }
    for child in &layout_box.children {
        find(child, id, found);
    }
}

fn size(css: &str, body: &str, id: &str) -> (f32, f32) {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }} {}</style></head><body>{}</body></html>",
        css, body
    );
    let root = html::parse(&source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let font = text::default_font();
    let tree = layout::layout_tree(&styled, viewport, &font, &layout::ImageMap::new());
    let mut found = None;
    find(&tree, id, &mut found);
    found.unwrap()
}

fn block(css: &str) -> (f32, f32) {
    size(&format!("#a {{ {} }}", css), "<div id=a></div>", "a")
}

#[test]
fn height_follows_the_width() {
    assert_eq!(block("width: 160px; aspect-ratio: 16 / 9"), (160.0, 90.0));
    assert_eq!(block("width: 100px; aspect-ratio: 2"), (100.0, 50.0));
    assert_eq!(block("aspect-ratio: 4 / 1"), (400.0, 100.0));
}

#[test]
fn explicit_height_wins() {
    assert_eq!(
        block("width: 100px; height: 20px; aspect-ratio: 1"),
        (100.0, 20.0)
    );
}

#[test]
fn auto_alone_changes_nothing() {
    assert_eq!(block("width: 100px; aspect-ratio: auto").1, 0.0);
}

#[test]
fn auto_with_a_ratio_uses_the_ratio() {
    assert_eq!(
        block("width: 100px; aspect-ratio: auto 2 / 1"),
        (100.0, 50.0)
    );
}

#[test]
fn padding_and_border_extend_the_content_box_ratio() {
    assert_eq!(
        block("width: 100px; padding: 10px; border: 2px solid red; aspect-ratio: 1"),
        (124.0, 124.0)
    );
}

#[test]
fn border_box_ratio_covers_padding_and_border() {
    assert_eq!(
        block("box-sizing: border-box; width: 100px; padding: 10px; border: 2px solid red; aspect-ratio: 1"),
        (100.0, 100.0)
    );
}

#[test]
fn min_and_max_height_still_clamp() {
    assert_eq!(
        block("width: 100px; aspect-ratio: 1; max-height: 40px").1,
        40.0
    );
    assert_eq!(
        block("width: 100px; aspect-ratio: 4; min-height: 60px").1,
        60.0
    );
}

#[test]
fn percent_width_resolves_before_the_ratio() {
    assert_eq!(block("width: 50%; aspect-ratio: 2"), (200.0, 100.0));
}

#[test]
fn flex_items_keep_their_ratio_instead_of_stretching() {
    let result = size(
        "#f { display: flex; height: 300px } #a { width: 100px; aspect-ratio: 2 }",
        "<div id=f><div id=a></div></div>",
        "a",
    );
    assert_eq!(result, (100.0, 50.0));
}

#[test]
fn column_flex_items_get_the_ratio_height() {
    let result = size(
        "#f { display: flex; flex-direction: column } #a { aspect-ratio: 4 }",
        "<div id=f><div id=a></div></div>",
        "a",
    );
    assert_eq!(result, (400.0, 100.0));
}

#[test]
fn grid_items_keep_their_ratio_instead_of_stretching() {
    let result = size(
        "#g { display: grid; grid-template-columns: 100px; grid-template-rows: 200px } #a { aspect-ratio: 2 }",
        "<div id=g><div id=a></div></div>",
        "a",
    );
    assert_eq!(result, (100.0, 50.0));
}

#[test]
fn inline_blocks_and_floats_follow_the_ratio() {
    for display in ["display: inline-block", "float: left", "position: absolute"] {
        let result = block(&format!("{}; width: 80px; aspect-ratio: 2", display));
        assert_eq!(result, (80.0, 40.0), "{}", display);
    }
}

#[test]
fn parsing_accepts_ratios_and_drops_invalid_values() {
    let sheet = css::parse(
        "a { aspect-ratio: 16/9 } b { aspect-ratio: 0 } c { aspect-ratio: -1 } d { aspect-ratio: wide } e { aspect-ratio: 1 / 0 } f { aspect-ratio: auto }",
    );
    assert_eq!(
        sheet.rules[0].declarations[0].value,
        Value::Number(16.0 / 9.0)
    );
    for rule in &sheet.rules[1..5] {
        assert!(rule.declarations.is_empty());
    }
    assert_eq!(
        sheet.rules[5].declarations[0].value,
        Value::Keyword("auto".to_string())
    );
}

#[test]
fn supports_the_property() {
    assert!(foundation::supports::evaluate("(aspect-ratio: 1 / 1)"));
}

#[test]
fn width_follows_the_height() {
    assert_eq!(block("height: 50px; aspect-ratio: 2"), (100.0, 50.0));
    assert_eq!(block("height: 90px; aspect-ratio: 16 / 9"), (160.0, 90.0));
}

#[test]
fn explicit_width_wins_over_the_height() {
    assert_eq!(
        block("width: 30px; height: 50px; aspect-ratio: 2"),
        (30.0, 50.0)
    );
}

#[test]
fn width_from_height_without_a_ratio_fills_the_container() {
    assert_eq!(block("height: 50px"), (400.0, 50.0));
    assert_eq!(block("height: 50px; aspect-ratio: auto"), (400.0, 50.0));
}

#[test]
fn width_from_height_adds_padding_and_border_to_the_content_box() {
    assert_eq!(
        block("height: 50px; padding: 10px; border: 2px solid red; aspect-ratio: 2"),
        (124.0, 74.0)
    );
}

#[test]
fn border_box_width_from_height_covers_padding_and_border() {
    assert_eq!(
        block("box-sizing: border-box; height: 60px; padding: 10px; border: 2px solid red; aspect-ratio: 2"),
        (60.0 * 2.0, 60.0)
    );
}

#[test]
fn width_from_height_respects_min_and_max_width() {
    assert_eq!(
        block("height: 50px; aspect-ratio: 4; max-width: 120px").0,
        120.0
    );
    assert_eq!(
        block("height: 50px; aspect-ratio: 1; min-width: 80px").0,
        80.0
    );
}

#[test]
fn width_from_height_centers_with_auto_margins() {
    let result = size(
        "#a { height: 50px; aspect-ratio: 2; margin: 0 auto }",
        "<div id=a></div>",
        "a",
    );
    assert_eq!(result, (100.0, 50.0));
}

#[test]
fn width_from_height_applies_to_floats_and_inline_blocks() {
    for display in ["display: inline-block", "float: left", "position: absolute"] {
        let result = block(&format!("{}; height: 40px; aspect-ratio: 2", display));
        assert_eq!(result, (80.0, 40.0), "{}", display);
    }
}

#[test]
fn absolute_boxes_with_both_insets_stretch_instead() {
    let result = size(
        "#a { position: absolute; left: 10px; right: 10px; height: 40px; aspect-ratio: 2 }",
        "<div id=a></div>",
        "a",
    );
    assert_eq!(result, (380.0, 40.0));
}
