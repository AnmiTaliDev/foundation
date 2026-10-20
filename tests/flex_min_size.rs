// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{BoxKind, ImageMap, LayoutBox};
use foundation::{html, layout, style, text};

fn widths(css: &str, body: &str, container_width: f32) -> Vec<(f32, f32)> {
    let source = format!(
        "<html><style>body {{ margin: 0 }} .c {{ display: flex }} {}</style><body>{}</body></html>",
        css, body
    );
    let root = html::parse(&source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(container_width, 400.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let font = text::default_font();
    let tree = layout::layout_tree(&styled, viewport, &font, &ImageMap::new());
    let container: &LayoutBox = &tree.children[0].children[0];
    assert!(matches!(container.kind, BoxKind::Block(_)));
    container
        .children
        .iter()
        .map(|child| {
            let rect = child.dimensions.border_box();
            (rect.width, rect.height)
        })
        .collect()
}

fn word_width() -> f32 {
    text::text_width(&text::default_font(), "xxxxxxxxxxxxxxxxxxxx", 16.0)
}

const LONG: &str = "<div class=c style=\"width: 50px\"><div id=a>xxxxxxxxxxxxxxxxxxxx</div></div>";

#[test]
fn item_does_not_shrink_below_its_min_content() {
    let result = widths("#a { width: 200px }", LONG, 400.0);
    assert!(word_width() > 50.0);
    assert!((result[0].0 - word_width()).abs() < 0.01, "{:?}", result);
}

#[test]
fn item_may_shrink_to_a_breakable_width() {
    let result = widths(
        "#a { width: 200px }",
        "<div class=c style=\"width: 50px\"><div id=a>ab cd</div></div>",
        400.0,
    );
    assert_eq!(result[0].0, 50.0);
}

#[test]
fn explicit_min_width_zero_allows_shrinking() {
    let result = widths("#a { width: 200px; min-width: 0 }", LONG, 400.0);
    assert_eq!(result[0].0, 50.0);
}

#[test]
fn clipped_overflow_allows_shrinking() {
    let result = widths("#a { width: 200px; overflow: hidden }", LONG, 400.0);
    assert_eq!(result[0].0, 50.0);
}

#[test]
fn specified_width_caps_the_automatic_minimum() {
    let result = widths(
        "#a { width: 30px }",
        "<div class=c style=\"width: 20px\"><div id=a>xxxxxxxxxxxxxxxxxxxx</div></div>",
        400.0,
    );
    assert_eq!(result[0].0, 30.0);
}

#[test]
fn nested_widths_count_toward_min_content() {
    let result = widths(
        "#a { width: 300px } #b { width: 80px; height: 5px }",
        "<div class=c style=\"width: 20px\"><div id=a><div id=b></div></div></div>",
        400.0,
    );
    assert_eq!(result[0].0, 80.0);
}

#[test]
fn column_items_keep_their_content_height() {
    let css = ".c { flex-direction: column; height: 50px } #b { height: 100px }";
    let body = "<div class=c><div id=a><div id=b></div></div></div>";
    assert_eq!(widths(css, body, 400.0)[0].1, 100.0);
    let free = widths(&format!("{} #a {{ min-height: 0 }}", css), body, 400.0);
    assert_eq!(free[0].1, 50.0);
}

#[test]
fn flex_basis_follows_box_sizing() {
    let result = widths(
        "#a { box-sizing: border-box; padding: 10px; flex: 0 0 100px }",
        "<div class=c><div id=a></div></div>",
        400.0,
    );
    assert_eq!(result[0].0, 100.0);
    let content_box = widths(
        "#a { padding: 10px; flex: 0 0 100px }",
        "<div class=c><div id=a></div></div>",
        400.0,
    );
    assert_eq!(content_box[0].0, 120.0);
}

#[test]
fn shrinking_still_shares_space_between_items_without_min_content() {
    let result = widths(
        "#a { width: 100px } #b { width: 200px }",
        "<div class=c style=\"width: 150px\"><div id=a></div><div id=b></div></div>",
        400.0,
    );
    assert_eq!(result[0].0, 50.0);
    assert_eq!(result[1].0, 100.0);
}
