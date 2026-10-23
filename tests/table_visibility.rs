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

fn cells(table: &str) -> Vec<Rect> {
    rects(table, "td", 400.0)
}

#[test]
fn a_collapsed_row_takes_no_height() {
    let table = "<table><tr><td style=\"height: 10px\">a</td></tr><tr style=\"visibility: collapse\"><td style=\"height: 30px\">b</td></tr><tr><td style=\"height: 10px\">c</td></tr></table>";
    let found = cells(table);
    let rows = rects(table, "tr", 400.0);
    assert!(close(rows[1].height, 0.0), "{:?}", rows);
    assert!(
        close(found[2].y, found[0].y + found[0].height + 2.0),
        "{:?}",
        found
    );
}

#[test]
fn a_collapsed_row_removes_its_spacing() {
    let with = "<table style=\"border-spacing: 6px\"><tr><td>a</td></tr><tr style=\"visibility: collapse\"><td>b</td></tr><tr><td>c</td></tr></table>";
    let without =
        "<table style=\"border-spacing: 6px\"><tr><td>a</td></tr><tr><td>c</td></tr></table>";
    let a = rects(with, "table", 400.0);
    let b = rects(without, "table", 400.0);
    assert!(close(a[0].height, b[0].height), "{:?} {:?}", a, b);
    assert!(close(cells(with)[2].y, cells(without)[1].y));
}

#[test]
fn a_collapsed_row_group_hides_all_its_rows() {
    let with = "<table><tbody style=\"visibility: collapse\"><tr><td>a</td></tr><tr><td>b</td></tr></tbody><tbody><tr><td>c</td></tr></tbody></table>";
    let without = "<table><tbody><tr><td>c</td></tr></tbody></table>";
    assert!(close(
        rects(with, "table", 400.0)[0].height,
        rects(without, "table", 400.0)[0].height
    ));
}

#[test]
fn a_collapsed_column_takes_no_width() {
    let table = "<table><col><col style=\"visibility: collapse\"><col><tr><td style=\"width: 10px\">a</td><td style=\"width: 40px\">b</td><td style=\"width: 10px\">c</td></tr></table>";
    let without =
        "<table><tr><td style=\"width: 10px\">a</td><td style=\"width: 10px\">c</td></tr></table>";
    let a = rects(table, "table", 400.0);
    let b = rects(without, "table", 400.0);
    assert!(close(a[0].width, b[0].width), "{:?} {:?}", a, b);
    let found = cells(table);
    assert!(close(found[2].x, cells(without)[1].x));
}

#[test]
fn a_collapsed_column_group_hides_its_columns() {
    let table = "<table><colgroup style=\"visibility: collapse\"><col><col></colgroup><col><tr><td style=\"width: 30px\">a</td><td style=\"width: 30px\">b</td><td style=\"width: 10px\">c</td></tr></table>";
    let without = "<table><tr><td style=\"width: 10px\">c</td></tr></table>";
    assert!(close(
        rects(table, "table", 400.0)[0].width,
        rects(without, "table", 400.0)[0].width
    ));
}

#[test]
fn collapsed_cells_are_not_painted() {
    let source = |style: &str| {
        format!(
            "<html><head><style>body {{ margin: 0 }} td {{ padding: 0; width: 10px; height: 10px; background: red }}</style></head><body><table style=\"border-spacing: 0\"><col><col style=\"{}\"><tr><td></td><td>x</td></tr></table></body></html>",
            style
        )
    };
    let canvas = foundation::Page::new(&source("visibility: collapse"))
        .render(60, 30, 0.0)
        .canvas;
    assert_eq!(
        canvas.pixel(5, 5),
        foundation::css::Color { r: 255, g: 0, b: 0 }
    );
    for x in 10..30 {
        assert_eq!(
            canvas.pixel(x, 5),
            foundation::css::Color {
                r: 255,
                g: 255,
                b: 255
            },
            "x {}",
            x
        );
    }
}

#[test]
fn collapsed_rows_do_not_paint_their_content() {
    let source = "<html><head><style>body { margin: 0 } td { padding: 0 }</style></head><body><table style=\"border-spacing: 0\"><tr style=\"visibility: collapse\"><td style=\"width: 20px; height: 20px; background: red\">text</td></tr></table></body></html>";
    let canvas = foundation::Page::new(source).render(60, 30, 0.0).canvas;
    for y in 0..30 {
        for x in 0..60 {
            assert_eq!(
                canvas.pixel(x, y),
                foundation::css::Color {
                    r: 255,
                    g: 255,
                    b: 255
                }
            );
        }
    }
}

#[test]
fn collapsed_cells_do_not_affect_the_row_height() {
    let table = "<table><col style=\"visibility: collapse\"><tr><td style=\"height: 50px\">a</td><td style=\"height: 25px\">b</td></tr></table>";
    let rows = rects(table, "tr", 400.0);
    assert!(close(rows[0].height, 25.0), "{:?}", rows);
}

#[test]
fn hidden_rows_keep_their_space() {
    let table = "<table><tr><td style=\"height: 10px\">a</td></tr><tr style=\"visibility: hidden\"><td style=\"height: 30px\">b</td></tr><tr><td style=\"height: 10px\">c</td></tr></table>";
    let rows = rects(table, "tr", 400.0);
    assert!(close(rows[1].height, 30.0), "{:?}", rows);
}

#[test]
fn visible_rows_in_a_table_without_collapse_are_unchanged() {
    let table = "<table><tr><td style=\"height: 20px\">a</td></tr><tr><td style=\"height: 30px\">b</td></tr></table>";
    let rows = rects(table, "tr", 400.0);
    assert!(close(rows[0].height, 20.0));
    assert!(close(rows[1].height, 30.0));
}
