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

fn cell_rects(table: &str) -> Vec<Rect> {
    rects(table, "td", 400.0)
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

fn second_baseline(align_row: &str, align_group: &str, align_cell: &str) -> f32 {
    let table = format!(
        "<table style=\"width:200px\"><tbody style=\"{}\"><tr style=\"{}\"><td style=\"height: 60px\">a</td><td style=\"{}\">b</td></tr></tbody></table>",
        align_group, align_row, align_cell
    );
    baselines(&table)[1]
}

#[test]
fn cells_are_centered_by_default() {
    let middle = second_baseline("", "", "");
    let explicit = second_baseline("", "", "vertical-align: middle");
    assert!(close(middle, explicit));
    let top = second_baseline("", "", "vertical-align: top");
    assert!(middle > top);
}

#[test]
fn a_row_sets_the_alignment_of_its_cells() {
    let top = second_baseline("", "", "vertical-align: top");
    let bottom = second_baseline("", "", "vertical-align: bottom");
    assert!(close(second_baseline("vertical-align: top", "", ""), top));
    assert!(close(
        second_baseline("vertical-align: bottom", "", ""),
        bottom
    ));
}

#[test]
fn a_row_group_sets_the_alignment_of_its_rows() {
    let bottom = second_baseline("", "", "vertical-align: bottom");
    assert!(close(
        second_baseline("", "vertical-align: bottom", ""),
        bottom
    ));
}

#[test]
fn the_row_beats_the_group_and_the_cell_beats_the_row() {
    let top = second_baseline("", "", "vertical-align: top");
    let bottom = second_baseline("", "", "vertical-align: bottom");
    assert!(close(
        second_baseline("vertical-align: top", "vertical-align: bottom", ""),
        top
    ));
    assert!(close(
        second_baseline(
            "vertical-align: top",
            "vertical-align: top",
            "vertical-align: bottom"
        ),
        bottom
    ));
}

#[test]
fn a_row_can_align_its_cells_to_the_baseline() {
    let table = "<table style=\"width:200px\"><tr style=\"vertical-align: baseline\"><td style=\"font-size: 32px\">A</td><td>b</td></tr></table>";
    let found = baselines(table);
    assert!(close(found[0], found[1]), "{:?}", found);
}

#[test]
fn a_group_can_align_its_cells_to_the_baseline() {
    let table = "<table style=\"width:200px\"><tbody style=\"vertical-align: baseline\"><tr><td style=\"font-size: 32px\">A</td><td>b</td></tr></tbody></table>";
    let found = baselines(table);
    assert!(close(found[0], found[1]), "{:?}", found);
}

#[test]
fn header_cells_follow_the_row_too() {
    let table = "<table style=\"width:200px\"><tr style=\"vertical-align: top\"><th style=\"height: 60px\">a</th><th>b</th></tr></table>";
    let found = baselines(table);
    assert!(close(found[0], found[1]), "{:?}", found);
}

#[test]
fn cell_heights_are_unaffected() {
    let table = "<table><tr style=\"vertical-align: bottom\"><td style=\"height: 60px\">a</td><td>b</td></tr></table>";
    let cells = cell_rects(table);
    assert!(close(cells[0].height, cells[1].height));
}
