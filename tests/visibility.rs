// SPDX-License-Identifier: GPL-3.0-only

use std::fs;

use foundation::css::Color;
use foundation::paint::Canvas;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn render(body: &str) -> Canvas {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    ))
    .render(100, 60, 0.0)
    .canvas
}

fn count(canvas: &Canvas, wanted: impl Fn(Color) -> bool) -> usize {
    let mut total = 0;
    for y in 0..canvas.height {
        for x in 0..canvas.width {
            if wanted(canvas.pixel(x, y)) {
                total += 1;
            }
        }
    }
    total
}

fn blank(canvas: &Canvas) -> bool {
    count(canvas, |color| color != WHITE) == 0
}

fn same(a: &Canvas, b: &Canvas) -> bool {
    (0..a.height).all(|y| (0..a.width).all(|x| a.pixel(x, y) == b.pixel(x, y)))
}

#[test]
fn hidden_boxes_keep_their_space_but_are_not_painted() {
    let canvas = render(
        "<div style=\"visibility: hidden; background: red; height: 10px\"></div><div style=\"background: blue; height: 10px\"></div>",
    );
    assert_eq!(canvas.pixel(5, 5), WHITE);
    assert_eq!(canvas.pixel(5, 15), BLUE);
    assert_eq!(count(&canvas, |color| color == RED), 0);
}

#[test]
fn visible_children_of_hidden_parents_are_painted() {
    let canvas = render(
        "<div style=\"visibility: hidden; background: red; height: 20px\"><div style=\"visibility: visible; background: blue; height: 10px\"></div></div>",
    );
    assert_eq!(canvas.pixel(5, 5), BLUE);
    assert_eq!(canvas.pixel(5, 15), WHITE);
}

#[test]
fn hidden_is_inherited() {
    let canvas = render(
        "<div style=\"visibility: hidden\"><div style=\"background: red; height: 10px\"></div></div>",
    );
    assert!(blank(&canvas));
}

#[test]
fn visibility_does_not_move_anything() {
    let shown = render(
        "<div style=\"height: 10px\"></div><div style=\"background: blue; height: 10px\"></div>",
    );
    let hidden = render(
        "<div style=\"visibility: hidden; height: 10px\"></div><div style=\"background: blue; height: 10px\"></div>",
    );
    assert!(same(&shown, &hidden));
}

#[test]
fn hidden_text_is_not_drawn() {
    let canvas = render("<div style=\"visibility: hidden\">Some text here</div>");
    assert!(blank(&canvas));
}

#[test]
fn hidden_inline_text_still_takes_space() {
    let hidden = render("<p>a<span style=\"visibility: hidden\">bbbb</span>c</p>");
    let white = render("<p>a<span style=\"color: white\">bbbb</span>c</p>");
    assert!(same(&hidden, &white));
    assert!(!blank(&hidden));
}

#[test]
fn hidden_inline_backgrounds_and_borders_are_not_drawn() {
    let canvas = render(
        "<span style=\"visibility: hidden; background: red; border: 2px solid red; padding: 2px\">x</span>",
    );
    assert!(blank(&canvas));
}

#[test]
fn hidden_text_decorations_are_not_drawn() {
    let canvas = render("<u style=\"visibility: hidden\">underlined</u>");
    assert!(blank(&canvas));
}

#[test]
fn a_visible_child_of_a_decorated_hidden_parent_keeps_its_text() {
    let canvas = render(
        "<div style=\"visibility: hidden\">hidden<span style=\"visibility: visible\"> shown</span></div>",
    );
    assert!(!blank(&canvas));
    let only = render("<div style=\"visibility: hidden\">hidden<span style=\"visibility: hidden\"> shown</span></div>");
    assert!(blank(&only));
}

#[test]
fn hidden_list_markers_are_not_drawn() {
    let canvas = render("<ul style=\"visibility: hidden\"><li></li></ul>");
    assert!(blank(&canvas));
}

#[test]
fn hidden_outlines_shadows_and_scrollbars_are_not_drawn() {
    let outline = render("<div style=\"visibility: hidden; outline: 3px solid red; margin: 10px; height: 10px\"></div>");
    assert!(blank(&outline));
    let shadow = render("<div style=\"visibility: hidden; box-shadow: 4px 4px 0 red; margin: 10px; height: 10px\"></div>");
    assert!(blank(&shadow));
    let bars = render(
        "<div style=\"visibility: hidden; overflow: scroll; width: 40px; height: 40px\"></div>",
    );
    assert!(blank(&bars));
}

#[test]
fn collapse_acts_like_hidden_outside_tables() {
    let canvas =
        render("<div style=\"visibility: collapse; background: red; height: 10px\"></div>");
    assert!(blank(&canvas));
}

#[test]
fn hidden_form_controls_are_not_drawn() {
    let canvas = render("<input style=\"visibility: hidden\" value=\"text\">");
    assert!(blank(&canvas));
}

#[test]
fn hidden_images_keep_their_size() {
    let directory =
        std::env::temp_dir().join(format!("foundation-visibility-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let mut image = image::RgbaImage::new(4, 2);
    for pixel in image.pixels_mut() {
        *pixel = image::Rgba([255, 0, 0, 255]);
    }
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    fs::write(directory.join("p.png"), bytes).unwrap();
    let write = |name: &str, style: &str| {
        fs::write(
            directory.join(name),
            format!(
                "<html><style>body {{ margin: 0 }} img {{ display: block; width: 20px; height: 10px; {} }}</style><body><img src=\"p.png\"><div style=\"background: blue; height: 10px\"></div></body></html>",
                style
            ),
        )
        .unwrap();
        Page::open(directory.join(name).to_str().unwrap())
            .unwrap()
            .render(60, 40, 0.0)
            .canvas
    };
    let shown = write("shown.html", "");
    let hidden = write("hidden.html", "visibility: hidden");
    fs::remove_dir_all(&directory).unwrap();
    assert_eq!(shown.pixel(5, 5), RED);
    assert_eq!(hidden.pixel(5, 5), WHITE);
    assert_eq!(hidden.pixel(5, 15), BLUE);
}
