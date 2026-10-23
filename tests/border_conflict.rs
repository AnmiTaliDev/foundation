// SPDX-License-Identifier: GPL-3.0-only

use foundation::collapse::Edge;
use foundation::css::Color;
use foundation::{html, style};

fn edge(declarations: &str, side: usize) -> Edge {
    let root = html::parse(&format!(
        "<html><body><div style=\"{}\"></div></body></html>",
        declarations
    ));
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(100.0, 100.0)).unwrap();
    styled.children[0].children[0].border_edge(side)
}

fn right(declarations: &str) -> Edge {
    edge(declarations, 1)
}

#[test]
fn an_edge_reads_width_style_and_color() {
    let found = right("border-right: 3px dashed #ff0000");
    assert_eq!(found.width, 3.0);
    assert_eq!(found.rank, 6);
    assert!(!found.hidden);
    assert_eq!(
        found.color.map(|c| c.color),
        Some(Color { r: 255, g: 0, b: 0 })
    );
}

#[test]
fn none_has_no_width_and_the_lowest_rank() {
    let found = right("border-right: 3px none red");
    assert_eq!(found.width, 0.0);
    assert_eq!(found.rank, 0);
    assert!(!found.hidden);
}

#[test]
fn hidden_is_marked() {
    assert!(right("border-right: 3px hidden red").hidden);
}

#[test]
fn style_ranks_follow_the_specification_order() {
    let styles = [
        "double", "solid", "dashed", "dotted", "ridge", "outset", "groove", "inset",
    ];
    let ranks: Vec<u8> = styles
        .iter()
        .map(|name| right(&format!("border-right: 2px {} black", name)).rank)
        .collect();
    assert!(
        ranks.windows(2).all(|pair| pair[0] > pair[1]),
        "{:?}",
        ranks
    );
    assert!(ranks.iter().all(|rank| *rank > 0));
}

#[test]
fn the_wider_border_wins() {
    let thin = right("border-right: 1px solid red");
    let wide = right("border-right: 3px dotted blue");
    assert_eq!(Edge::resolve(thin, wide), wide);
    assert_eq!(Edge::resolve(wide, thin), wide);
}

#[test]
fn equal_widths_compare_styles() {
    let solid = right("border-right: 2px solid red");
    let double = right("border-right: 2px double blue");
    let dashed = right("border-right: 2px dashed lime");
    assert_eq!(Edge::resolve(solid, double), double);
    assert_eq!(Edge::resolve(dashed, solid), solid);
}

#[test]
fn the_first_edge_wins_a_tie() {
    let first = right("border-right: 2px solid red");
    let second = right("border-right: 2px solid blue");
    assert_eq!(Edge::resolve(first, second), first);
    assert_eq!(Edge::resolve(second, first), second);
}

#[test]
fn hidden_removes_the_border() {
    let line = right("border-right: 3px solid red");
    let hidden = right("border-right: 1px hidden");
    assert_eq!(Edge::resolve(line, hidden), Edge::NONE);
    assert_eq!(Edge::resolve(hidden, line), Edge::NONE);
}

#[test]
fn none_loses_to_any_line() {
    let line = right("border-right: 1px solid red");
    let none = right("border-right: 5px none");
    assert_eq!(Edge::resolve(none, line), line);
    assert_eq!(Edge::resolve(line, none), line);
    assert_eq!(Edge::resolve(none, none).width, 0.0);
}

#[test]
fn every_side_is_read() {
    let sides: Vec<f32> = (0..4)
        .map(|side| {
            edge(
                "border-top: 1px solid red; border-right: 2px solid red; border-bottom: 3px solid red; border-left: 4px solid red",
                side,
            )
            .width
        })
        .collect();
    assert_eq!(sides, vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn a_higher_source_priority_wins_a_full_tie() {
    let cell = right("border-right: 2px solid red").from_source(5);
    let row = right("border-right: 2px solid blue").from_source(4);
    assert_eq!(Edge::resolve(row, cell), cell);
    assert_eq!(Edge::resolve(cell, row), cell);
}

#[test]
fn priority_does_not_beat_width_or_style() {
    let cell = right("border-right: 1px solid red").from_source(5);
    let row = right("border-right: 2px solid blue").from_source(4);
    assert_eq!(Edge::resolve(cell, row), row);
    let dashed = right("border-right: 2px dashed red").from_source(5);
    assert_eq!(Edge::resolve(dashed, row), row);
}

#[test]
fn hidden_anywhere_in_a_list_removes_the_border() {
    let line = right("border-right: 3px solid red");
    let hidden = right("border-right: 1px hidden");
    assert_eq!(Edge::resolve_all(&[line, line, hidden, line]), Edge::NONE);
    assert_eq!(Edge::resolve_all(&[line, hidden, line]), Edge::NONE);
}

#[test]
fn an_empty_list_has_no_border() {
    assert_eq!(Edge::resolve_all(&[]), Edge::NONE);
}

#[test]
fn the_earliest_edge_wins_among_equals_in_a_list() {
    let first = right("border-right: 2px solid red");
    let second = right("border-right: 2px solid blue");
    let third = right("border-right: 2px solid lime");
    assert_eq!(Edge::resolve_all(&[first, second, third]), first);
}
