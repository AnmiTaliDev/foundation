// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::paint::Canvas;
use foundation::style::Content;
use foundation::{html, style, text, Page};

const BLACK: Color = Color { r: 0, g: 0, b: 0 };
const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn source(table: &str) -> String {
    format!(
        "<html><head><style>body {{ margin: 0; width: 60px }} td {{ padding: 0; width: 10px; height: 10px }}</style></head><body>{}</body></html>",
        table
    )
}

fn find(layout_box: &LayoutBox, tag: &str, found: &mut Vec<Rect>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if matches!(style.content, Content::Element(e) if e.tag_name == tag) {
            found.push(layout_box.dimensions.border_box());
        }
    }
    for child in &layout_box.children {
        find(child, tag, found);
    }
}

fn rects(table: &str, tag: &str) -> Vec<Rect> {
    let root = html::parse(&source(table));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(60.0, 60.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, tag, &mut found);
    found
}

fn render(table: &str) -> Canvas {
    Page::new(&source(table)).render(60, 30, 0.0).canvas
}

fn row_of(canvas: &Canvas, y: usize) -> Vec<Color> {
    (0..canvas.width).map(|x| canvas.pixel(x, y)).collect()
}

fn xs(rects: &[Rect]) -> Vec<f32> {
    rects.iter().map(|rect| rect.x).collect()
}

const THREE: &str = "<tr><td></td><td></td><td></td></tr>";

#[test]
fn columns_run_from_right_to_left() {
    let ltr = rects(
        &format!(
            "<table style=\"width: 60px; border-spacing: 0\">{}</table>",
            THREE
        ),
        "td",
    );
    let rtl = rects(
        &format!(
            "<table style=\"direction: rtl; width: 60px; border-spacing: 0\">{}</table>",
            THREE
        ),
        "td",
    );
    assert_eq!(xs(&ltr), vec![0.0, 20.0, 40.0]);
    assert_eq!(xs(&rtl), vec![40.0, 20.0, 0.0]);
    assert_eq!(ltr[0].width, rtl[0].width);
}

#[test]
fn spacing_is_kept_on_both_sides() {
    let rtl = rects(
        &format!(
            "<table style=\"direction: rtl; width: 60px; border-spacing: 4px\">{}</table>",
            "<tr><td></td><td></td></tr>"
        ),
        "td",
    );
    assert_eq!(rtl[0].x + rtl[0].width, 56.0);
    assert_eq!(rtl[1].x, 4.0);
}

#[test]
fn column_spans_are_mirrored_as_a_unit() {
    let table = "<table style=\"direction: rtl; width: 60px; border-spacing: 0\"><tr><td colspan=2></td><td></td></tr><tr><td></td><td></td><td></td></tr></table>";
    let found = rects(table, "td");
    assert_eq!(found[0].x, 20.0);
    assert_eq!(found[0].width, 40.0);
    assert_eq!(found[1].x, 0.0);
    assert_eq!(xs(&found[2..]), vec![40.0, 20.0, 0.0]);
}

#[test]
fn tables_in_rtl_containers_sit_on_the_right() {
    let table = "<div style=\"direction: rtl\"><table style=\"border-spacing: 0\"><tr><td></td></tr></table></div>";
    let found = rects(table, "td");
    assert_eq!(found[0].x, 50.0);
    let table = "<div><table style=\"border-spacing: 0\"><tr><td></td></tr></table></div>";
    assert_eq!(rects(table, "td")[0].x, 0.0);
}

#[test]
fn collapsed_borders_mirror_with_the_cells() {
    let cells = "<tr><td style=\"border: 1px solid black\"></td><td style=\"border: 1px solid black\"></td><td style=\"border: 1px solid black\"></td></tr>";
    let ltr = render(&format!(
        "<table style=\"width: 60px; border-collapse: collapse\">{}</table>",
        cells
    ));
    let rtl = render(&format!(
        "<table style=\"direction: rtl; width: 60px; border-collapse: collapse\">{}</table>",
        cells
    ));
    let a = row_of(&ltr, 5);
    let b = row_of(&rtl, 5);
    let lines = |row: &Vec<Color>| row.iter().filter(|c| **c == BLACK).count();
    assert_eq!(lines(&a), 4);
    assert_eq!(lines(&b), 4);
    let reversed: Vec<Color> = a.iter().rev().copied().collect();
    let width = a.iter().rposition(|c| *c == BLACK).unwrap() + 1;
    let tail = &b[..width];
    let head = &reversed[reversed.len() - width..];
    assert_eq!(tail.len(), head.len());
    for (x, (left, right)) in tail.iter().zip(head).enumerate() {
        assert!(
            left == right || (*left == WHITE && *right == WHITE) || x + 1 >= width,
            "x {}",
            x
        );
    }
}

#[test]
fn the_shared_border_is_drawn_once_between_mirrored_cells() {
    let table = "<table style=\"direction: rtl; width: 60px; border-collapse: collapse\"><tr><td style=\"border-left: 3px solid red\"></td><td style=\"border-right: 1px solid blue\"></td></tr></table>";
    let found = rects(table, "td");
    let canvas = render(table);
    let boundary = found[0].x.round() as usize;
    assert_eq!(canvas.pixel(boundary, 5), RED);
    assert_eq!(canvas.pixel(boundary + 1, 5), RED);
    assert_eq!(canvas.pixel(boundary + 2, 5), RED);
    assert_eq!(canvas.pixel(boundary - 1, 5), WHITE);
    assert_eq!(canvas.pixel(boundary + 3, 5), WHITE);
}

#[test]
fn outer_borders_of_the_table_stay_on_their_physical_sides() {
    let table = "<table style=\"direction: rtl; width: 60px; border-collapse: collapse; border-left: 2px solid red\"><tr><td style=\"border: 1px solid blue\"></td><td style=\"border: 1px solid blue\"></td></tr></table>";
    let canvas = render(table);
    assert_eq!(canvas.pixel(0, 5), RED);
    assert_eq!(canvas.pixel(1, 5), RED);
    assert_eq!(canvas.pixel(2, 5), WHITE);
}

#[test]
fn outer_cell_borders_appear_on_the_matching_sides() {
    let table = "<table style=\"direction: rtl; width: 60px; border-collapse: collapse\"><tr><td style=\"border-right: 2px solid red\"></td><td style=\"border-left: 2px solid blue\"></td></tr></table>";
    let found = rects(table, "td");
    let canvas = render(table);
    let right_edge = (found[0].x + found[0].width).round() as usize;
    assert_eq!(canvas.pixel(right_edge - 1, 5), RED);
    assert_eq!(canvas.pixel(right_edge - 2, 5), RED);
    let left_edge = found[1].x.round() as usize;
    assert_eq!(canvas.pixel(left_edge, 5), BLUE);
    assert_eq!(canvas.pixel(left_edge + 1, 5), BLUE);
}

#[test]
fn rows_and_captions_are_not_moved() {
    let table = "<table style=\"direction: rtl; width: 60px; border-spacing: 0\"><caption>c</caption><tr><td></td><td></td></tr></table>";
    let rows = rects(table, "tr");
    assert_eq!(rows[0].x, 0.0);
    assert_eq!(rows[0].width, 60.0);
    let captions = rects(table, "caption");
    assert_eq!(captions[0].x, 0.0);
}
