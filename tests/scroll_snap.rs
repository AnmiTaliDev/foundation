// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };

const SIZE: (usize, usize) = (150, 150);

fn bands(attribute: &str) -> String {
    ["red", "blue", "lime"]
        .iter()
        .map(|color| {
            format!(
                "<div style=\"height: 100px; background: {}; scroll-snap-align: start; {}\"></div>",
                color, attribute
            )
        })
        .collect()
}

fn page(container: &str, content: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 100px; height: 100px; overflow: auto; {} }}</style></head><body><div id=a>{}</div></body></html>",
        container, content
    ))
}

fn vertical(container: &str) -> Page {
    page(container, &bands(""))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

fn wheel(page: &mut Page, dx: f32, dy: f32) -> (f32, f32) {
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (dx, dy))
}

#[test]
fn a_wheel_step_moves_to_the_next_snap_point() {
    let mut page = vertical("scroll-snap-type: y mandatory");
    assert_eq!(wheel(&mut page, 0.0, 40.0), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 5, 95), BLUE);
    wheel(&mut page, 0.0, 40.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn the_last_point_passes_the_scroll_on() {
    let mut page = vertical("scroll-snap-type: y mandatory");
    wheel(&mut page, 0.0, 250.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
    assert_eq!(wheel(&mut page, 0.0, 40.0), (0.0, 40.0));
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn scrolling_back_moves_to_the_previous_point() {
    let mut page = vertical("scroll-snap-type: y mandatory");
    wheel(&mut page, 0.0, 250.0);
    wheel(&mut page, 0.0, -40.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    wheel(&mut page, 0.0, -40.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(wheel(&mut page, 0.0, -40.0), (0.0, -40.0));
}

#[test]
fn a_large_step_lands_on_the_nearest_point_to_the_target() {
    let mut page = vertical("scroll-snap-type: y mandatory");
    wheel(&mut page, 0.0, 130.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    let mut page = vertical("scroll-snap-type: y mandatory");
    wheel(&mut page, 0.0, 170.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn proximity_snaps_only_when_close() {
    let mut page = vertical("scroll-snap-type: y proximity");
    wheel(&mut page, 0.0, 40.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 5, 55), RED);
    assert_eq!(pixel(&page, 5, 65), BLUE);
}

#[test]
fn proximity_uses_a_quarter_of_the_port() {
    let mut page = vertical("scroll-snap-type: y proximity");
    wheel(&mut page, 0.0, 80.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    assert_eq!(pixel(&page, 5, 95), BLUE);
}

#[test]
fn always_stops_a_long_scroll() {
    let page = page(
        "scroll-snap-type: y mandatory",
        "<div style=\"height: 100px; background: red; scroll-snap-align: start\"></div><div style=\"height: 100px; background: blue; scroll-snap-align: start; scroll-snap-stop: always\"></div><div style=\"height: 100px; background: lime; scroll-snap-align: start\"></div>",
    );
    let mut page = page;
    wheel(&mut page, 0.0, 250.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    wheel(&mut page, 0.0, 250.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn without_a_snap_type_scrolling_is_free() {
    let mut page = vertical("");
    wheel(&mut page, 0.0, 40.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 5, 70), BLUE);
}

#[test]
fn only_the_snapping_axis_snaps() {
    let mut page = vertical("scroll-snap-type: x mandatory");
    wheel(&mut page, 0.0, 40.0);
    assert_eq!(pixel(&page, 5, 70), BLUE);
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn the_horizontal_axis_snaps_too() {
    let children = "<div style=\"display: flex; width: 300px\"><div style=\"flex: none; width: 100px; height: 80px; background: red; scroll-snap-align: start\"></div><div style=\"flex: none; width: 100px; height: 80px; background: blue; scroll-snap-align: start\"></div><div style=\"flex: none; width: 100px; height: 80px; background: lime; scroll-snap-align: start\"></div></div>";
    let mut page = page("scroll-snap-type: x mandatory", children);
    assert_eq!(wheel(&mut page, 40.0, 0.0), (0.0, 0.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    wheel(&mut page, 40.0, 0.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn dragging_a_thumb_settles_on_the_nearest_point() {
    let mut page = vertical("scroll-snap-type: y mandatory");
    let grab = page.scroll_grab(SIZE, 0.0, (96.0, 5.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (96.0, 32.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
    page.scroll_drag(SIZE, 0.0, grab, (96.0, 500.0));
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn centered_items_snap_to_the_center() {
    let children = "<div style=\"height: 40px; background: red; scroll-snap-align: center\"></div><div style=\"height: 40px; background: blue; scroll-snap-align: center\"></div><div style=\"height: 40px; background: lime; scroll-snap-align: center\"></div><div style=\"height: 200px\"></div>";
    let mut page = page("scroll-snap-type: y mandatory", children);
    wheel(&mut page, 0.0, 5.0);
    assert_eq!(pixel(&page, 5, 40), BLUE);
    assert_eq!(pixel(&page, 5, 60), BLUE);
}

#[test]
fn the_remainder_reaches_the_outer_container_when_stuck() {
    let mut page = Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #o {{ width: 100px; height: 100px; overflow: auto }} #a {{ width: 100px; height: 100px; overflow: auto; scroll-snap-type: y mandatory }}</style></head><body><div id=o><div id=a>{}</div><div style=\"height: 300px; background: white\"></div></div></body></html>",
        bands("")
    ));
    wheel(&mut page, 0.0, 250.0);
    let (_, remaining) = wheel(&mut page, 0.0, 40.0);
    assert_eq!(remaining, 0.0);
}
