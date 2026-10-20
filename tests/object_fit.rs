// SPDX-License-Identifier: GPL-3.0-only

use std::fs;

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

fn picture() -> Vec<u8> {
    let mut image = image::RgbaImage::new(4, 2);
    for (x, _, pixel) in image.enumerate_pixels_mut() {
        *pixel = if x < 2 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 0, 255, 255])
        };
    }
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn render(name: &str, style: &str, display: &str) -> Canvas {
    let directory =
        std::env::temp_dir().join(format!("foundation-fit-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("p.png"), picture()).unwrap();
    fs::write(
        directory.join("index.html"),
        format!(
            "<html><style>body {{ margin: 0 }} img {{ display: {}; width: 20px; height: 20px; {} }}</style><body><img src=\"p.png\"></body></html>",
            display, style
        ),
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(60, 60, 0.0).canvas;
    fs::remove_dir_all(&directory).unwrap();
    canvas
}

fn block(style: &str) -> Canvas {
    render(&style.replace(' ', "").replace(':', "-"), style, "block")
}

fn bounds(canvas: &Canvas) -> (usize, usize, usize, usize) {
    let mut found = (usize::MAX, usize::MAX, 0, 0);
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if canvas.pixel(x, y) != WHITE {
                found = (
                    found.0.min(x),
                    found.1.min(y),
                    found.2.max(x),
                    found.3.max(y),
                );
            }
        }
    }
    found
}

#[test]
fn fill_stretches_the_picture() {
    let canvas = block("object-fit: fill");
    assert_eq!(canvas.pixel(2, 2), RED);
    assert_eq!(canvas.pixel(2, 18), RED);
    assert_eq!(canvas.pixel(18, 2), BLUE);
    assert_eq!(bounds(&canvas), (0, 0, 19, 19));
}

#[test]
fn contain_fits_inside_and_centers() {
    let canvas = block("object-fit: contain");
    assert_eq!(bounds(&canvas), (0, 5, 19, 14));
    assert_eq!(canvas.pixel(2, 10), RED);
    assert_eq!(canvas.pixel(18, 10), BLUE);
    assert_eq!(canvas.pixel(10, 2), WHITE);
}

#[test]
fn cover_fills_and_crops() {
    let canvas = block("object-fit: cover");
    assert_eq!(bounds(&canvas), (0, 0, 19, 19));
    assert_eq!(canvas.pixel(2, 10), RED);
    assert_eq!(canvas.pixel(18, 10), BLUE);
}

#[test]
fn cover_is_clipped_to_the_box() {
    let canvas = render(
        "clip",
        "object-fit: cover; margin: 20px; object-position: left",
        "block",
    );
    assert_eq!(bounds(&canvas), (20, 20, 39, 39));
}

#[test]
fn cover_position_shifts_the_crop() {
    let canvas = block("object-fit: cover; object-position: left");
    assert_eq!(canvas.pixel(18, 10), RED);
    let right = block("object-fit: cover; object-position: right");
    assert_eq!(right.pixel(2, 10), BLUE);
}

#[test]
fn none_keeps_the_natural_size() {
    let canvas = block("object-fit: none");
    assert_eq!(bounds(&canvas), (8, 9, 11, 10));
    assert_eq!(canvas.pixel(9, 10), RED);
    assert_eq!(canvas.pixel(10, 10), BLUE);
}

#[test]
fn scale_down_never_enlarges() {
    let canvas = block("object-fit: scale-down");
    assert_eq!(bounds(&canvas), (8, 9, 11, 10));
}

#[test]
fn scale_down_shrinks_like_contain_when_the_box_is_smaller() {
    let canvas = block("object-fit: scale-down; width: 2px; height: 2px");
    assert_eq!(bounds(&canvas), (0, 1, 1, 1));
}

#[test]
fn object_position_accepts_keywords_lengths_and_percentages() {
    let top_left = block("object-fit: contain; object-position: top left");
    assert_eq!(bounds(&top_left), (0, 0, 19, 9));
    let lengths = block("object-fit: none; object-position: 1px 1px");
    assert_eq!(bounds(&lengths), (1, 1, 4, 2));
    let percent = block("object-fit: none; object-position: 100% 100%");
    assert_eq!(bounds(&percent), (16, 18, 19, 19));
}

#[test]
fn inline_images_follow_object_fit() {
    let canvas = render("inline", "object-fit: contain", "inline");
    let (left, top, right, bottom) = bounds(&canvas);
    assert_eq!((left, right), (0, 19));
    assert_eq!(bottom - top, 9);
}

#[test]
fn parsing_drops_invalid_values() {
    let sheet = css::parse("a { object-fit: stretch } b { object-fit: cover } c { object-position: 5 } d { object-position: left top }");
    assert!(sheet.rules[0].declarations.is_empty());
    assert_eq!(sheet.rules[1].declarations.len(), 1);
    assert!(sheet.rules[2].declarations.is_empty());
    assert_eq!(sheet.rules[3].declarations.len(), 2);
}

#[test]
fn supports_the_property() {
    assert!(foundation::supports::evaluate("(object-fit: cover)"));
}
