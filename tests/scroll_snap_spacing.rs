// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout;
use foundation::scroll::{Offsets, Scroller};
use foundation::{html, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };

fn scrollers(container: &str, children: &str) -> Vec<Scroller> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 100px; height: 100px; overflow: auto; scroll-snap-type: y mandatory; {} }}</style></head><body><div id=a>{}</div></body></html>",
        container, children
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(300.0, 300.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let mut layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    layout_root.apply_scrolls(&Offsets::new(), &fonts)
}

fn ys(found: &[Scroller]) -> Vec<f32> {
    found[0]
        .points
        .1
        .iter()
        .map(|point| point.position)
        .collect()
}

fn items(style: &str) -> String {
    (0..3)
        .map(|_| {
            format!(
                "<div style=\"height: 100px; scroll-snap-align: {}\"></div>",
                style
            )
        })
        .collect()
}

#[test]
fn padding_moves_start_points() {
    let found = scrollers("scroll-padding: 10px", &items("start"));
    assert_eq!(ys(&found), vec![0.0, 90.0, 190.0]);
}

#[test]
fn padding_moves_end_points() {
    let children = "<div style=\"height: 100px\"></div><div style=\"height: 100px; scroll-snap-align: end\"></div><div style=\"height: 100px\"></div>";
    let plain = scrollers("", children);
    let padded = scrollers("scroll-padding-bottom: 20px", children);
    assert_eq!(ys(&plain), vec![100.0]);
    assert_eq!(ys(&padded), vec![120.0]);
}

#[test]
fn padding_narrows_the_port_for_centering() {
    let children = "<div style=\"height: 40px\"></div><div style=\"height: 40px; scroll-snap-align: center\"></div><div style=\"height: 200px\"></div>";
    let plain = scrollers("", children);
    let padded = scrollers("scroll-padding-bottom: 30px", children);
    assert_eq!(ys(&plain), vec![10.0]);
    assert_eq!(ys(&padded), vec![25.0]);
}

#[test]
fn padding_percentages_use_the_port() {
    let found = scrollers("scroll-padding: 10%", &items("start"));
    assert_eq!(ys(&found), vec![0.0, 90.0, 190.0]);
}

#[test]
fn margin_expands_the_snap_area() {
    let children = "<div style=\"height: 100px\"></div><div style=\"height: 100px; scroll-snap-align: start; scroll-margin-top: 15px\"></div><div style=\"height: 100px\"></div>";
    let found = scrollers("", children);
    assert_eq!(ys(&found), vec![85.0]);
}

#[test]
fn margin_grows_the_area_for_end_alignment() {
    let children = "<div style=\"height: 100px\"></div><div style=\"height: 100px; scroll-snap-align: end; scroll-margin-bottom: 10px\"></div><div style=\"height: 100px\"></div>";
    let found = scrollers("", children);
    assert_eq!(ys(&found), vec![110.0]);
}

#[test]
fn negative_margins_shrink_the_area() {
    let children = "<div style=\"height: 100px\"></div><div style=\"height: 100px; scroll-snap-align: start; scroll-margin-top: -10px\"></div><div style=\"height: 100px\"></div>";
    let found = scrollers("", children);
    assert_eq!(ys(&found), vec![110.0]);
}

#[test]
fn points_are_clamped_after_the_adjustment() {
    let found = scrollers("scroll-padding-top: 50px", &items("start"));
    assert_eq!(ys(&found), vec![0.0, 50.0, 150.0]);
}

#[test]
fn the_padding_of_other_boxes_is_ignored() {
    let children = "<div style=\"scroll-padding: 40px\"><div style=\"height: 100px; scroll-snap-align: start\"></div><div style=\"height: 100px; scroll-snap-align: start\"></div></div>";
    let found = scrollers("", children);
    assert_eq!(ys(&found), vec![0.0, 100.0]);
}

#[test]
fn the_wheel_lands_on_the_adjusted_point() {
    let mut page = Page::new(
        "<html><head><style>body { margin: 0 } #a { width: 100px; height: 100px; overflow: auto; scroll-snap-type: y mandatory; scroll-padding-top: 10px } .i { height: 100px; scroll-snap-align: start }</style></head><body><div id=a><div class=i style=\"background: red\"></div><div class=i style=\"background: blue\"></div><div class=i style=\"background: red\"></div></div></body></html>",
    );
    page.scroll_by((150, 150), 0.0, (5.0, 5.0), (0.0, 40.0));
    let canvas = page.render(150, 150, 0.0).canvas;
    assert_eq!(canvas.pixel(5, 9), RED);
    assert_eq!(canvas.pixel(5, 10), BLUE);
}
