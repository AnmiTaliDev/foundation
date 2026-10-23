// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text};

fn with_layout<R>(body: &str, width: f32, f: impl FnOnce(&LayoutBox) -> R) -> R {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} td, th {{ padding: 0; vertical-align: baseline }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    f(&layout_root)
}

fn find(layout_box: &LayoutBox, tag: &str, found: &mut Vec<Rect>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if let Content::Element(element) = style.content {
            if element.tag_name == tag {
                found.push(layout_box.dimensions.border_box());
            }
        }
    }
    for child in &layout_box.children {
        find(child, tag, found);
    }
}

fn rects(body: &str, tag: &str, width: f32) -> Vec<Rect> {
    with_layout(body, width, |root| {
        let mut found = Vec::new();
        find(root, tag, &mut found);
        found
    })
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

fn baselines(table: &str) -> Vec<f32> {
    fn walk(layout_box: &LayoutBox, out: &mut Vec<f32>) {
        for fragment in &layout_box.fragments {
            out.push(fragment.baseline);
        }
        for child in &layout_box.children {
            walk(child, out);
        }
    }
    with_layout(table, 400.0, |root| {
        let mut found = Vec::new();
        walk(root, &mut found);
        found
    })
}

fn line() -> f32 {
    text::line_height(&text::default_font(), 16.0)
}

#[test]
fn baseline_cells_share_the_first_baseline() {
    let table = "<table style=\"width:200px\"><tr><td style=\"font-size: 32px\">A</td><td>b</td></tr></table>";
    let found = baselines(table);
    assert!(close(found[0], found[1]), "{:?}", found);
}

#[test]
fn middle_cells_do_not_share_a_baseline() {
    let table = "<table style=\"width:200px\"><tr><td style=\"font-size: 32px; vertical-align: middle\">A</td><td style=\"vertical-align: middle\">b</td></tr></table>";
    let found = baselines(table);
    assert!(!close(found[0], found[1]), "{:?}", found);
}

#[test]
fn the_smaller_cell_moves_down_to_the_baseline() {
    let aligned = "<table style=\"width:200px\"><tr><td style=\"font-size: 32px\">A</td><td>b</td></tr></table>";
    let top = "<table style=\"width:200px\"><tr><td style=\"font-size: 32px; vertical-align: top\">A</td><td style=\"vertical-align: top\">b</td></tr></table>";
    let aligned = baselines(aligned);
    let top = baselines(top);
    assert!(close(aligned[0], top[0]));
    assert!(aligned[1] > top[1]);
}

#[test]
fn the_row_grows_to_fit_the_shifted_cell() {
    let table = "<table><tr><td style=\"font-size: 32px\">A</td><td style=\"padding-top: 20px\">b</td></tr></table>";
    let cells = rects(table, "td", 400.0);
    let rows = rects(table, "tr", 400.0);
    assert!(close(cells[0].height, cells[1].height));
    assert!(close(rows[0].height, cells[0].height));
    assert!(cells[0].height > 32.0);
}

#[test]
fn padding_is_part_of_the_baseline_offset() {
    let table = "<table style=\"width:200px\"><tr><td style=\"padding-top: 10px\">a</td><td>b</td></tr></table>";
    let found = baselines(table);
    assert!(close(found[0], found[1]), "{:?}", found);
}

#[test]
fn another_value_on_a_cell_opts_out() {
    let table = "<table style=\"width:200px\"><tr><td style=\"font-size: 32px\">A</td><td style=\"vertical-align: bottom\">b</td></tr></table>";
    let found = baselines(table);
    assert!(!close(found[0], found[1]), "{:?}", found);
}

#[test]
fn nested_blocks_use_the_first_line() {
    let table = "<table style=\"width:200px\"><tr><td><div style=\"font-size: 32px\">A</div><div>second</div></td><td>b</td></tr></table>";
    let found = baselines(table);
    assert!(close(found[0], found[2]), "{:?}", found);
}

#[test]
fn spanning_cells_are_aligned_to_the_top() {
    let baseline = "<table style=\"width:200px\"><tr><td rowspan=2 style=\"font-size: 32px\">A</td><td>b</td></tr><tr><td>c</td></tr></table>";
    let top = "<table style=\"width:200px\"><tr><td rowspan=2 style=\"font-size: 32px; vertical-align: top\">A</td><td>b</td></tr><tr><td>c</td></tr></table>";
    assert_eq!(baselines(baseline)[0], baselines(top)[0]);
}

#[test]
fn rows_without_baseline_cells_are_unchanged() {
    let table = "<table><tr><td style=\"vertical-align: middle\">a</td><td style=\"vertical-align: middle\">b</td></tr></table>";
    let rows = rects(table, "tr", 400.0);
    assert!(close(rows[0].height, line()));
}

#[test]
fn empty_baseline_cells_are_left_alone() {
    let table = "<table><tr><td></td><td>b</td></tr></table>";
    let rows = rects(table, "tr", 400.0);
    assert!(close(rows[0].height, line()));
}
