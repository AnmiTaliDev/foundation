// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text};

fn find(layout_box: &LayoutBox, tag: &str, found: &mut Vec<Rect>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if matches!(style.content, Content::Element(e) if e.tag_name == tag) {
            found.push(layout_box.dimensions.border_box());
        }
    }
    for child in &layout_box.children {
        find(child, tag, found);
    }
}

fn items(container: &str, item_style: &str, count: usize) -> Vec<Rect> {
    let children: String = (0..count)
        .map(|_| format!("<section style=\"{}\"></section>", item_style))
        .collect();
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ display: flex; width: 300px; {} }} section {{ width: 50px; height: 10px; flex: none }}</style></head><body><div id=a>{}</div></body></html>",
        container, children
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, "section", &mut found);
    found
}

fn xs(rects: &[Rect]) -> Vec<f32> {
    rects.iter().map(|rect| rect.x).collect()
}

#[test]
fn ltr_rows_start_at_the_left() {
    assert_eq!(xs(&items("", "", 3)), vec![0.0, 50.0, 100.0]);
}

#[test]
fn rtl_rows_start_at_the_right() {
    assert_eq!(
        xs(&items("direction: rtl", "", 3)),
        vec![250.0, 200.0, 150.0]
    );
}

#[test]
fn row_reverse_flips_back_in_rtl() {
    assert_eq!(
        xs(&items("direction: rtl; flex-direction: row-reverse", "", 3)),
        vec![0.0, 50.0, 100.0]
    );
    assert_eq!(
        xs(&items("flex-direction: row-reverse", "", 3)),
        vec![250.0, 200.0, 150.0]
    );
}

#[test]
fn justify_content_follows_the_direction() {
    assert_eq!(
        xs(&items("direction: rtl; justify-content: flex-end", "", 2)),
        vec![50.0, 0.0]
    );
    assert_eq!(
        xs(&items("direction: rtl; justify-content: center", "", 2)),
        vec![150.0, 100.0]
    );
}

#[test]
fn gaps_and_margins_are_kept() {
    let found = items("direction: rtl; gap: 10px", "", 3);
    assert_eq!(xs(&found), vec![250.0, 190.0, 130.0]);
    let found = items("direction: rtl", "margin-left: 10px", 2);
    assert_eq!(xs(&found), vec![250.0, 190.0]);
}

#[test]
fn space_between_is_symmetric() {
    let found = items("direction: rtl; justify-content: space-between", "", 3);
    assert_eq!(xs(&found), vec![250.0, 125.0, 0.0]);
}

#[test]
fn columns_align_to_the_cross_start() {
    let found = items(
        "direction: rtl; flex-direction: column; align-items: flex-start",
        "",
        2,
    );
    assert_eq!(xs(&found), vec![250.0, 250.0]);
    let found = items(
        "direction: rtl; flex-direction: column; align-items: flex-end",
        "",
        2,
    );
    assert_eq!(xs(&found), vec![0.0, 0.0]);
    let found = items("flex-direction: column; align-items: flex-start", "", 2);
    assert_eq!(xs(&found), vec![0.0, 0.0]);
}

#[test]
fn stretched_columns_fill_the_width() {
    let found = items("direction: rtl; flex-direction: column", "width: auto", 2);
    assert_eq!(xs(&found), vec![0.0, 0.0]);
    assert_eq!(found[0].width, 300.0);
}

#[test]
fn wrapped_lines_are_mirrored_one_by_one() {
    let found = items("direction: rtl; flex-wrap: wrap; width: 120px", "", 3);
    assert_eq!(xs(&found), vec![70.0, 20.0, 70.0]);
}

#[test]
fn item_content_is_not_flipped() {
    let found = items("direction: rtl", "text-align: left", 1);
    assert_eq!(found[0].x, 250.0);
    assert_eq!(found[0].width, 50.0);
}

#[test]
fn ltr_inside_rtl_flex_is_unaffected() {
    let root = html::parse(
        "<html><head><style>body { margin: 0 } #a { display: flex; direction: rtl; width: 300px } #b { display: flex; direction: ltr; width: 100px; flex: none } section { width: 20px; height: 10px; flex: none }</style></head><body><div id=a><div id=b><section></section><section></section></div></div></body></html>",
    );
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, "section", &mut found);
    assert_eq!(xs(&found), vec![200.0, 220.0]);
}
