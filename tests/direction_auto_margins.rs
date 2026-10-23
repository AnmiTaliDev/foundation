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

fn xs(container: &str, children: &str) -> Vec<f32> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} #a {{ width: 300px; {} }} section {{ width: 50px; height: 10px; flex: none }}</style></head><body><div id=a>{}</div></body></html>",
        container, children
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(500.0, 500.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, "section", &mut found);
    found.iter().map(|rect| rect.x).collect()
}

fn item(style: &str) -> String {
    format!("<section style=\"{}\"></section>", style)
}

#[test]
fn a_left_auto_margin_pushes_a_single_flex_item_right() {
    let direction = ["", "direction: rtl;"];
    for extra in direction {
        let found = xs(
            &format!("display: flex; {}", extra),
            &item("margin-left: auto"),
        );
        assert_eq!(found, vec![250.0], "{}", extra);
    }
}

#[test]
fn a_right_auto_margin_keeps_a_single_flex_item_left() {
    for extra in ["", "direction: rtl;"] {
        let found = xs(
            &format!("display: flex; {}", extra),
            &item("margin-right: auto"),
        );
        assert_eq!(found, vec![0.0], "{}", extra);
    }
}

#[test]
fn auto_margins_between_items_separate_them_in_both_directions() {
    let children = format!("{}{}", item("margin-left: auto"), item(""));
    assert_eq!(xs("display: flex", &children), vec![200.0, 250.0]);
    assert_eq!(
        xs("display: flex; direction: rtl", &children),
        vec![250.0, 0.0]
    );
}

#[test]
fn both_auto_margins_center_the_item() {
    for extra in ["", "direction: rtl;"] {
        let found = xs(
            &format!("display: flex; {}", extra),
            &item("margin: 0 auto"),
        );
        assert_eq!(found, vec![125.0], "{}", extra);
    }
}

#[test]
fn grid_items_keep_physical_auto_margins() {
    let column = "grid-template-columns: 100px;";
    let left = item("width: 40px; margin-left: auto");
    let right = item("width: 40px; margin-right: auto");
    assert_eq!(xs(&format!("display: grid; {}", column), &left), vec![60.0]);
    assert_eq!(
        xs(&format!("display: grid; direction: rtl; {}", column), &left),
        vec![260.0]
    );
    assert_eq!(xs(&format!("display: grid; {}", column), &right), vec![0.0]);
    assert_eq!(
        xs(
            &format!("display: grid; direction: rtl; {}", column),
            &right
        ),
        vec![200.0]
    );
}

#[test]
fn fixed_physical_margins_stay_on_their_sides() {
    let found = xs("display: flex; direction: rtl", &item("margin-left: 10px"));
    assert_eq!(found, vec![250.0]);
    let found = xs("display: flex; direction: rtl", &item("margin-right: 10px"));
    assert_eq!(found, vec![240.0]);
}

#[test]
fn cross_axis_auto_margins_in_columns_keep_their_sides() {
    let column = "display: flex; flex-direction: column;";
    let left = item("margin-left: auto");
    let right = item("margin-right: auto");
    assert_eq!(xs(column, &left), vec![250.0]);
    assert_eq!(
        xs(&format!("{} direction: rtl", column), &left),
        vec![250.0]
    );
    assert_eq!(xs(column, &right), vec![0.0]);
    assert_eq!(xs(&format!("{} direction: rtl", column), &right), vec![0.0]);
}
