// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color};
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(body: &str, scroll: f32) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }} .big {{ width: 100px; height: 100px; background-color: red }}</style></head><body>{}</body></html>",
        body
    );
    Page::new(&html).render(120, 120, scroll).canvas
}

#[test]
fn hidden_clips_children_to_the_padding_box() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 30px; height: 20px\"><div class=\"big\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(29, 19), RED);
    assert_eq!(canvas.pixel(30, 10), WHITE);
    assert_eq!(canvas.pixel(10, 20), WHITE);
}

#[test]
fn visible_does_not_clip() {
    let canvas = render(
        "<div style=\"width: 30px; height: 20px\"><div class=\"big\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(60, 60), RED);
}

#[test]
fn clip_includes_padding_but_not_border() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 20px; height: 20px; padding: 5px; border: 5px solid blue\"><div class=\"big\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(2, 2), BLUE);
    assert_eq!(canvas.pixel(7, 7), WHITE);
    assert_eq!(canvas.pixel(12, 12), RED);
    assert_eq!(canvas.pixel(33, 33), RED);
    assert_eq!(canvas.pixel(37, 37), BLUE);
    assert_eq!(canvas.pixel(45, 45), WHITE);
}

#[test]
fn the_box_own_background_and_border_are_not_clipped() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 20px; height: 20px; background-color: blue\"></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(10, 10), BLUE);
}

#[test]
fn clip_applies_to_text() {
    let clipped = render(
        "<div style=\"overflow: hidden; width: 100px; height: 4px; color: red\">MMMMMMMM</div>",
        0.0,
    );
    let free = render(
        "<div style=\"width: 100px; height: 4px; color: red\">MMMMMMMM</div>",
        0.0,
    );
    let ink_below =
        |canvas: &Canvas| (4..20).any(|y| (0..100).any(|x| canvas.pixel(x, y) != WHITE));
    assert!(ink_below(&free));
    assert!(!ink_below(&clipped));
}

#[test]
fn clip_applies_to_images_and_gradients() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 20px; height: 20px\"><div style=\"width: 80px; height: 80px; background-image: linear-gradient(red, red)\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(10, 10), RED);
    assert_eq!(canvas.pixel(40, 10), WHITE);
    assert_eq!(canvas.pixel(10, 40), WHITE);
}

#[test]
fn nested_clips_intersect() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 50px; height: 50px\"><div style=\"overflow: hidden; width: 80px; height: 20px\"><div class=\"big\"></div></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(45, 10), RED);
    assert_eq!(canvas.pixel(60, 10), WHITE);
    assert_eq!(canvas.pixel(10, 30), WHITE);
}

#[test]
fn a_relative_child_is_clipped() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 30px; height: 30px\"><div class=\"big\" style=\"position: relative; left: 10px\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(40, 20), WHITE);
}

#[test]
fn an_absolute_child_is_clipped_when_the_box_is_its_containing_block() {
    let canvas = render(
        "<div style=\"overflow: hidden; position: relative; width: 30px; height: 30px\"><div class=\"big\" style=\"position: absolute; top: 0; left: 0\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(40, 20), WHITE);
}

#[test]
fn an_absolute_child_escapes_a_static_clipping_ancestor() {
    let canvas = render(
        "<div style=\"position: relative\"><div style=\"overflow: hidden; width: 30px; height: 30px\"><div class=\"big\" style=\"position: absolute; top: 0; left: 0\"></div></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(60, 60), RED);
}

#[test]
fn clip_scrolls_with_the_page() {
    let canvas = render(
        "<div style=\"height: 40px\"></div><div style=\"overflow: hidden; width: 30px; height: 20px\"><div class=\"big\"></div></div><div style=\"height: 300px\"></div>",
        30.0,
    );
    assert_eq!(canvas.pixel(10, 15), RED);
    assert_eq!(canvas.pixel(10, 5), WHITE);
    assert_eq!(canvas.pixel(10, 35), WHITE);
}

#[test]
fn clip_works_inside_an_opacity_group() {
    let canvas = render(
        "<div style=\"opacity: 1; overflow: hidden; width: 30px; height: 20px\"><div class=\"big\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(10, 10), RED);
    assert_eq!(canvas.pixel(40, 10), WHITE);
}

#[test]
fn clip_works_inside_a_transform() {
    let canvas = render(
        "<div style=\"transform: translate(10px, 0); overflow: hidden; width: 30px; height: 20px\"><div class=\"big\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(20, 10), RED);
    assert_eq!(canvas.pixel(45, 10), WHITE);
}

#[test]
fn an_ancestor_clip_applies_to_a_transformed_child() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 30px; height: 30px\"><div class=\"big\" style=\"transform: translate(10px, 0)\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(20, 10), RED);
    assert_eq!(canvas.pixel(40, 10), WHITE);
}

#[test]
fn clip_and_scroll_and_auto_all_clip() {
    for value in ["clip", "scroll", "auto"] {
        let canvas = render(
            &format!(
                "<div style=\"overflow: {}; width: 30px; height: 20px\"><div class=\"big\"></div></div>",
                value
            ),
            0.0,
        );
        assert_eq!(canvas.pixel(40, 10), WHITE, "{}", value);
    }
}

#[test]
fn a_single_axis_still_clips_both() {
    let canvas = render(
        "<div style=\"overflow-x: hidden; width: 30px; height: 20px\"><div class=\"big\"></div></div>",
        0.0,
    );
    assert_eq!(canvas.pixel(40, 10), WHITE);
    assert_eq!(canvas.pixel(10, 40), WHITE);
}

#[test]
fn the_two_value_form_is_split_into_axes() {
    let declarations = css::parse_declarations("overflow: hidden auto");
    let names: Vec<&str> = declarations.iter().map(|d| d.name.as_str()).collect();
    assert_eq!(names, ["overflow-x", "overflow-y"]);
    assert!(css::parse_declarations("overflow: bogus").is_empty());
}

#[test]
fn overflow_on_body_does_not_clip_the_page() {
    let html = "<html><head><style>body { margin: 0; overflow: hidden; height: 10px } .big { width: 100px; height: 100px; background-color: red }</style></head><body><div class=\"big\"></div></body></html>";
    let canvas = Page::new(html).render(120, 120, 0.0).canvas;
    assert_eq!(canvas.pixel(50, 50), RED);
}
