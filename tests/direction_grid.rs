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

fn cells(container: &str, children: &str) -> Vec<Rect> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ display: grid; width: 300px; {} }} section {{ height: 10px }}</style></head><body><div id=a>{}</div></body></html>",
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

fn sections(count: usize) -> String {
    "<section></section>".repeat(count)
}

fn xs(rects: &[Rect]) -> Vec<f32> {
    rects.iter().map(|rect| rect.x).collect()
}

#[test]
fn ltr_columns_start_at_the_left() {
    let found = cells("grid-template-columns: 100px 100px 100px", &sections(3));
    assert_eq!(xs(&found), vec![0.0, 100.0, 200.0]);
}

#[test]
fn rtl_columns_start_at_the_right() {
    let found = cells(
        "direction: rtl; grid-template-columns: 100px 100px 100px",
        &sections(3),
    );
    assert_eq!(xs(&found), vec![200.0, 100.0, 0.0]);
}

#[test]
fn rows_keep_their_order() {
    let found = cells(
        "direction: rtl; grid-template-columns: 150px 150px",
        &sections(4),
    );
    let ys: Vec<f32> = found.iter().map(|rect| rect.y).collect();
    assert_eq!(xs(&found), vec![150.0, 0.0, 150.0, 0.0]);
    assert_eq!(ys, vec![0.0, 0.0, 10.0, 10.0]);
}

#[test]
fn gaps_are_kept() {
    let found = cells(
        "direction: rtl; grid-template-columns: 100px 100px; column-gap: 20px",
        &sections(2),
    );
    assert_eq!(xs(&found), vec![200.0, 80.0]);
}

#[test]
fn spanning_items_are_mirrored_as_a_unit() {
    let children = "<section style=\"grid-column: span 2\"></section><section></section>";
    let found = cells(
        "direction: rtl; grid-template-columns: 100px 100px 100px",
        children,
    );
    assert_eq!(xs(&found), vec![100.0, 0.0]);
    assert_eq!(found[0].width, 200.0);
}

#[test]
fn tracks_narrower_than_the_container_align_to_the_right() {
    let found = cells(
        "direction: rtl; grid-template-columns: 50px 50px",
        &sections(2),
    );
    assert_eq!(xs(&found), vec![250.0, 200.0]);
}

#[test]
fn justify_content_end_goes_left_in_rtl() {
    let found = cells(
        "direction: rtl; grid-template-columns: 50px 50px; justify-content: end",
        &sections(2),
    );
    assert_eq!(xs(&found), vec![50.0, 0.0]);
}

#[test]
fn item_alignment_follows_the_direction() {
    let found = cells(
        "direction: rtl; grid-template-columns: 100px; justify-items: start",
        "<section style=\"width: 40px\"></section>",
    );
    assert_eq!(xs(&found), vec![260.0]);
}

#[test]
fn explicit_column_lines_are_mirrored() {
    let children =
        "<section style=\"grid-column: 3\"></section><section style=\"grid-column: 1\"></section>";
    let found = cells(
        "direction: rtl; grid-template-columns: 100px 100px 100px",
        children,
    );
    assert_eq!(xs(&found), vec![0.0, 200.0]);
}

#[test]
fn fr_columns_fill_the_width() {
    let found = cells(
        "direction: rtl; grid-template-columns: 1fr 2fr",
        &sections(2),
    );
    assert_eq!(xs(&found), vec![200.0, 0.0]);
    assert_eq!(found[0].width, 100.0);
    assert_eq!(found[1].width, 200.0);
}
