// SPDX-License-Identifier: GPL-3.0-only

use std::fs;

use foundation::Page;

const WHITE: foundation::css::Color = foundation::css::Color {
    r: 255,
    g: 255,
    b: 255,
};

fn picture() -> Vec<u8> {
    let image = image::RgbaImage::from_pixel(4, 2, image::Rgba([255, 0, 0, 255]));
    let mut bytes = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut bytes),
            image::ImageFormat::Png,
        )
        .unwrap();
    bytes
}

fn size(name: &str, style: &str, display: &str) -> (usize, usize) {
    let directory =
        std::env::temp_dir().join(format!("foundation-ratio-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::write(directory.join("p.png"), picture()).unwrap();
    fs::write(
        directory.join("index.html"),
        format!(
            "<html><style>body {{ margin: 0 }} img {{ display: {}; {} }}</style><body><img src=\"p.png\"></body></html>",
            display, style
        ),
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    let canvas = page.render(100, 100, 0.0).canvas;
    let _ = fs::remove_dir_all(&directory);
    let painted = |x: usize, y: usize| canvas.pixel(x, y) != WHITE;
    let width = (0..100).filter(|x| painted(*x, 0)).count();
    let height = (0..100).filter(|y| painted(0, *y)).count();
    (width, height)
}

#[test]
fn the_ratio_replaces_the_natural_one_when_a_side_is_free() {
    for display in ["block", "inline"] {
        assert_eq!(size("w", "width: 40px; aspect-ratio: 1", display), (40, 40));
        assert_eq!(
            size("h", "height: 30px; aspect-ratio: 1", display),
            (30, 30)
        );
    }
}

#[test]
fn natural_ratio_is_used_without_the_property() {
    for display in ["block", "inline"] {
        assert_eq!(size("n", "width: 40px", display), (40, 20));
    }
}

#[test]
fn auto_with_a_ratio_prefers_the_natural_ratio() {
    for display in ["block", "inline"] {
        assert_eq!(
            size("a", "width: 40px; aspect-ratio: auto 1", display),
            (40, 20)
        );
    }
}

#[test]
fn both_sides_set_ignore_the_ratio() {
    assert_eq!(
        size("b", "width: 40px; height: 10px; aspect-ratio: 1", "block"),
        (40, 10)
    );
}

#[test]
fn without_sizes_the_natural_width_is_kept_and_the_height_follows() {
    assert_eq!(size("u", "aspect-ratio: 1", "block"), (4, 4));
    assert_eq!(size("u2", "aspect-ratio: 1", "inline"), (4, 4));
}

#[test]
fn missing_picture_uses_the_ratio_with_a_given_side() {
    let page = Page::new(
        "<html><style>body { margin: 0 } img { display: block; width: 40px; aspect-ratio: 2; background: red }</style><body><img src=\"missing.png\"></body></html>",
    );
    let canvas = page.render(100, 100, 0.0).canvas;
    let painted = |x: usize, y: usize| canvas.pixel(x, y) != WHITE;
    assert_eq!((0..100).filter(|x| painted(*x, 0)).count(), 40);
    assert_eq!((0..100).filter(|y| painted(0, *y)).count(), 20);
}
