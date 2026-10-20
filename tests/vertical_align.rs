// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::{html, style, text};

fn tree(body: &str) -> LayoutBox<'static> {
    let source = format!(
        "<html><head><style>body {{ margin: 0 }} .ib {{ display: inline-block; width: 10px }}</style></head><body>{}</body></html>",
        body
    );
    let root = Box::leak(Box::new(html::parse(&source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new())
}

fn baseline(body: &str, word: &str) -> f32 {
    let mut found = Vec::new();
    texts(&tree(body), &mut found);
    found
        .iter()
        .find(|(text, _)| text == word)
        .unwrap_or_else(|| panic!("{} in {:?}", word, found))
        .1
}

fn texts(layout_box: &LayoutBox, found: &mut Vec<(String, f32)>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push((fragment.text.clone(), fragment.baseline));
        }
    }
    for child in &layout_box.children {
        texts(child, found);
    }
}

fn blocks(layout_box: &LayoutBox, found: &mut Vec<(f32, f32)>) {
    if let BoxKind::Block(style) = layout_box.kind {
        if style.display() == style::Display::InlineBlock {
            let rect = layout_box.dimensions.border_box();
            found.push((rect.y, rect.y + rect.height));
        }
    }
    for child in &layout_box.children {
        blocks(child, found);
    }
}

fn block_edges(body: &str) -> Vec<(f32, f32)> {
    let mut found = Vec::new();
    blocks(&tree(body), &mut found);
    found
}

fn line_box_height(body: &str) -> f32 {
    let root = tree(body);
    root.dimensions.content.height
}

#[test]
fn baseline_is_the_default() {
    let plain = baseline("a <span>b</span>", "b");
    let explicit = baseline("a <span style=\"vertical-align: baseline\">b</span>", "b");
    assert_eq!(plain, explicit);
}

#[test]
fn super_raises_the_text() {
    let body = "<span>a</span> <sup style=\"vertical-align: super\">b</sup>";
    let main = baseline(body, "a");
    let raised = baseline(body, "b");
    assert!(raised < main - 4.0, "{} {}", raised, main);
}

#[test]
fn sub_lowers_the_text() {
    let main = baseline(
        "<span>a</span> <span style=\"vertical-align: sub\">b</span>",
        "a",
    );
    let lowered = baseline(
        "<span>a</span> <span style=\"vertical-align: sub\">b</span>",
        "b",
    );
    assert!(lowered > main + 2.0);
}

#[test]
fn raising_a_word_moves_the_rest_of_the_line_down_when_it_pokes_out() {
    let plain = line_box_height("<p style=\"margin: 0\">a b</p>");
    let raised = line_box_height(
        "<p style=\"margin: 0\">a <span style=\"vertical-align: 30px\">b</span></p>",
    );
    assert!(raised > plain + 20.0, "{} {}", raised, plain);
}

#[test]
fn a_length_raises_by_that_amount() {
    let main = baseline(
        "<span>a</span><span style=\"vertical-align: 7px\">b</span>",
        "a",
    );
    let shifted = baseline(
        "<span>a</span><span style=\"vertical-align: 7px\">b</span>",
        "b",
    );
    assert!((main - shifted - 7.0).abs() < 0.01, "{} {}", main, shifted);
}

#[test]
fn a_negative_length_lowers() {
    let main = baseline(
        "<span>a</span><span style=\"vertical-align: -5px\">b</span>",
        "a",
    );
    let shifted = baseline(
        "<span>a</span><span style=\"vertical-align: -5px\">b</span>",
        "b",
    );
    assert!((shifted - main - 5.0).abs() < 0.01);
}

#[test]
fn em_and_percent_scale() {
    let main = baseline(
        "<span>a</span><span style=\"vertical-align: 0.5em\">b</span>",
        "a",
    );
    let em = baseline(
        "<span>a</span><span style=\"vertical-align: 0.5em\">b</span>",
        "b",
    );
    assert!((main - em - 8.0).abs() < 0.01);
    let main = baseline(
        "<span style=\"line-height: 20px\">a</span><span style=\"line-height: 20px; vertical-align: 50%\">b</span>",
        "a",
    );
    let percent = baseline(
        "<span style=\"line-height: 20px\">a</span><span style=\"line-height: 20px; vertical-align: 50%\">b</span>",
        "b",
    );
    assert!((main - percent - 10.0).abs() < 0.01);
}

#[test]
fn nested_shifts_add_up() {
    let body = "<span>a</span><span style=\"vertical-align: 4px\"><span style=\"vertical-align: 3px\">b</span></span>";
    let main = baseline(body, "a");
    let nested = baseline(body, "b");
    assert!((main - nested - 7.0).abs() < 0.01);
}

#[test]
fn a_baseline_child_follows_a_raised_parent() {
    let body = "<span>a</span><span style=\"vertical-align: 6px\"><span>b</span></span>";
    let main = baseline(body, "a");
    let child = baseline(body, "b");
    assert!((main - child - 6.0).abs() < 0.01);
}

#[test]
fn the_vertical_align_of_a_parent_does_not_leak_to_siblings() {
    let body = "<span style=\"vertical-align: 6px\">b</span><span>c</span><span>a</span>";
    let main = baseline(body, "a");
    let after = baseline(body, "c");
    assert_eq!(main, after);
}

#[test]
fn middle_centers_an_inline_block_on_the_text() {
    let tall = "<span class=\"ib\" style=\"height: 40px; vertical-align: middle\"></span>text";
    let edges = block_edges(tall);
    let word = baseline(tall, "text");
    let middle = (edges[0].0 + edges[0].1) / 2.0;
    let x_middle = word - 0.27 * 16.0;
    assert!((middle - x_middle).abs() < 0.5, "{} {}", middle, x_middle);
}

#[test]
fn top_aligns_to_the_top_of_the_line() {
    let body = "<span class=\"ib\" style=\"height: 10px; vertical-align: top\"></span><span class=\"ib\" style=\"height: 50px\"></span>";
    let edges = block_edges(body);
    assert_eq!(edges[0].0, edges[1].0);
}

#[test]
fn bottom_aligns_to_the_bottom_of_the_line() {
    let body = "<span class=\"ib\" style=\"height: 10px; vertical-align: bottom\"></span><span class=\"ib\" style=\"height: 50px\"></span>";
    let edges = block_edges(body);
    let line_bottom = edges[1].1 + 0.0;
    assert!(edges[0].1 >= line_bottom - 0.01);
}

#[test]
fn top_grows_the_line_when_taller_than_everything_else() {
    let plain = line_box_height("<p style=\"margin: 0\">a</p>");
    let tall = line_box_height("<p style=\"margin: 0\">a<span class=\"ib\" style=\"height: 80px; vertical-align: top\"></span></p>");
    assert!(tall >= 80.0);
    assert!(tall > plain);
}

#[test]
fn text_top_and_text_bottom_move_in_opposite_directions() {
    let body = "<span>a</span><span class=\"ib\" style=\"height: 4px; vertical-align: text-top\"></span><span class=\"ib\" style=\"height: 4px; vertical-align: text-bottom\"></span>";
    let edges = block_edges(body);
    assert!(edges[0].0 < edges[1].0);
}

#[test]
fn invalid_values_fall_back_to_baseline() {
    let main = baseline(
        "<span>a</span><span style=\"vertical-align: sideways\">b</span>",
        "a",
    );
    let other = baseline(
        "<span>a</span><span style=\"vertical-align: sideways\">b</span>",
        "b",
    );
    assert_eq!(main, other);
}

#[test]
fn table_cells_still_use_vertical_align() {
    let source = "<html><head><style>body { margin: 0 } td { padding: 0; height: 40px; vertical-align: bottom }</style></head><body><table><tr><td>x</td></tr></table></body></html>";
    let root = Box::leak(Box::new(html::parse(source)));
    let sheet = Box::leak(Box::new(style::stylesheet_for_document(root)));
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = Box::leak(Box::new(style::style_tree(root, sheet, viewport).unwrap()));
    let fonts = Box::leak(Box::new(text::default_font()));
    let layout_root = layout::layout_tree(styled, viewport, fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    texts(&layout_root, &mut found);
    assert!(found[0].1 > 30.0);
}

#[test]
fn supports_recognizes_vertical_align() {
    assert!(foundation::supports::evaluate("(vertical-align: middle)"));
}

#[test]
fn sub_and_sup_have_default_styles() {
    let body = "<span>a</span><sup>b</sup><sub>c</sub>";
    let main = baseline(body, "a");
    assert!(baseline(body, "b") < main - 4.0);
    assert!(baseline(body, "c") > main + 2.0);
}
