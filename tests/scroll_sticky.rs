// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::Page;

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

const SIZE: (usize, usize) = (60, 80);

fn pixel(page: &Page, x: usize, y: usize) -> Color {
    page.render(SIZE.0, SIZE.1, 0.0).canvas.pixel(x, y)
}

fn local(attachment: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 20px; height: 20px; overflow: auto; background: linear-gradient(red, red) 0 0 / 20px 20px no-repeat {a}, linear-gradient(blue, blue) 0 20px / 20px 20px no-repeat {a} }}</style></head><body><div id=a><div style=\"height: 60px\"></div></div></body></html>",
        a = attachment
    ))
}

#[test]
fn local_backgrounds_scroll_with_the_content() {
    let mut page = local("local");
    assert_eq!(pixel(&page, 5, 5), RED);
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 20.0));
    assert_eq!(pixel(&page, 5, 5), BLUE);
}

#[test]
fn scroll_backgrounds_stay_with_the_box() {
    let mut page = local("scroll");
    page.scroll_by(SIZE, 0.0, (5.0, 5.0), (0.0, 20.0));
    assert_eq!(pixel(&page, 5, 5), RED);
}

fn sticky(extra: &str) -> Page {
    Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ margin-top: 20px; width: 40px; height: 40px; overflow: auto; {} }} .s {{ position: sticky; top: 0; height: 10px; background: red }}</style></head><body><div id=a><div style=\"height: 20px\"></div><div class=s></div><div style=\"height: 100px; background: blue\"></div></div><div style=\"height: 400px\"></div></body></html>",
        extra
    ))
}

#[test]
fn sticky_sticks_to_the_scroll_container() {
    let mut page = sticky("");
    assert_eq!(pixel(&page, 5, 45), RED);
    page.scroll_by(SIZE, 0.0, (5.0, 25.0), (0.0, 30.0));
    assert_eq!(pixel(&page, 5, 25), RED);
    assert_eq!(pixel(&page, 5, 35), BLUE);
    assert_eq!(pixel(&page, 5, 15), WHITE);
    assert_eq!(pixel(&page, 5, 5), WHITE);
}

#[test]
fn sticky_offset_is_measured_from_the_container() {
    let mut page = Page::new(
        "<html><head><style>body { margin: 0 } #a { margin-top: 20px; width: 40px; height: 40px; overflow: auto } .s { position: sticky; top: 5px; height: 10px; background: red }</style></head><body><div id=a><div class=s></div><div style=\"height: 100px; background: blue\"></div></div><div style=\"height: 400px\"></div></body></html>",
    );
    page.scroll_by(SIZE, 0.0, (5.0, 25.0), (0.0, 30.0));
    assert_eq!(pixel(&page, 5, 24), BLUE);
    assert_eq!(pixel(&page, 5, 26), RED);
    assert_eq!(pixel(&page, 5, 34), RED);
    assert_eq!(pixel(&page, 5, 36), BLUE);
}

#[test]
fn sticky_in_a_positioned_container_also_uses_it() {
    let mut page = sticky("position: relative");
    page.scroll_by(SIZE, 0.0, (5.0, 25.0), (0.0, 30.0));
    assert_eq!(pixel(&page, 5, 25), RED);
    assert_eq!(pixel(&page, 5, 15), WHITE);
}

#[test]
fn sticky_still_follows_the_page_outside_containers() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } .s { position: sticky; top: 0; height: 10px; background: red }</style></head><body><div style=\"height: 20px\"></div><div class=s></div><div style=\"height: 400px\"></div></body></html>",
    );
    assert_eq!(page.render(SIZE.0, SIZE.1, 100.0).canvas.pixel(5, 5), RED);
}
