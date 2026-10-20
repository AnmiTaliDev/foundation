// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{BoxKind, ImageMap, LayoutBox};
use foundation::{html, layout, style, text};

const WORD: &str = "xxxxxxxxxxxxxxxxxxxx";

fn columns(css: &str, first: &str) -> Vec<(f32, f32)> {
    let source = format!(
        "<html><style>body {{ margin: 0 }} .c {{ display: grid; grid-template-columns: 1fr 1fr; width: 100px }} {}</style><body><div class=c><div id=a>{}</div><div>y</div></div></body></html>",
        css, first
    );
    let root = html::parse(&source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 400.0);
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
            (rect.x, rect.width)
        })
        .collect()
}

fn word_width() -> f32 {
    text::text_width(&text::default_font(), WORD, 16.0)
}

#[test]
fn fr_track_grows_to_the_min_content_of_its_item() {
    let result = columns("", WORD);
    assert!((result[0].1 - word_width()).abs() < 0.01, "{:?}", result);
    assert!(word_width() > 50.0);
}

#[test]
fn zero_min_width_lets_the_track_shrink() {
    let result = columns("#a { min-width: 0 }", WORD);
    assert_eq!(result[0].1, 50.0);
    assert_eq!(result[1].0, 50.0);
}

#[test]
fn clipped_overflow_lets_the_track_shrink() {
    let result = columns("#a { overflow: hidden }", WORD);
    assert_eq!(result[0].1, 50.0);
}

#[test]
fn a_wide_specified_width_does_not_stretch_the_track() {
    let result = columns("#a { width: 200px }", "ab");
    assert_eq!(result[0].1, 200.0);
    assert_eq!(result[1].0, 50.0);
}

#[test]
fn specified_width_caps_the_minimum_contribution() {
    let result = columns("#a { width: 30px }", WORD);
    assert_eq!(result[1].0, 50.0);
}

#[test]
fn explicit_min_width_raises_the_track() {
    let result = columns("#a { min-width: 80px }", "ab");
    assert_eq!(result[1].0, 80.0);
}

#[test]
fn border_box_min_width_counts_padding() {
    let result = columns(
        "#a { box-sizing: border-box; padding: 10px; min-width: 80px }",
        "ab",
    );
    assert_eq!(result[1].0, 80.0);
}

#[test]
fn padding_and_border_add_to_the_contribution() {
    let result = columns("#a { padding: 0 10px }", WORD);
    assert!(
        (result[1].0 - word_width() - 20.0).abs() < 0.01,
        "{:?}",
        result
    );
}

#[test]
fn auto_tracks_still_size_to_content() {
    let source = "<html><style>body { margin: 0 } .c { display: grid; grid-template-columns: auto auto }</style><body><div class=c><div>aaa</div><div>b</div></div></body></html>";
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 400.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let font = text::default_font();
    let tree = layout::layout_tree(&styled, viewport, &font, &ImageMap::new());
    let container: &LayoutBox = &tree.children[0].children[0];
    let first = container.children[0].dimensions.border_box();
    let second = container.children[1].dimensions.border_box();
    assert!(first.width > second.width);
    assert_eq!(second.x, first.width);
    let difference = text::text_width(&font, "aaa", 16.0) - text::text_width(&font, "b", 16.0);
    assert!((first.width - second.width - difference).abs() < 0.01);
}
