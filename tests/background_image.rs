// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::path::{Path, PathBuf};

use foundation::css::{self, Color, Value};
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn checker() -> Vec<u8> {
    let mut image = image::RgbaImage::new(2, 2);
    for (x, y, pixel) in image.enumerate_pixels_mut() {
        *pixel = if (x + y) % 2 == 0 {
            image::Rgba([255, 0, 0, 255])
        } else {
            image::Rgba([0, 255, 0, 255])
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

fn temp_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("foundation-bg-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(directory.join("img")).unwrap();
    fs::create_dir_all(directory.join("css")).unwrap();
    fs::write(directory.join("img/t.png"), checker()).unwrap();
    directory
}

fn render(directory: &Path, rule: &str, size: usize) -> foundation::paint::Canvas {
    fs::write(
        directory.join("css/a.css"),
        format!(
            "body {{ margin: 0 }} .a {{ width: 10px; height: 10px; {} }}",
            rule
        ),
    )
    .unwrap();
    fs::write(
        directory.join("index.html"),
        "<html><head><link rel=\"stylesheet\" href=\"css/a.css\"></head><body><div class=\"a\"></div></body></html>",
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    page.render(size, size, 0.0).canvas
}

#[test]
fn image_tiles_in_both_directions_by_default() {
    let directory = temp_dir("tiles");
    let canvas = render(&directory, "background-image: url(../img/t.png)", 20);
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 0), GREEN);
    assert_eq!(canvas.pixel(0, 1), GREEN);
    assert_eq!(canvas.pixel(1, 1), RED);
    assert_eq!(canvas.pixel(8, 8), RED);
    assert_eq!(canvas.pixel(9, 8), GREEN);
    assert_eq!(canvas.pixel(10, 0), WHITE);
    assert_eq!(canvas.pixel(0, 10), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn no_repeat_draws_a_single_tile() {
    let directory = temp_dir("single");
    let canvas = render(
        &directory,
        "background: url(\"../img/t.png\") no-repeat",
        20,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 0), GREEN);
    assert_eq!(canvas.pixel(2, 0), WHITE);
    assert_eq!(canvas.pixel(0, 2), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn repeat_x_and_repeat_y_tile_one_axis() {
    let directory = temp_dir("axis");
    let canvas = render(&directory, "background: url(../img/t.png) repeat-x", 20);
    assert_eq!(canvas.pixel(4, 0), RED);
    assert_eq!(canvas.pixel(4, 2), WHITE);
    let canvas = render(&directory, "background: url(../img/t.png) repeat-y", 20);
    assert_eq!(canvas.pixel(0, 4), RED);
    assert_eq!(canvas.pixel(2, 4), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn position_keywords_lengths_and_percentages() {
    let directory = temp_dir("position");
    let canvas = render(
        &directory,
        "background: url(../img/t.png) no-repeat right bottom",
        20,
    );
    assert_eq!(canvas.pixel(9, 9), RED);
    assert_eq!(canvas.pixel(8, 9), GREEN);
    assert_eq!(canvas.pixel(7, 9), WHITE);
    assert_eq!(canvas.pixel(0, 0), WHITE);

    let canvas = render(
        &directory,
        "background-image: url(../img/t.png); background-repeat: no-repeat; background-position: 50% 50%",
        20,
    );
    assert_eq!(canvas.pixel(4, 4), RED);
    assert_eq!(canvas.pixel(5, 4), GREEN);
    assert_eq!(canvas.pixel(3, 4), WHITE);

    let canvas = render(
        &directory,
        "background: url(../img/t.png) no-repeat 3px 5px",
        20,
    );
    assert_eq!(canvas.pixel(3, 5), RED);
    assert_eq!(canvas.pixel(4, 5), GREEN);
    assert_eq!(canvas.pixel(2, 5), WHITE);

    let canvas = render(
        &directory,
        "background: url(../img/t.png) no-repeat center",
        20,
    );
    assert_eq!(canvas.pixel(4, 4), RED);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn size_scales_the_tile() {
    let directory = temp_dir("size");
    let canvas = render(
        &directory,
        "background: url(../img/t.png) no-repeat; background-size: 10px 10px",
        20,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(7, 0), GREEN);
    assert_eq!(canvas.pixel(0, 7), GREEN);
    assert_eq!(canvas.pixel(9, 9), RED);

    let canvas = render(
        &directory,
        "background: url(../img/t.png) no-repeat; background-size: 100%",
        20,
    );
    assert_eq!(canvas.pixel(9, 9), RED);
    assert_eq!(canvas.pixel(9, 0), GREEN);

    let canvas = render(
        &directory,
        "width: 20px; height: 10px; background: url(../img/t.png) no-repeat center / cover",
        30,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(19, 0), GREEN);
    assert_eq!(canvas.pixel(9, 9), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn contain_fits_the_whole_image() {
    let directory = temp_dir("contain");
    let canvas = render(
        &directory,
        "width: 20px; height: 10px; background: url(../img/t.png) no-repeat 0 0 / contain",
        30,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(9, 0), GREEN);
    assert_eq!(canvas.pixel(9, 9), RED);
    assert_eq!(canvas.pixel(10, 0), WHITE);
    assert_eq!(canvas.pixel(19, 0), WHITE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn color_stays_behind_the_image() {
    let directory = temp_dir("color");
    let canvas = render(
        &directory,
        "background: blue url(../img/t.png) no-repeat",
        20,
    );
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(5, 5), BLUE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn inline_style_url_is_relative_to_the_document() {
    let directory = temp_dir("inline");
    fs::write(
        directory.join("index.html"),
        "<html><body style=\"margin:0\"><div style=\"width:10px;height:10px;background:url(img/t.png) no-repeat\"></div></body></html>",
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert!(page.warnings.is_empty(), "{:?}", page.warnings);
    let canvas = page.render(20, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(1, 0), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn missing_image_warns_and_keeps_the_color() {
    let directory = temp_dir("missing");
    fs::write(
        directory.join("index.html"),
        "<html><body style=\"margin:0\"><div style=\"width:10px;height:10px;background:blue url(none.png)\"></div></body></html>",
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    assert_eq!(page.warnings.len(), 1, "{:?}", page.warnings);
    let canvas = page.render(20, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(5, 5), BLUE);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn scroll_moves_the_image_with_the_page() {
    let directory = temp_dir("scroll");
    fs::write(
        directory.join("index.html"),
        "<html><body style=\"margin:0\"><div style=\"height:30px\"></div><div style=\"width:10px;height:10px;background:url(img/t.png) no-repeat\"></div><div style=\"height:100px\"></div></body></html>",
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    let canvas = page.render(20, 20, 25.0).canvas;
    assert_eq!(canvas.pixel(0, 5), RED);
    assert_eq!(canvas.pixel(1, 5), GREEN);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn url_values_and_shorthand_parts_are_parsed() {
    let sheet = css::parse(
        "a { background: url('x y.png') repeat-x 10px 20% / 5px; b: url( z.png ); c: url() }",
    );
    let declarations = &sheet.rules[0].declarations;
    let find = |name: &str| {
        declarations
            .iter()
            .find(|d| d.name == name)
            .map(|d| &d.value)
    };
    assert_eq!(
        find("background-image"),
        Some(&Value::Url("x y.png".to_string()))
    );
    assert_eq!(
        find("background-repeat"),
        Some(&Value::Keyword("repeat-x".to_string()))
    );
    assert_eq!(find("background-position-x"), Some(&Value::Length(10.0)));
    assert_eq!(find("background-position-y"), Some(&Value::Percent(20.0)));
    assert_eq!(find("background-size-width"), Some(&Value::Length(5.0)));
    assert_eq!(find("b"), Some(&Value::Url("z.png".to_string())));
    assert_eq!(find("c"), None);
}
