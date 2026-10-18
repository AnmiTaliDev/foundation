// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color};
use foundation::media::MediaList;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn matches(query: &str, width: f32, height: f32) -> bool {
    MediaList::parse(query).matches(width, height)
}

#[test]
fn width_and_height_features_compare_with_the_viewport() {
    assert!(matches("(min-width: 600px)", 600.0, 400.0));
    assert!(!matches("(min-width: 600px)", 599.0, 400.0));
    assert!(matches("(max-width: 600px)", 600.0, 400.0));
    assert!(!matches("(max-width: 600px)", 601.0, 400.0));
    assert!(matches("(width: 600px)", 600.0, 400.0));
    assert!(matches("(min-height: 400px)", 600.0, 400.0));
    assert!(!matches("(max-height: 399px)", 600.0, 400.0));
    assert!(matches("(min-width: 40em)", 640.0, 400.0));
    assert!(!matches("(min-width: 40em)", 639.0, 400.0));
    assert!(matches("(min-width: 0)", 1.0, 1.0));
}

#[test]
fn media_types_and_logic() {
    assert!(matches("screen", 100.0, 100.0));
    assert!(matches("all", 100.0, 100.0));
    assert!(matches("", 100.0, 100.0));
    assert!(matches("only screen and (min-width: 50px)", 100.0, 100.0));
    assert!(!matches("print", 100.0, 100.0));
    assert!(!matches("print and (min-width: 50px)", 100.0, 100.0));
    assert!(!matches("not screen", 100.0, 100.0));
    assert!(matches("not print", 100.0, 100.0));
    assert!(matches("not (min-width: 200px)", 100.0, 100.0));
    assert!(matches("print, (min-width: 50px)", 100.0, 100.0));
    assert!(!matches("print, (min-width: 500px)", 100.0, 100.0));
    assert!(matches(
        "(min-width: 50px) and (max-width: 150px)",
        100.0,
        100.0
    ));
    assert!(!matches(
        "(min-width: 50px) and (max-width: 90px)",
        100.0,
        100.0
    ));
}

#[test]
fn range_syntax() {
    assert!(matches("(width >= 300px)", 300.0, 100.0));
    assert!(!matches("(width > 300px)", 300.0, 100.0));
    assert!(matches("(width < 300px)", 299.0, 100.0));
    assert!(matches("(300px <= width)", 300.0, 100.0));
    assert!(matches("(300px <= width <= 500px)", 400.0, 100.0));
    assert!(!matches("(300px <= width <= 500px)", 501.0, 100.0));
    assert!(matches("(300px < width < 500px)", 400.0, 100.0));
    assert!(!matches("(300px < width < 500px)", 300.0, 100.0));
    assert!(matches("(height > 50px)", 300.0, 100.0));
}

#[test]
fn orientation_and_color_scheme() {
    assert!(matches("(orientation: landscape)", 200.0, 100.0));
    assert!(!matches("(orientation: portrait)", 200.0, 100.0));
    assert!(matches("(orientation: portrait)", 100.0, 100.0));
    assert!(matches("(prefers-color-scheme: light)", 100.0, 100.0));
    assert!(!matches("(prefers-color-scheme: dark)", 100.0, 100.0));
}

#[test]
fn unsupported_features_never_match() {
    assert!(!matches("(hover: hover)", 100.0, 100.0));
    assert!(!matches("(min-resolution: 2dppx)", 100.0, 100.0));
    assert!(!matches("(min-width: abc)", 100.0, 100.0));
    assert!(!matches("not (hover: hover)", 100.0, 100.0));
}

#[test]
fn media_blocks_attach_conditions_to_rules() {
    let sheet = css::parse(
        "a { x: 1px } @media (min-width: 10px) { b { x: 2px } @media print { c { x: 3px } } } d { x: 4px }",
    );
    let depths: Vec<usize> = sheet.rules.iter().map(|rule| rule.media.len()).collect();
    assert_eq!(depths, vec![0, 1, 2, 0]);
}

fn pixel_at(page: &Page, width: usize) -> Color {
    page.render(width, 40, 0.0).canvas.pixel(0, 0)
}

#[test]
fn rules_follow_the_viewport_on_each_render() {
    let mut html = String::from("<html><head><style>");
    html.push_str(".a { height: 10px; width: 10px }");
    html.push_str("@media (min-width: 100px) { .a { background-color: red } }");
    html.push_str(
        "@media (min-width: 100px) and (max-width: 199px) { .a { background-color: blue } }",
    );
    html.push_str("@media print { .a { background-color: green } }");
    html.push_str("</style></head><body style=\"margin:0\"><div class=\"a\"></div></body></html>");
    let page = Page::new(&html);
    assert_eq!(pixel_at(&page, 50), WHITE);
    assert_eq!(pixel_at(&page, 150), Color { r: 0, g: 0, b: 255 });
    assert_eq!(pixel_at(&page, 250), RED);
    assert_eq!(pixel_at(&page, 50), WHITE);
}

#[test]
fn rules_after_a_media_block_still_apply() {
    let html = "<html><head><style>.a { height: 10px; width: 10px } @media (min-width: 9999px) { .a { height: 30px } } .a { background-color: red }</style></head><body style=\"margin:0\"><div class=\"a\"></div></body></html>";
    let page = Page::new(html);
    let canvas = page.render(50, 40, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 0), RED);
    assert_eq!(canvas.pixel(0, 20), WHITE);
}
