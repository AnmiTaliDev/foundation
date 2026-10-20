// SPDX-License-Identifier: GPL-3.0-only

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
    let root = Box::leak(Box::new(html::parse(&source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    let tree = layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new());
    let mut found = None;
    find(&tree, id, &mut found);
    found.unwrap()
}

const BOX: &str = "padding: 10px; border: 2px solid red;";

fn block(css: &str) -> (f32, f32) {
    size(
        &format!("#a {{ {} {} }}", BOX, css),
        "<div id=a></div>",
        "a",
    )
}

#[test]
fn content_box_adds_padding_and_border() {
    assert_eq!(block("width: 50px; height: 30px"), (74.0, 54.0));
}

#[test]
fn border_box_includes_padding_and_border() {
    assert_eq!(
        block("box-sizing: border-box; width: 50px; height: 80px"),
        (50.0, 80.0)
    );
}

#[test]
fn border_box_min_and_max_widths_include_extras() {
    assert_eq!(
        block("box-sizing: border-box; min-width: 100px; width: 20px").0,
        100.0
    );
    assert_eq!(
        block("box-sizing: border-box; max-width: 100px; width: 300px").0,
        100.0
    );
}

#[test]
fn border_box_min_and_max_heights_include_extras() {
    assert_eq!(block("box-sizing: border-box; min-height: 100px").1, 100.0);
    assert_eq!(
        block("box-sizing: border-box; height: 300px; max-height: 100px").1,
        100.0
    );
}

#[test]
fn border_box_smaller_than_the_extras_clamps_to_them() {
    assert_eq!(
        block("box-sizing: border-box; width: 10px; height: 10px"),
        (24.0, 24.0)
    );
}

#[test]
fn border_box_percent_width_uses_the_container() {
    assert_eq!(block("box-sizing: border-box; width: 50%").0, 200.0);
}

#[test]
fn auto_sizes_are_unaffected() {
    assert_eq!(block("box-sizing: border-box").0, 400.0);
}

#[test]
fn box_sizing_is_not_inherited() {
    let result = size(
        &format!(
            "#p {{ box-sizing: border-box }} #a {{ {} width: 50px }}",
            BOX
        ),
        "<div id=p><div id=a></div></div>",
        "a",
    );
    assert_eq!(result.0, 74.0);
}

#[test]
fn none_style_removes_the_border_from_the_extras() {
    assert_eq!(
        block("box-sizing: border-box; border-style: none; width: 50px").0,
        50.0
    );
}

#[test]
fn flex_items_follow_box_sizing() {
    let result = size(
        &format!(
            "#f {{ display: flex }} #a {{ {} box-sizing: border-box; width: 100px }}",
            BOX
        ),
        "<div id=f><div id=a></div></div>",
        "a",
    );
    assert_eq!(result.0, 100.0);
    let min = size(
        &format!(
            "#f {{ display: flex }} #a {{ {} box-sizing: border-box; width: 20px; min-width: 100px }}",
            BOX
        ),
        "<div id=f><div id=a></div></div>",
        "a",
    );
    assert_eq!(min.0, 100.0);
}

#[test]
fn grid_items_follow_box_sizing() {
    let result = size(
        &format!(
            "#g {{ display: grid }} #a {{ {} box-sizing: border-box; width: 120px; justify-self: start }}",
            BOX
        ),
        "<div id=g><div id=a></div></div>",
        "a",
    );
    assert_eq!(result.0, 120.0);
}

#[test]
fn positioned_floated_and_inline_blocks_follow_box_sizing() {
    for display in ["position: absolute", "float: left", "display: inline-block"] {
        let result = size(
            &format!(
                "#a {{ {} {}; box-sizing: border-box; width: 90px; height: 60px }}",
                BOX, display
            ),
            "<div id=a></div>",
            "a",
        );
        assert_eq!(result, (90.0, 60.0), "{}", display);
    }
}

#[test]
fn supports_the_property() {
    assert!(foundation::supports::evaluate("(box-sizing: border-box)"));
}
