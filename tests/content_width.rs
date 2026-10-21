// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };

const CHILDREN: &str = "<span style=\"display: inline-block; width: 30px; height: 10px\"></span><span style=\"display: inline-block; width: 40px; height: 10px\"></span>";

fn render(box_style: &str, container: u32) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: {}px\"><div style=\"background: red; {}\">{}</div></div></body></html>",
        container, box_style, CHILDREN
    );
    Page::new(&html).render(200, 40, 0.0).canvas
}

fn filled(canvas: &Canvas) -> (usize, usize) {
    let reds: Vec<usize> = (0..200).filter(|x| canvas.pixel(*x, 2) == RED).collect();
    match (reds.first(), reds.last()) {
        (Some(first), Some(last)) => (*first, last + 1),
        _ => (0, 0),
    }
}

#[test]
fn max_content_fits_all_children_on_one_line() {
    assert_eq!(filled(&render("width: max-content", 100)), (0, 70));
}

#[test]
fn max_content_overflows_a_narrow_container() {
    assert_eq!(filled(&render("width: max-content", 50)), (0, 70));
}

#[test]
fn min_content_fits_the_widest_child() {
    assert_eq!(filled(&render("width: min-content", 100)), (0, 40));
}

#[test]
fn fit_content_is_limited_by_the_container() {
    assert_eq!(filled(&render("width: fit-content", 200)), (0, 70));
    assert_eq!(filled(&render("width: fit-content", 50)), (0, 50));
    assert_eq!(filled(&render("width: fit-content", 20)), (0, 40));
}

#[test]
fn auto_margins_center_a_content_sized_box() {
    assert_eq!(
        filled(&render("width: max-content; margin: 0 auto", 100)),
        (15, 85)
    );
}

#[test]
fn padding_and_border_are_added_outside_the_content() {
    assert_eq!(
        filled(&render("width: min-content; padding: 0 5px", 100)),
        (0, 50)
    );
}

#[test]
fn max_width_accepts_content_keywords() {
    assert_eq!(
        filled(&render("width: 100%; max-width: min-content", 100)),
        (0, 40)
    );
}

#[test]
fn min_width_accepts_content_keywords() {
    assert_eq!(
        filled(&render("width: 10px; min-width: max-content", 100)),
        (0, 70)
    );
}

#[test]
fn border_box_does_not_change_content_sizes() {
    assert_eq!(
        filled(&render("box-sizing: border-box; width: min-content", 100)),
        (0, 40)
    );
}

#[test]
fn supports_accepts_the_keywords() {
    assert!(foundation::supports::evaluate("(width: fit-content)"));
    assert!(foundation::supports::evaluate("(min-width: max-content)"));
}

fn render_in(outer: &str, inner: &str, container: u32) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: {}px; {}\"><div style=\"background: red; {}\">{}</div></div></body></html>",
        container, outer, inner, CHILDREN
    );
    Page::new(&html).render(200, 40, 0.0).canvas
}

#[test]
fn floats_accept_content_keywords() {
    assert_eq!(
        filled(&render_in("", "float: left; width: min-content", 100)),
        (0, 40)
    );
    assert_eq!(
        filled(&render_in("", "float: left; width: max-content", 50)),
        (0, 70)
    );
    assert_eq!(
        filled(&render_in("", "float: left; width: fit-content", 50)),
        (0, 50)
    );
}

#[test]
fn inline_blocks_accept_content_keywords() {
    assert_eq!(
        filled(&render_in(
            "",
            "display: inline-block; width: min-content",
            100
        )),
        (0, 40)
    );
}

#[test]
fn absolute_boxes_accept_content_keywords() {
    assert_eq!(
        filled(&render_in(
            "position: relative",
            "position: absolute; width: min-content",
            100
        )),
        (0, 40)
    );
    assert_eq!(
        filled(&render_in(
            "position: relative",
            "position: absolute; width: 100%; max-width: min-content",
            100
        )),
        (0, 40)
    );
}

#[test]
fn flex_items_accept_content_keywords() {
    assert_eq!(
        filled(&render_in(
            "display: flex",
            "flex: none; width: min-content",
            100
        )),
        (0, 40)
    );
    assert_eq!(
        filled(&render_in(
            "display: flex",
            "flex: none; width: max-content",
            50
        )),
        (0, 70)
    );
    assert_eq!(
        filled(&render_in(
            "display: flex",
            "flex: 1; max-width: min-content",
            100
        )),
        (0, 40)
    );
}

#[test]
fn flex_min_width_accepts_content_keywords() {
    assert_eq!(
        filled(&render_in(
            "display: flex",
            "flex: 0 1 10px; min-width: max-content",
            100
        )),
        (0, 70)
    );
}

#[test]
fn column_flex_items_accept_content_keywords_across() {
    assert_eq!(
        filled(&render_in(
            "display: flex; flex-direction: column; align-items: flex-start",
            "width: min-content",
            100
        )),
        (0, 40)
    );
}

#[test]
fn grid_items_accept_content_keywords() {
    assert_eq!(
        filled(&render_in(
            "display: grid; grid-template-columns: 100px",
            "width: min-content",
            100
        )),
        (0, 40)
    );
    assert_eq!(
        filled(&render_in(
            "display: grid; grid-template-columns: 100px",
            "width: 100%; max-width: max-content",
            100
        )),
        (0, 70)
    );
}
