// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::layout::Rect;
use foundation::paint::Canvas;
use foundation::radius::{Radii, Round};
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(body: &str) -> Canvas {
    let html = format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    );
    Page::new(&html).render(120, 120, 0.0).canvas
}

fn values(text: &str) -> Vec<(String, f32)> {
    css::parse_declarations(text)
        .into_iter()
        .map(|d| match d.value {
            Value::Length(px) => (d.name, px),
            other => panic!("{:?}", other),
        })
        .collect()
}

#[test]
fn square_corners_stay_square_without_a_radius() {
    let canvas = render("<div style=\"width: 40px; height: 40px; background-color: red\"></div>");
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(39, 39), RED);
}

#[test]
fn a_radius_rounds_the_background_corners() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-radius: 10px\"></div>",
    );
    assert_eq!(canvas.pixel(0, 0), WHITE);
    assert_eq!(canvas.pixel(39, 0), WHITE);
    assert_eq!(canvas.pixel(0, 39), WHITE);
    assert_eq!(canvas.pixel(39, 39), WHITE);
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(20, 0), RED);
    assert_eq!(canvas.pixel(0, 20), RED);
    assert_eq!(canvas.pixel(2, 2), WHITE);
    assert_eq!(canvas.pixel(8, 8), RED);
}

#[test]
fn the_edge_is_anti_aliased() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-radius: 20px\"></div>",
    );
    let mixed = (0..40)
        .flat_map(|y| (0..40).map(move |x| (x, y)))
        .filter(|(x, y)| {
            let pixel = canvas.pixel(*x, *y);
            pixel != WHITE && pixel != RED
        })
        .count();
    assert!(mixed > 20);
}

#[test]
fn half_the_size_makes_a_circle() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-radius: 50%\"></div>",
    );
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(1, 1), WHITE);
    assert_eq!(canvas.pixel(38, 38), WHITE);
    assert_eq!(canvas.pixel(20, 1), RED);
}

#[test]
fn radii_larger_than_the_box_are_scaled_down() {
    let canvas = render(
        "<div style=\"width: 20px; height: 20px; background-color: red; border-radius: 100px\"></div>",
    );
    assert_eq!(canvas.pixel(10, 10), RED);
    assert_eq!(canvas.pixel(1, 1), WHITE);
}

#[test]
fn each_corner_can_differ() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-radius: 15px 0 0 0\"></div>",
    );
    assert_eq!(canvas.pixel(0, 0), WHITE);
    assert_eq!(canvas.pixel(39, 0), RED);
    assert_eq!(canvas.pixel(0, 39), RED);
    assert_eq!(canvas.pixel(39, 39), RED);
}

#[test]
fn longhand_sets_one_corner() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-bottom-right-radius: 15px\"></div>",
    );
    assert_eq!(canvas.pixel(39, 39), WHITE);
    assert_eq!(canvas.pixel(0, 0), RED);
}

#[test]
fn borders_follow_the_curve() {
    let canvas = render(
        "<div style=\"width: 30px; height: 30px; border: 5px solid blue; border-radius: 20px\"></div>",
    );
    assert_eq!(canvas.pixel(0, 0), WHITE);
    assert_eq!(canvas.pixel(20, 1), BLUE);
    assert_eq!(canvas.pixel(1, 20), BLUE);
    assert_eq!(canvas.pixel(20, 20), WHITE);
}

#[test]
fn a_border_on_a_rounded_box_covers_the_background_edge() {
    let canvas = render(
        "<div style=\"width: 30px; height: 30px; background-color: red; border: 5px solid blue; border-radius: 12px\"></div>",
    );
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(20, 2), BLUE);
    assert_eq!(canvas.pixel(0, 0), WHITE);
}

#[test]
fn overflow_clips_children_to_the_rounded_padding_box() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 40px; height: 40px; border-radius: 20px\"><div style=\"width: 100px; height: 100px; background-color: red\"></div></div>",
    );
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(1, 1), WHITE);
    assert_eq!(canvas.pixel(38, 1), WHITE);
}

#[test]
fn gradients_are_rounded() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-image: linear-gradient(red, red); border-radius: 15px\"></div>",
    );
    assert_eq!(canvas.pixel(0, 0), WHITE);
    assert_eq!(canvas.pixel(20, 20), RED);
}

#[test]
fn rounding_is_kept_inside_a_scrolled_page() {
    let html = "<html><head><style>body { margin: 0 }</style></head><body><div style=\"height: 50px\"></div><div style=\"width: 40px; height: 40px; background-color: red; border-radius: 20px\"></div><div style=\"height: 400px\"></div></body></html>";
    let canvas = Page::new(html).render(120, 120, 30.0).canvas;
    assert_eq!(canvas.pixel(20, 40), RED);
    assert_eq!(canvas.pixel(1, 21), WHITE);
}

#[test]
fn rounding_is_kept_under_opacity_and_transform() {
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-radius: 20px; opacity: 1\"></div>",
    );
    assert_eq!(canvas.pixel(1, 1), WHITE);
    let canvas = render(
        "<div style=\"width: 40px; height: 40px; background-color: red; border-radius: 20px; transform: translate(10px, 10px)\"></div>",
    );
    assert_eq!(canvas.pixel(30, 30), RED);
    assert_eq!(canvas.pixel(11, 11), WHITE);
}

#[test]
fn an_ancestor_rounding_clips_a_transformed_child() {
    let canvas = render(
        "<div style=\"overflow: hidden; width: 40px; height: 40px; border-radius: 20px\"><div style=\"width: 100px; height: 100px; background-color: red; transform: translate(0, 0)\"></div></div>",
    );
    assert_eq!(canvas.pixel(20, 20), RED);
    assert_eq!(canvas.pixel(1, 1), WHITE);
}

#[test]
fn shorthand_expands_to_corner_pairs() {
    let all = values("border-radius: 4px");
    assert_eq!(all.len(), 8);
    assert!(all.iter().all(|(_, px)| *px == 4.0));
    let two = values("border-radius: 1px 2px");
    let find = |name: &str| two.iter().find(|(n, _)| n == name).unwrap().1;
    assert_eq!(find("border-top-left-radius-x"), 1.0);
    assert_eq!(find("border-top-right-radius-x"), 2.0);
    assert_eq!(find("border-bottom-right-radius-x"), 1.0);
    assert_eq!(find("border-bottom-left-radius-x"), 2.0);
    let three = values("border-radius: 1px 2px 3px");
    let find = |name: &str| three.iter().find(|(n, _)| n == name).unwrap().1;
    assert_eq!(find("border-bottom-right-radius-x"), 3.0);
    assert_eq!(find("border-bottom-left-radius-x"), 2.0);
}

#[test]
fn a_slash_separates_horizontal_and_vertical_radii() {
    let split = values("border-radius: 10px / 20px");
    let find = |name: &str| split.iter().find(|(n, _)| n == name).unwrap().1;
    assert_eq!(find("border-top-left-radius-x"), 10.0);
    assert_eq!(find("border-top-left-radius-y"), 20.0);
}

#[test]
fn invalid_radii_are_dropped() {
    assert!(css::parse_declarations("border-radius: red").is_empty());
    assert!(css::parse_declarations("border-radius: 1px 2px 3px 4px 5px").is_empty());
    assert!(css::parse_declarations("border-top-left-radius: 1px 2px 3px").is_empty());
}

#[test]
fn fit_scales_overlapping_radii() {
    let radii = Radii {
        corners: [(30.0, 30.0), (30.0, 30.0), (0.0, 0.0), (0.0, 0.0)],
    }
    .fit(40.0, 100.0);
    assert!((radii.corners[0].0 - 20.0).abs() < 0.001);
    assert!((radii.corners[1].1 - 20.0).abs() < 0.001);
}

#[test]
fn shrink_reduces_by_the_border_widths() {
    let radii = Radii {
        corners: [(10.0, 10.0); 4],
    }
    .shrink([2.0, 3.0, 4.0, 5.0]);
    assert_eq!(radii.corners[0], (5.0, 8.0));
    assert_eq!(radii.corners[1], (7.0, 8.0));
    assert_eq!(radii.corners[2], (7.0, 6.0));
    assert_eq!(radii.corners[3], (5.0, 6.0));
}

#[test]
fn coverage_is_full_inside_and_empty_outside() {
    let round = Round {
        rect: Rect {
            x: 0.0,
            y: 0.0,
            width: 40.0,
            height: 40.0,
        },
        radii: Radii {
            corners: [(20.0, 20.0); 4],
        },
        inverted: false,
    };
    assert_eq!(round.coverage(20.0, 20.0), 1.0);
    assert_eq!(round.coverage(0.5, 0.5), 0.0);
    assert_eq!(round.coverage(-5.0, 20.0), 0.0);
    let inverted = Round {
        inverted: true,
        ..round
    };
    assert_eq!(inverted.coverage(20.0, 20.0), 0.0);
}

#[test]
fn supports_recognizes_the_property() {
    assert!(foundation::supports::evaluate("(border-radius: 4px)"));
}
