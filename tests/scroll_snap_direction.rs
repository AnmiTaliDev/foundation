// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const LIME: Color = Color { r: 0, g: 255, b: 0 };

const SIZE: (usize, usize) = (60, 60);

fn page(direction: &str, container: &str) -> Page {
    let item = |color: &str| {
        format!(
            "<div style=\"float: left; width: 50px; height: 10px; background: {}; scroll-snap-align: start\"></div>",
            color
        )
    };
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 50px; height: 20px; overflow-x: scroll; scroll-snap-type: x mandatory; direction: {}; {} }} #w {{ width: 150px; direction: ltr; height: 10px }}</style></head><body><div id=a><div id=w>{}{}{}</div></div></body></html>",
        direction,
        container,
        item("blue"),
        item("red"),
        item("lime")
    ))
}

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

fn wheel(page: &mut Page, dx: f32) -> (f32, f32) {
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (dx, 0.0))
}

#[test]
fn ltr_containers_snap_to_the_left_edges() {
    let mut page = page("ltr", "");
    assert_eq!(pixel(&page, 5, 5), BLUE);
    wheel(&mut page, 20.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    wheel(&mut page, 20.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
}

#[test]
fn rtl_containers_snap_to_the_right_edges() {
    let mut page = page("rtl", "");
    assert_eq!(pixel(&page, 5, 5), LIME);
    wheel(&mut page, -20.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    wheel(&mut page, -20.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
}

#[test]
fn rtl_snapping_works_back_towards_the_origin() {
    let mut page = page("rtl", "");
    wheel(&mut page, -500.0);
    assert_eq!(pixel(&page, 5, 5), BLUE);
    wheel(&mut page, 20.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    wheel(&mut page, 20.0);
    assert_eq!(pixel(&page, 5, 5), LIME);
    assert_eq!(wheel(&mut page, 20.0), (20.0, 0.0));
}

#[test]
fn the_last_point_passes_the_scroll_on_in_rtl() {
    let mut page = page("rtl", "");
    wheel(&mut page, -500.0);
    assert_eq!(wheel(&mut page, -20.0), (-20.0, 0.0));
}

#[test]
fn dragging_settles_on_the_nearest_point_in_rtl() {
    let mut page = page("rtl", "");
    let grab = page.scroll_grab(SIZE, 0.0, (40.0, 16.0)).unwrap();
    page.scroll_drag(SIZE, 0.0, grab, (19.0, 16.0));
    assert_eq!(pixel(&page, 5, 5), RED);
}

#[test]
fn proximity_uses_the_same_mirrored_points() {
    let mut page = page("rtl", "scroll-snap-type: x proximity");
    wheel(&mut page, -10.0);
    assert_eq!(pixel(&page, 5, 5), RED);
    assert_eq!(pixel(&page, 15, 5), LIME);
}
