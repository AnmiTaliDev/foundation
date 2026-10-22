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

fn row(left: &str, right: &str) -> String {
    format!(
        "<table><tr><td style=\"{}\"></td><td style=\"{}\"></td></tr></table>",
        left, right
    )
}

fn column(top: &str, bottom: &str) -> String {
    format!(
        "<table><tr><td style=\"{}\"></td></tr><tr><td style=\"{}\"></td></tr></table>",
        top, bottom
    )
}

#[test]
fn a_wider_right_neighbor_wins() {
    let table = row("border-right: 1px solid red", "border-left: 3px solid blue");
    let boundary = cells(&table)[1].x.round() as usize;
    let canvas = render(&table);
    for offset in 1..=3 {
        assert_eq!(canvas.pixel(boundary - offset, 5), BLUE);
    }
    assert_eq!(canvas.pixel(boundary - 4, 5), WHITE);
    assert_eq!(canvas.pixel(boundary, 5), WHITE);
}

#[test]
fn a_wider_left_cell_wins() {
    let table = row("border-right: 3px solid red", "border-left: 1px solid blue");
    let boundary = cells(&table)[1].x.round() as usize;
    let canvas = render(&table);
    for offset in 1..=3 {
        assert_eq!(canvas.pixel(boundary - offset, 5), RED);
    }
}

#[test]
fn the_left_cell_wins_a_tie() {
    let table = row("border-right: 2px solid red", "border-left: 2px solid blue");
    let boundary = cells(&table)[1].x.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(boundary - 1, 5), RED);
    assert_eq!(canvas.pixel(boundary - 2, 5), RED);
}

#[test]
fn a_better_style_wins_at_equal_width() {
    let table = row(
        "border-right: 2px dotted red",
        "border-left: 2px solid blue",
    );
    let boundary = cells(&table)[1].x.round() as usize;
    let canvas = render(&table);
    for y in 0..10 {
        assert_eq!(canvas.pixel(boundary - 1, y), BLUE, "row {}", y);
    }
}

#[test]
fn a_none_neighbor_does_not_hide_the_other_side() {
    let table = row("border-right: none", "border-left: 2px solid blue");
    let boundary = cells(&table)[1].x.round() as usize;
    let canvas = render(&table);
    assert_eq!(canvas.pixel(boundary - 1, 5), BLUE);
    assert_eq!(canvas.pixel(boundary - 2, 5), BLUE);
}

#[test]
fn hidden_removes_the_shared_border() {
    let table = row("border-right: 2px solid red", "border-left: 1px hidden");
    let found = cells(&table);
    let canvas = render(&table);
    let boundary = found[1].x.round() as usize;
    assert_eq!(canvas.pixel(boundary - 1, 5), WHITE);
    assert_eq!(found[0].width, 10.0);
}

#[test]
fn the_bottom_edge_is_resolved_too() {
    let table = column("border-bottom: 1px solid red", "border-top: 3px solid blue");
    let boundary = cells(&table)[1].y.round() as usize;
    let canvas = render(&table);
    for offset in 1..=3 {
        assert_eq!(canvas.pixel(5, boundary - offset), BLUE);
    }
    assert_eq!(canvas.pixel(5, boundary), WHITE);
}

#[test]
fn rowspan_neighbors_are_matched() {
    let table = "<table><tr><td rowspan=2 style=\"border-right: 1px solid red\"></td><td style=\"border-left: 3px solid blue\"></td></tr><tr><td></td></tr></table>";
    let found = cells(table);
    let boundary = found[1].x.round() as usize;
    let canvas = render(table);
    assert_eq!(canvas.pixel(boundary - 1, 5), BLUE);
    assert_eq!(canvas.pixel(boundary - 3, 5), BLUE);
}

#[test]
fn separate_borders_are_not_resolved() {
    let table = "<table style=\"border-collapse: separate\"><tr><td style=\"border-right: 1px solid red\"></td><td style=\"border-left: 3px solid blue\"></td></tr></table>";
    let found = cells(table);
    assert_eq!(found[0].width, 11.0);
    assert_eq!(found[1].width, 13.0);
}
