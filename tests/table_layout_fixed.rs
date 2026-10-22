// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text};

fn with_layout<R>(body: &str, width: f32, f: impl FnOnce(&LayoutBox) -> R) -> R {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} td, th {{ padding: 0 }}</style></head><body>{}</body></html>",
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

fn widths(body: &str) -> Vec<f32> {
    rects(body, "td", 400.0).iter().map(|r| r.width).collect()
}

#[test]
fn content_does_not_change_fixed_columns() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:300px\"><tr><td>a</td><td>b</td><td>c</td></tr><tr><td>a very long first cell</td><td>b</td><td>c</td></tr></table>",
    );
    for cell in &cells {
        assert!(close(*cell, 292.0 / 3.0), "{:?}", cells);
    }
}

#[test]
fn first_row_widths_set_the_columns() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:300px\"><tr><td style=\"width:50px\">a</td><td style=\"width:100px\">b</td><td>c</td></tr></table>",
    );
    assert!(close(cells[0], 50.0), "{:?}", cells);
    assert!(close(cells[1], 100.0), "{:?}", cells);
    assert!(close(cells[2], 142.0), "{:?}", cells);
}

#[test]
fn later_rows_do_not_set_columns() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:200px\"><tr><td>a</td><td>b</td></tr><tr><td style=\"width:10px\">a</td><td>b</td></tr></table>",
    );
    assert!(
        close(cells[0], 97.0) && close(cells[1], 97.0),
        "{:?}",
        cells
    );
}

#[test]
fn col_widths_win_over_cells() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:300px\"><col style=\"width:80px\"><tr><td style=\"width:20px\">a</td><td>b</td></tr></table>",
    );
    assert!(close(cells[0], 80.0), "{:?}", cells);
    assert!(close(cells[1], 214.0), "{:?}", cells);
}

#[test]
fn percent_widths_resolve_against_the_table() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:200px\"><tr><td style=\"width:25%\">a</td><td>b</td></tr></table>",
    );
    assert!(close(cells[0], 50.0), "{:?}", cells);
    assert!(close(cells[1], 144.0), "{:?}", cells);
}

#[test]
fn colspan_splits_the_width() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:300px\"><tr><td colspan=2 style=\"width:100px\">a</td><td>b</td></tr><tr><td>c</td><td>d</td><td>e</td></tr></table>",
    );
    assert!(close(cells[0], 100.0), "{:?}", cells);
    assert!(close(cells[1], 194.0), "{:?}", cells);
    assert!(close(cells[2], 49.0), "{:?}", cells);
    assert!(close(cells[3], 49.0), "{:?}", cells);
    assert!(close(cells[4], 194.0), "{:?}", cells);
}

#[test]
fn padding_and_border_add_to_the_column() {
    let cells = rects(
        "<table style=\"table-layout:fixed;width:300px\"><tr><td style=\"width:50px;padding:5px\">a</td><td>b</td></tr></table>",
        "td",
        400.0,
    );
    assert!(close(cells[0].width, 60.0), "{:?}", cells);
}

#[test]
fn all_fixed_columns_fill_the_table() {
    let cells = widths(
        "<table style=\"table-layout:fixed;width:200px\"><tr><td style=\"width:50px\">a</td><td style=\"width:50px\">b</td></tr></table>",
    );
    assert!(
        close(cells[0], 97.0) && close(cells[1], 97.0),
        "{:?}",
        cells
    );
}

#[test]
fn auto_width_falls_back_to_automatic_layout() {
    let fixed = widths(
        "<table style=\"table-layout:fixed\"><tr><td>a</td><td>a much longer cell</td></tr></table>",
    );
    let auto = widths("<table><tr><td>a</td><td>a much longer cell</td></tr></table>");
    assert_eq!(fixed, auto);
}

#[test]
fn automatic_layout_is_the_default() {
    let cells = widths(
        "<table style=\"width:300px\"><tr><td>a</td><td>a much longer cell</td></tr></table>",
    );
    assert!(cells[0] < cells[1], "{:?}", cells);
}
