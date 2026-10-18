// SPDX-License-Identifier: GPL-3.0-only

use std::fs;
use std::path::{Path, PathBuf};

use foundation::css::{self, Value};
use foundation::layout::{self, BoxKind, LayoutBox};
use foundation::{html, style, text, Page};

fn faces(body: &str) -> Vec<(String, usize)> {
    let root = html::parse(&format!(
        "<html><style>body {{ margin: 0 }}</style><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(800.0, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    collect(&layout_root, &mut found);
    found
}

fn collect(layout_box: &LayoutBox, found: &mut Vec<(String, usize)>) {
    if let BoxKind::Anonymous(_) = layout_box.kind {
        for fragment in &layout_box.fragments {
            found.push((fragment.text.clone(), fragment.face));
        }
    }
    for child in &layout_box.children {
        collect(child, found);
    }
}

fn face_of(body: &str) -> usize {
    faces(body)[0].1
}

#[test]
fn family_lists_are_parsed() {
    assert_eq!(
        css::parse_declarations("font-family: \"Open Sans\", 'x, y', Arial, sans-serif")[0].value,
        Value::Families(vec![
            "Open Sans".to_string(),
            "x, y".to_string(),
            "Arial".to_string(),
            "sans-serif".to_string()
        ])
    );
    assert!(css::parse_declarations("font-family: a,,b").is_empty());
}

#[test]
fn font_shorthand_sets_style_weight_size_and_family() {
    let declarations = css::parse_declarations("font: italic bold 12px/20px Georgia, serif");
    let find = |name: &str| {
        declarations
            .iter()
            .find(|d| d.name == name)
            .map(|d| &d.value)
    };
    assert_eq!(
        find("font-style"),
        Some(&Value::Keyword("italic".to_string()))
    );
    assert_eq!(
        find("font-weight"),
        Some(&Value::Keyword("bold".to_string()))
    );
    assert_eq!(find("font-size"), Some(&Value::Length(12.0)));
    assert_eq!(find("line-height"), Some(&Value::Length(20.0)));
    assert_eq!(
        find("font-family"),
        Some(&Value::Families(vec![
            "Georgia".to_string(),
            "serif".to_string()
        ]))
    );
}

#[test]
fn default_text_uses_the_regular_sans_face() {
    assert_eq!(face_of("<p>a</p>"), 0);
}

#[test]
fn bold_and_italic_select_matching_faces() {
    assert_eq!(face_of("<p><b>a</b></p>"), 1);
    assert_eq!(face_of("<p><i>a</i></p>"), 2);
    assert_eq!(face_of("<p><b><i>a</i></b></p>"), 3);
    assert_eq!(face_of("<h1>a</h1>"), 1);
    assert_eq!(face_of("<p style=\"font-weight: 700\">a</p>"), 1);
    assert_eq!(face_of("<p style=\"font-weight: 400\">a</p>"), 0);
    assert_eq!(face_of("<p style=\"font-style: italic\">a</p>"), 2);
}

#[test]
fn font_properties_are_inherited() {
    assert_eq!(
        face_of("<div style=\"font-family: serif; font-weight: bold\"><p><span>a</span></p></div>"),
        5
    );
}

#[test]
fn families_map_to_the_bundled_faces() {
    assert_eq!(face_of("<p style=\"font-family: serif\">a</p>"), 4);
    assert_eq!(face_of("<p style=\"font-family: Georgia, Times\">a</p>"), 4);
    assert_eq!(face_of("<p style=\"font-family: monospace\">a</p>"), 8);
    assert_eq!(face_of("<p style=\"font-family: 'Courier New'\">a</p>"), 8);
    assert_eq!(face_of("<code>a</code>"), 8);
    assert_eq!(
        face_of("<p style=\"font-family: Unknown, monospace\">a</p>"),
        8
    );
    assert_eq!(face_of("<p style=\"font-family: Unknown\">a</p>"), 0);
    assert_eq!(
        face_of("<p style=\"font: italic bold 12px Courier\">a</p>"),
        11
    );
}

#[test]
fn faces_differ_in_width() {
    let fonts = text::default_font();
    let mono = fonts.select(&["monospace".to_string()], false, false);
    assert_eq!(fonts.width(mono, "i", 16.0), fonts.width(mono, "W", 16.0));
    assert_ne!(fonts.width(0, "i", 16.0), fonts.width(0, "W", 16.0));
    assert!(fonts.width(1, "abc", 16.0) > fonts.width(0, "abc", 16.0));
}

fn temp_dir(name: &str) -> PathBuf {
    let directory =
        std::env::temp_dir().join(format!("foundation-font-{}-{}", name, std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    fs::copy(
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/DejaVuSerif.ttf"),
        directory.join("web.ttf"),
    )
    .unwrap();
    directory
}

fn render_page(
    directory: &Path,
    css: &str,
    family: &str,
) -> (foundation::paint::Canvas, Vec<String>) {
    fs::write(directory.join("a.css"), css).unwrap();
    fs::write(
        directory.join("index.html"),
        format!(
            "<html><head><link rel=\"stylesheet\" href=\"a.css\"></head><body><p style=\"font-family: {}\">Hello world</p></body></html>",
            family
        ),
    )
    .unwrap();
    let page = Page::open(directory.join("index.html").to_str().unwrap()).unwrap();
    let canvas = page.render(200, 60, 0.0).canvas;
    (canvas, page.warnings.clone())
}

#[test]
fn font_face_loads_a_font_file_and_matches_by_family() {
    let directory = temp_dir("load");
    let (web, warnings) = render_page(
        &directory,
        "@font-face { font-family: \"Web Face\"; src: url(web.ttf) format(\"truetype\") }",
        "'Web Face'",
    );
    assert!(warnings.is_empty(), "{:?}", warnings);
    let (serif, _) = render_page(&directory, "", "serif");
    let (sans, _) = render_page(&directory, "", "sans-serif");
    assert_eq!(web, serif);
    assert_ne!(web, sans);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn font_face_skips_unsupported_formats_and_uses_the_next_source() {
    let directory = temp_dir("formats");
    let (web, warnings) = render_page(
        &directory,
        "@font-face { font-family: W; src: url(a.woff2) format('woff2'), url(web.ttf) format('truetype') }",
        "W",
    );
    assert!(warnings.is_empty(), "{:?}", warnings);
    let (serif, _) = render_page(&directory, "", "serif");
    assert_eq!(web, serif);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn font_face_picks_the_face_by_weight() {
    let directory = temp_dir("weight");
    let css = "@font-face { font-family: W; src: url(web.ttf) } @font-face { font-family: W; font-weight: bold; src: url(missing.ttf) } p { font-weight: bold }";
    let (_, warnings) = render_page(&directory, css, "W");
    assert_eq!(warnings.len(), 1, "{:?}", warnings);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn broken_fonts_are_warnings_and_fall_back() {
    let directory = temp_dir("broken");
    fs::write(directory.join("bad.ttf"), b"not a font").unwrap();
    let (canvas, warnings) = render_page(
        &directory,
        "@font-face { font-family: W; src: url(bad.ttf) } @font-face { font-family: V; src: url(none.ttf) }",
        "W, V",
    );
    assert_eq!(warnings.len(), 2, "{:?}", warnings);
    let (sans, _) = render_page(&directory, "", "sans-serif");
    assert_eq!(canvas, sans);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn font_face_rules_do_not_leak_into_the_cascade() {
    let sheet = css::parse("@font-face { font-family: W; src: url(a.ttf) } p { x: 1px }");
    assert_eq!(sheet.rules.len(), 1);
    assert_eq!(sheet.font_faces.len(), 1);
    assert_eq!(sheet.font_faces[0].family, "W");
    assert_eq!(sheet.font_faces[0].sources, ["a.ttf"]);
}
