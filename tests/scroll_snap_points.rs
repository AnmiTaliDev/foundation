// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout;
use foundation::scroll::{Offsets, Scroller};
use foundation::{html, style, text};

fn scrollers(container: &str, children: &str, offset: (f32, f32)) -> Vec<Scroller> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 100px; height: 100px; overflow: auto; {} }} .c {{ width: 100px; height: 100px }} .row {{ display: flex; width: 400px }} .row .c {{ flex: none }}</style></head><body><div id=a>{}</div></body></html>",
        container, children
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(300.0, 300.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let mut layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut offsets = Offsets::new();
    offsets.insert(0, offset);
    layout_root.apply_scrolls(&offsets, &fonts)
}

fn positions(points: &[foundation::snap::SnapPoint]) -> Vec<f32> {
    points.iter().map(|point| point.position).collect()
}

const THREE: &str = "<div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div>";

#[test]
fn start_aligned_children_give_their_offsets() {
    let found = scrollers("scroll-snap-type: y mandatory", THREE, (0.0, 0.0));
    assert_eq!(positions(&found[0].points.1), vec![0.0, 100.0, 200.0]);
    assert!(found[0].points.0.is_empty());
}

#[test]
fn points_do_not_depend_on_the_current_offset() {
    let found = scrollers("scroll-snap-type: y mandatory", THREE, (0.0, 150.0));
    assert_eq!(found[0].offset, (0.0, 150.0));
    assert_eq!(positions(&found[0].points.1), vec![0.0, 100.0, 200.0]);
}

#[test]
fn center_and_end_alignment_use_the_port() {
    let children = "<div class=c style=\"height: 40px; scroll-snap-align: center\"></div><div class=c style=\"height: 40px; scroll-snap-align: center\"></div><div class=c style=\"height: 40px; scroll-snap-align: end\"></div><div class=c style=\"height: 200px\"></div>";
    let found = scrollers("scroll-snap-type: y mandatory", children, (0.0, 0.0));
    assert_eq!(found[0].max.1, 320.0 - 100.0);
    assert_eq!(positions(&found[0].points.1), vec![0.0, 10.0, 20.0]);
}

#[test]
fn positions_are_clamped_to_the_scrollable_range() {
    let children = "<div class=c style=\"scroll-snap-align: end\"></div><div class=c style=\"scroll-snap-align: start\"></div>";
    let found = scrollers("scroll-snap-type: y mandatory", children, (0.0, 0.0));
    assert_eq!(found[0].max.1, 100.0);
    assert_eq!(positions(&found[0].points.1), vec![0.0, 100.0]);
}

#[test]
fn the_inline_axis_uses_x() {
    let children = "<div class=row><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div></div>";
    let found = scrollers("scroll-snap-type: x mandatory", children, (0.0, 0.0));
    assert_eq!(
        positions(&found[0].points.0),
        vec![0.0, 100.0, 200.0, 300.0]
    );
    assert!(found[0].points.1.is_empty());
}

#[test]
fn both_axes_are_collected() {
    let children = "<div class=row><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div></div><div class=row><div class=c style=\"scroll-snap-align: start\"></div></div>";
    let found = scrollers("scroll-snap-type: both mandatory", children, (0.0, 0.0));
    assert_eq!(
        positions(&found[0].points.0),
        vec![0.0, 100.0, 200.0, 300.0]
    );
    assert_eq!(positions(&found[0].points.1), vec![0.0, 100.0]);
}

#[test]
fn containers_without_a_snap_type_have_no_points() {
    let found = scrollers("", THREE, (0.0, 0.0));
    assert_eq!(found[0].snap, (None, None));
    assert!(found[0].points.1.is_empty());
}

#[test]
fn the_stop_flag_is_kept_and_merged_at_equal_positions() {
    let children = "<div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start; scroll-snap-stop: always\"></div><div class=c style=\"scroll-snap-align: start\"></div>";
    let found = scrollers("scroll-snap-type: y mandatory", children, (0.0, 0.0));
    let always: Vec<bool> = found[0].points.1.iter().map(|point| point.always).collect();
    assert_eq!(always, vec![false, true, false]);
    let merged = "<div class=c style=\"height: 0; scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start; scroll-snap-stop: always\"></div><div class=c></div>";
    let found = scrollers("scroll-snap-type: y mandatory", merged, (0.0, 0.0));
    assert_eq!(positions(&found[0].points.1), vec![0.0]);
    assert!(found[0].points.1[0].always);
}

#[test]
fn nested_scroll_containers_keep_their_own_children() {
    let children = "<div class=c style=\"scroll-snap-align: start; overflow: auto\"><div style=\"height: 300px\"><div class=c style=\"scroll-snap-align: start; margin-top: 50px\"></div></div></div><div class=c style=\"scroll-snap-align: start\"></div>";
    let found = scrollers("scroll-snap-type: y mandatory", children, (0.0, 0.0));
    assert_eq!(positions(&found[0].points.1), vec![0.0, 100.0]);
}

#[test]
fn descendants_below_non_container_wrappers_count() {
    let children = "<div><div class=c style=\"scroll-snap-align: start\"></div><div class=c style=\"scroll-snap-align: start\"></div></div>";
    let found = scrollers("scroll-snap-type: y mandatory", children, (0.0, 0.0));
    assert_eq!(positions(&found[0].points.1), vec![0.0, 100.0]);
}
