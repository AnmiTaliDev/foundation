// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Value};
use foundation::snap::{SnapAlign, Strictness};
use foundation::{html, style};

fn keyword(declarations: &str, name: &str) -> Option<String> {
    let sheet = css::parse(&format!("a {{ {} }}", declarations));
    sheet.rules[0]
        .declarations
        .iter()
        .find(|declaration| declaration.name == name)
        .and_then(|declaration| match &declaration.value {
            Value::Keyword(keyword) => Some(keyword.clone()),
            _ => None,
        })
}

type Snap = (
    (Option<Strictness>, Option<Strictness>),
    (SnapAlign, SnapAlign),
    bool,
);

fn node(declarations: &str) -> Snap {
    let root = html::parse(&format!(
        "<html><body><div style=\"{}\"></div></body></html>",
        declarations
    ));
    let sheet = style::stylesheet_for_document(&root);
    let styled = style::style_tree(&root, &sheet, style::Viewport::new(100.0, 100.0)).unwrap();
    let div = &styled.children[0].children[0];
    (
        div.scroll_snap_type(),
        div.scroll_snap_align(),
        div.scroll_snap_stop_always(),
    )
}

#[test]
fn type_is_normalized() {
    let get = |text: &str| keyword(&format!("scroll-snap-type: {}", text), "scroll-snap-type");
    assert_eq!(get("none").as_deref(), Some("none"));
    assert_eq!(get("x mandatory").as_deref(), Some("x mandatory"));
    assert_eq!(get("y proximity").as_deref(), Some("y proximity"));
    assert_eq!(get("both").as_deref(), Some("both proximity"));
    assert_eq!(get("inline mandatory").as_deref(), Some("x mandatory"));
    assert_eq!(get("block").as_deref(), Some("y proximity"));
    assert_eq!(get("X MANDATORY").as_deref(), Some("x mandatory"));
}

#[test]
fn invalid_types_are_dropped() {
    let get = |text: &str| keyword(&format!("scroll-snap-type: {}", text), "scroll-snap-type");
    assert_eq!(get("mandatory"), None);
    assert_eq!(get("x sometimes"), None);
    assert_eq!(get("x mandatory extra"), None);
    assert_eq!(get("diagonal"), None);
}

#[test]
fn align_takes_one_or_two_values() {
    let get = |text: &str| keyword(&format!("scroll-snap-align: {}", text), "scroll-snap-align");
    assert_eq!(get("start").as_deref(), Some("start start"));
    assert_eq!(get("none").as_deref(), Some("none none"));
    assert_eq!(get("center end").as_deref(), Some("center end"));
    assert_eq!(get("middle"), None);
    assert_eq!(get("start start start"), None);
}

#[test]
fn stop_accepts_normal_and_always() {
    let get = |text: &str| keyword(&format!("scroll-snap-stop: {}", text), "scroll-snap-stop");
    assert_eq!(get("always").as_deref(), Some("always"));
    assert_eq!(get("normal").as_deref(), Some("normal"));
    assert_eq!(get("sometimes"), None);
}

#[test]
fn the_type_maps_to_axes() {
    let kind = |text: &str| node(&format!("scroll-snap-type: {}", text)).0;
    assert_eq!(kind("none"), (None, None));
    assert_eq!(kind("x mandatory"), (Some(Strictness::Mandatory), None));
    assert_eq!(kind("y proximity"), (None, Some(Strictness::Proximity)));
    assert_eq!(
        kind("both mandatory"),
        (Some(Strictness::Mandatory), Some(Strictness::Mandatory))
    );
    assert_eq!(node("").0, (None, None));
}

#[test]
fn align_maps_block_to_y_and_inline_to_x() {
    let align = |text: &str| node(&format!("scroll-snap-align: {}", text)).1;
    assert_eq!(align("start"), (SnapAlign::Start, SnapAlign::Start));
    assert_eq!(align("center end"), (SnapAlign::End, SnapAlign::Center));
    assert_eq!(node("").1, (SnapAlign::None, SnapAlign::None));
}

#[test]
fn stop_maps_to_a_flag() {
    assert!(node("scroll-snap-stop: always").2);
    assert!(!node("scroll-snap-stop: normal").2);
    assert!(!node("").2);
}
