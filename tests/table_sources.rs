// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn source(table: &str) -> String {
    format!(
        "<html><head><style>body {{ margin: 0 }} table {{ border-collapse: collapse }} td {{ padding: 0; width: 10px; height: 10px }}</style></head><body>{}</body></html>",
        table
    )
}

fn cells(table: &str) -> Vec<Rect> {
    fn walk(layout_box: &LayoutBox, found: &mut Vec<Rect>) {
        if let BoxKind::Block(style) = &layout_box.kind {
            if matches!(style.content, Content::Element(e) if e.tag_name == "td") {
                found.push(layout_box.dimensions.border_box());
            }
        }
        for child in &layout_box.children {
            walk(child, found);
        }
    }
    let root = html::parse(&source(table));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(200.0, 200.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    walk(&layout_root, &mut found);
    found
}

fn render(table: &str) -> foundation::paint::Canvas {
    Page::new(&source(table)).render(100, 100, 0.0).canvas
}

fn rows(markup: &str) -> String {
    format!("<table>{}</table>", markup)
}

#[test]
fn a_row_border_sits_between_rows() {
    let table =
        rows("<tr style=\"border-bottom: 3px solid blue\"><td></td></tr><tr><td></td></tr>");
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    for offset in 1..=3 {
        assert_eq!(canvas.pixel(5, boundary - offset), BLUE);
    }
    assert_eq!(canvas.pixel(5, boundary - 4), WHITE);
    assert_eq!(canvas.pixel(5, boundary), WHITE);
}

#[test]
fn a_row_top_border_applies_from_the_next_row_too() {
    let table = rows("<tr><td></td></tr><tr style=\"border-top: 3px solid blue\"><td></td></tr>");
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, boundary - 1), BLUE);
    assert_eq!(canvas.pixel(5, boundary - 3), BLUE);
}

#[test]
fn the_first_row_top_border_is_drawn() {
    let table = rows("<tr style=\"border-top: 2px solid blue\"><td></td></tr>");
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, 0), BLUE);
    assert_eq!(canvas.pixel(5, 1), BLUE);
    assert_eq!(canvas.pixel(5, 3), WHITE);
}

#[test]
fn row_side_borders_apply_to_the_end_cells() {
    let table = rows(
        "<tr style=\"border-left: 2px solid blue; border-right: 2px solid red\"><td></td><td></td></tr>",
    );
    let found = cells(&table);
    let canvas = render(&table);
    assert_eq!(canvas.pixel(0, 5), BLUE);
    assert_eq!(canvas.pixel(1, 5), BLUE);
    assert_eq!(canvas.pixel(2, 5), WHITE);
    let edge = (found[1].x + found[1].width).round() as usize;
    assert_eq!(canvas.pixel(edge - 1, 5), RED);
    assert_eq!(canvas.pixel(edge - 2, 5), RED);
    let middle = found[1].x.round() as usize;
    assert_eq!(canvas.pixel(middle - 1, 5), WHITE);
}

#[test]
fn a_row_group_border_sits_between_groups() {
    let table = rows(
        "<tbody style=\"border-bottom: 2px solid blue\"><tr><td></td></tr></tbody><tbody><tr><td></td></tr></tbody>",
    );
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, boundary - 1), BLUE);
    assert_eq!(canvas.pixel(5, boundary - 2), BLUE);
    assert_eq!(canvas.pixel(5, boundary - 3), WHITE);
}

#[test]
fn a_row_group_border_does_not_apply_between_its_rows() {
    let table = rows(
        "<tbody style=\"border-bottom: 2px solid blue\"><tr><td></td></tr><tr><td></td></tr></tbody>",
    );
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, boundary - 1), WHITE);
    let end = (cells(&table)[1].y + cells(&table)[1].height).round() as usize;
    assert_eq!(canvas.pixel(5, end - 1), BLUE);
}

#[test]
fn a_column_border_sits_between_columns() {
    let table = "<table><col><col style=\"border-left: 3px solid blue\"><tr><td></td><td></td></tr></table>";
    let boundary = cells(table)[1].x.round() as usize;
    let canvas = render(table);
    for offset in 1..=3 {
        assert_eq!(canvas.pixel(boundary - offset, 5), BLUE);
    }
}

#[test]
fn a_column_group_border_sits_between_groups() {
    let table = "<table><colgroup><col></colgroup><colgroup style=\"border-left: 2px solid blue\"><col></colgroup><tr><td></td><td></td></tr></table>";
    let boundary = cells(table)[1].x.round() as usize;
    let canvas = render(table);
    assert_eq!(canvas.pixel(boundary - 1, 5), BLUE);
    assert_eq!(canvas.pixel(boundary - 2, 5), BLUE);
}

#[test]
fn a_column_border_applies_to_the_table_edge() {
    let table =
        "<table><col style=\"border-left: 2px solid blue\"><tr><td></td><td></td></tr></table>";
    let canvas = render(table);
    assert_eq!(canvas.pixel(0, 5), BLUE);
    assert_eq!(canvas.pixel(1, 5), BLUE);
}

#[test]
fn the_cell_wins_a_full_tie_against_its_row() {
    let table = rows(
        "<tr style=\"border-bottom: 2px solid blue\"><td style=\"border-bottom: 2px solid red\"></td></tr><tr><td></td></tr>",
    );
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, boundary - 1), RED);
    assert_eq!(canvas.pixel(5, boundary - 2), RED);
}

#[test]
fn the_row_wins_a_tie_against_its_group() {
    let table = rows(
        "<tbody style=\"border-bottom: 2px solid blue\"><tr style=\"border-bottom: 2px solid red\"><td></td></tr></tbody><tbody><tr><td></td></tr></tbody>",
    );
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, boundary - 1), RED);
}

#[test]
fn a_wider_row_border_beats_a_cell() {
    let table = rows(
        "<tr style=\"border-bottom: 3px solid blue\"><td style=\"border-bottom: 1px solid red\"></td></tr><tr><td></td></tr>",
    );
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, boundary - 1), BLUE);
    assert_eq!(canvas.pixel(5, boundary - 3), BLUE);
}

#[test]
fn a_hidden_row_border_removes_the_cell_border() {
    let table = rows(
        "<tr style=\"border-bottom: hidden\"><td style=\"border-bottom: 3px solid red\"></td></tr><tr><td></td></tr>",
    );
    let found = cells(&table);
    let canvas = render(&table);
    assert_eq!(canvas.pixel(5, found[1].y.round() as usize - 1), WHITE);
    assert_eq!(found[0].height, 10.0);
}

#[test]
fn the_table_border_still_wins_the_outer_edge() {
    let table = "<table style=\"border: 1px solid red\"><tr style=\"border-top: 3px solid blue\"><td></td></tr></table>";
    let canvas = render(table);
    assert_eq!(canvas.pixel(5, 0), RED);
    assert_eq!(canvas.pixel(5, 1), WHITE);
}

#[test]
fn separate_tables_ignore_row_borders() {
    let with_border = "<table style=\"border-collapse: separate\"><tr style=\"border-bottom: 3px solid blue\"><td></td></tr><tr><td></td></tr></table>";
    let without =
        "<table style=\"border-collapse: separate\"><tr><td></td></tr><tr><td></td></tr></table>";
    assert_eq!(cells(with_border)[1].y, cells(without)[1].y);
}

#[test]
fn row_borders_are_not_drawn_twice() {
    let table = rows("<tr style=\"border-left: 2px solid blue\"><td></td></tr>");
    let canvas = render(&table);
    assert_eq!(canvas.pixel(0, 5), BLUE);
    assert_eq!(canvas.pixel(1, 5), BLUE);
    assert_eq!(canvas.pixel(2, 5), WHITE);
}
