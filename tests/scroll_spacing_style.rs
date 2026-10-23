// SPDX-License-Identifier: GPL-3.0-only

use foundation::css;
use foundation::{html, style};

fn values(declarations: &str) -> ([f32; 4], [f32; 4]) {
    let root = html::parse(&format!(
        "<html><body><div style=\"font-size: 10px; {}\"></div></body></html>",
        declarations
    ));
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(100.0, 100.0)).unwrap();
    let div = &styled.children[0].children[0];
    (div.scroll_padding((200.0, 100.0)), div.scroll_margin())
}

fn names(declarations: &str) -> Vec<String> {
    css::parse(&format!("a {{ {} }}", declarations)).rules[0]
        .declarations
        .iter()
        .map(|declaration| declaration.name.clone())
        .collect()
}

#[test]
fn padding_shorthand_expands_like_padding() {
    assert_eq!(values("scroll-padding: 5px").0, [5.0; 4]);
    assert_eq!(values("scroll-padding: 1px 2px").0, [1.0, 2.0, 1.0, 2.0]);
    assert_eq!(
        values("scroll-padding: 1px 2px 3px").0,
        [1.0, 2.0, 3.0, 2.0]
    );
    assert_eq!(
        values("scroll-padding: 1px 2px 3px 4px").0,
        [1.0, 2.0, 3.0, 4.0]
    );
}

#[test]
fn margin_shorthand_expands_like_margin() {
    assert_eq!(values("scroll-margin: 5px").1, [5.0; 4]);
    assert_eq!(
        values("scroll-margin: 1px 2px 3px 4px").1,
        [1.0, 2.0, 3.0, 4.0]
    );
    assert_eq!(values("scroll-margin: -4px").1, [-4.0; 4]);
}

#[test]
fn longhands_work_alone() {
    assert_eq!(values("scroll-padding-left: 7px").0, [0.0, 0.0, 0.0, 7.0]);
    assert_eq!(
        values("scroll-margin-top: 3px; scroll-margin-bottom: 4px").1,
        [3.0, 0.0, 4.0, 0.0]
    );
}

#[test]
fn padding_percentages_use_the_port() {
    assert_eq!(values("scroll-padding: 10%").0, [10.0, 20.0, 10.0, 20.0]);
}

#[test]
fn relative_units_resolve() {
    assert_eq!(values("scroll-padding: 2em").0, [20.0; 4]);
    assert_eq!(values("scroll-margin: 1em").1, [10.0; 4]);
}

#[test]
fn auto_padding_counts_as_zero() {
    assert_eq!(values("scroll-padding: auto").0, [0.0; 4]);
}

#[test]
fn the_defaults_are_zero() {
    assert_eq!(values(""), ([0.0; 4], [0.0; 4]));
}

#[test]
fn shorthands_become_four_longhands() {
    assert_eq!(
        names("scroll-padding: 1px"),
        vec![
            "scroll-padding-top",
            "scroll-padding-right",
            "scroll-padding-bottom",
            "scroll-padding-left"
        ]
    );
    assert_eq!(names("scroll-margin: 1px 2px").len(), 4);
}

#[test]
fn invalid_shorthands_are_dropped() {
    assert!(names("scroll-padding: 1px 2px 3px 4px 5px").is_empty());
    assert!(names("scroll-margin: ???").is_empty());
}
