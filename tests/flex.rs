// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Color, Value};
use foundation::layout::{BoxKind, ImageMap, LayoutBox, Rect};
use foundation::{html, layout, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const GREEN: Color = Color { r: 0, g: 255, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn with_layout(source: &str, width: f32, check: impl FnOnce(&LayoutBox)) {
    let root = html::parse(source);
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width, 400.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let font = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &font, &ImageMap::new());
    check(&layout_root);
}

fn flex_page(css_text: &str, body_html: &str) -> String {
    format!(
        "<html><style>body {{ margin: 0 }} .c {{ display: flex }} .c > div {{ height: 10px }} {}</style><body>{}</body></html>",
        css_text, body_html
    )
}

fn container<'a, 'b>(root: &'b LayoutBox<'a>) -> &'b LayoutBox<'a> {
    &root.children[0].children[0]
}

fn boxes(root: &LayoutBox) -> Vec<Rect> {
    container(root)
        .children
        .iter()
        .map(|child| child.dimensions.border_box())
        .collect()
}

fn check_rects(actual: &[Rect], expected: &[(f32, f32, f32, f32)]) {
    assert_eq!(actual.len(), expected.len());
    for (index, (rect, want)) in actual.iter().zip(expected).enumerate() {
        let got = (rect.x, rect.y, rect.width, rect.height);
        assert!(
            (got.0 - want.0).abs() < 0.01
                && (got.1 - want.1).abs() < 0.01
                && (got.2 - want.2).abs() < 0.01
                && (got.3 - want.3).abs() < 0.01,
            "item {}: got {:?}, want {:?}",
            index,
            got,
            want
        );
    }
}

fn run(css_text: &str, body_html: &str, width: f32, expected: &[(f32, f32, f32, f32)]) {
    with_layout(&flex_page(css_text, body_html), width, |root| {
        check_rects(&boxes(root), expected);
    });
}

#[test]
fn css_expands_flex_shorthands() {
    let sheet = css::parse(
        ".a { flex: 1 } .b { flex: 2 3 10px } .c { flex: none } .d { flex: auto } .e { flex: 0 0 25% } .f { flex: 100px } .g { flex-flow: column wrap } .h { gap: 4px 8px } .i { gap: 5px } .j { flex: 1 2 3 4 } .k { flex: initial }",
    );
    let get = |rule: usize, name: &str| {
        sheet.rules[rule]
            .declarations
            .iter()
            .find(|d| d.name == name)
            .map(|d| d.value.clone())
    };
    assert_eq!(get(0, "flex-grow"), Some(Value::Number(1.0)));
    assert_eq!(get(0, "flex-shrink"), Some(Value::Number(1.0)));
    assert_eq!(get(0, "flex-basis"), Some(Value::Percent(0.0)));
    assert_eq!(get(1, "flex-grow"), Some(Value::Number(2.0)));
    assert_eq!(get(1, "flex-shrink"), Some(Value::Number(3.0)));
    assert_eq!(get(1, "flex-basis"), Some(Value::Length(10.0)));
    assert_eq!(get(2, "flex-grow"), Some(Value::Number(0.0)));
    assert_eq!(get(2, "flex-shrink"), Some(Value::Number(0.0)));
    assert_eq!(get(2, "flex-basis"), Some(Value::Keyword("auto".into())));
    assert_eq!(get(3, "flex-grow"), Some(Value::Number(1.0)));
    assert_eq!(get(4, "flex-grow"), Some(Value::Number(0.0)));
    assert_eq!(get(4, "flex-shrink"), Some(Value::Number(0.0)));
    assert_eq!(get(4, "flex-basis"), Some(Value::Percent(25.0)));
    assert_eq!(get(5, "flex-grow"), Some(Value::Number(1.0)));
    assert_eq!(get(5, "flex-basis"), Some(Value::Length(100.0)));
    assert_eq!(
        get(6, "flex-direction"),
        Some(Value::Keyword("column".into()))
    );
    assert_eq!(get(6, "flex-wrap"), Some(Value::Keyword("wrap".into())));
    assert_eq!(get(7, "row-gap"), Some(Value::Length(4.0)));
    assert_eq!(get(7, "column-gap"), Some(Value::Length(8.0)));
    assert_eq!(get(8, "row-gap"), Some(Value::Length(5.0)));
    assert_eq!(get(8, "column-gap"), Some(Value::Length(5.0)));
    assert_eq!(get(9, "flex-grow"), None);
    assert_eq!(get(10, "flex-grow"), Some(Value::Number(0.0)));
    assert_eq!(get(10, "flex-shrink"), Some(Value::Number(1.0)));
}

#[test]
fn items_flow_in_a_row_with_fixed_widths() {
    run(
        ".c > div { width: 30px }",
        "<div class=\"c\"><div></div><div></div><div></div></div>",
        200.0,
        &[
            (0.0, 0.0, 30.0, 10.0),
            (30.0, 0.0, 30.0, 10.0),
            (60.0, 0.0, 30.0, 10.0),
        ],
    );
}

#[test]
fn flex_grow_distributes_free_space_by_factor() {
    run(
        ".c > div { width: 20px } .g1 { flex-grow: 1 } .g2 { flex-grow: 2 }",
        "<div class=\"c\"><div class=\"g1\"></div><div class=\"g2\"></div><div></div></div>",
        200.0,
        &[
            (0.0, 0.0, 66.666_664, 10.0),
            (66.666_664, 0.0, 113.333_336, 10.0),
            (180.0, 0.0, 20.0, 10.0),
        ],
    );
}

#[test]
fn flex_shorthand_one_splits_equally() {
    run(
        ".c > div { flex: 1 }",
        "<div class=\"c\"><div></div><div></div><div></div></div>",
        90.0,
        &[
            (0.0, 0.0, 30.0, 10.0),
            (30.0, 0.0, 30.0, 10.0),
            (60.0, 0.0, 30.0, 10.0),
        ],
    );
}

#[test]
fn flex_shrink_reduces_items_proportionally_to_basis() {
    run(
        ".a { width: 100px } .b { width: 200px }",
        "<div class=\"c\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        150.0,
        &[(0.0, 0.0, 50.0, 10.0), (50.0, 0.0, 100.0, 10.0)],
    );
    run(
        ".a { width: 100px; flex-shrink: 0 } .b { width: 200px }",
        "<div class=\"c\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        150.0,
        &[(0.0, 0.0, 100.0, 10.0), (100.0, 0.0, 50.0, 10.0)],
    );
}

#[test]
fn min_and_max_width_freeze_flexible_items() {
    run(
        ".a { flex: 1; max-width: 20px } .b { flex: 1 }",
        "<div class=\"c\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        100.0,
        &[(0.0, 0.0, 20.0, 10.0), (20.0, 0.0, 80.0, 10.0)],
    );
    run(
        ".a { flex: 1; min-width: 70px } .b { flex: 1 }",
        "<div class=\"c\"><div class=\"a\"></div><div class=\"b\"></div></div>",
        100.0,
        &[(0.0, 0.0, 70.0, 10.0), (70.0, 0.0, 30.0, 10.0)],
    );
}

#[test]
fn flex_basis_overrides_width() {
    run(
        ".c > div { width: 10px; flex-basis: 40px }",
        "<div class=\"c\"><div></div><div></div></div>",
        200.0,
        &[(0.0, 0.0, 40.0, 10.0), (40.0, 0.0, 40.0, 10.0)],
    );
    run(
        ".c > div { flex: 0 0 25% }",
        "<div class=\"c\"><div></div><div></div></div>",
        200.0,
        &[(0.0, 0.0, 50.0, 10.0), (50.0, 0.0, 50.0, 10.0)],
    );
}

#[test]
fn content_size_is_used_without_width_or_basis() {
    let font = text::default_font();
    let first = text::text_width(&font, "ab", 16.0);
    let second = text::text_width(&font, "cde fg", 16.0);
    with_layout(
        &flex_page("", "<div class=\"c\"><div>ab</div><div>cde fg</div></div>"),
        400.0,
        |root| {
            let items = boxes(root);
            assert!((items[0].width - first).abs() < 0.01);
            assert!((items[1].width - second).abs() < 0.01);
            assert!((items[1].x - first).abs() < 0.01);
        },
    );
}

#[test]
fn justify_content_distributes_remaining_space() {
    let items = "<div class=\"c\"><div></div><div></div><div></div></div>";
    let base = ".c > div { width: 20px }";
    let cases: [(&str, [f32; 3]); 6] = [
        ("flex-start", [0.0, 20.0, 40.0]),
        ("flex-end", [40.0, 60.0, 80.0]),
        ("center", [20.0, 40.0, 60.0]),
        ("space-between", [0.0, 40.0, 80.0]),
        ("space-around", [6.666_667, 40.0, 73.333_33]),
        ("space-evenly", [10.0, 40.0, 70.0]),
    ];
    for (value, xs) in cases {
        let css_text = format!("{} .c {{ justify-content: {} }}", base, value);
        let expected: Vec<(f32, f32, f32, f32)> =
            xs.iter().map(|x| (*x, 0.0, 20.0, 10.0)).collect();
        run(&css_text, items, 100.0, &expected);
    }
}

#[test]
fn row_reverse_mirrors_the_main_axis() {
    run(
        ".c { flex-direction: row-reverse } .c > div { width: 20px }",
        "<div class=\"c\"><div></div><div></div></div>",
        100.0,
        &[(80.0, 0.0, 20.0, 10.0), (60.0, 0.0, 20.0, 10.0)],
    );
    run(
        ".c { flex-direction: row-reverse; justify-content: flex-end } .c > div { width: 20px }",
        "<div class=\"c\"><div></div><div></div></div>",
        100.0,
        &[(20.0, 0.0, 20.0, 10.0), (0.0, 0.0, 20.0, 10.0)],
    );
}

#[test]
fn order_changes_the_visual_sequence() {
    run(
        ".c > div { width: 20px } .first { order: -1 } .last { order: 5 }",
        "<div class=\"c\"><div class=\"last\"></div><div></div><div class=\"first\"></div></div>",
        100.0,
        &[
            (40.0, 0.0, 20.0, 10.0),
            (20.0, 0.0, 20.0, 10.0),
            (0.0, 0.0, 20.0, 10.0),
        ],
    );
}

#[test]
fn gap_separates_items_and_counts_in_the_free_space() {
    run(
        ".c { gap: 10px } .c > div { width: 20px }",
        "<div class=\"c\"><div></div><div></div><div></div></div>",
        200.0,
        &[
            (0.0, 0.0, 20.0, 10.0),
            (30.0, 0.0, 20.0, 10.0),
            (60.0, 0.0, 20.0, 10.0),
        ],
    );
    run(
        ".c { column-gap: 10px } .c > div { flex: 1 }",
        "<div class=\"c\"><div></div><div></div><div></div></div>",
        100.0,
        &[
            (0.0, 0.0, 26.666_666, 10.0),
            (36.666_668, 0.0, 26.666_666, 10.0),
            (73.333_336, 0.0, 26.666_666, 10.0),
        ],
    );
}

#[test]
fn auto_margins_absorb_free_space() {
    run(
        ".c > div { width: 20px } .m { margin-left: auto }",
        "<div class=\"c\"><div></div><div class=\"m\"></div></div>",
        100.0,
        &[(0.0, 0.0, 20.0, 10.0), (80.0, 0.0, 20.0, 10.0)],
    );
    run(
        ".c > div { width: 20px; margin: 0 auto }",
        "<div class=\"c\"><div></div></div>",
        100.0,
        &[(40.0, 0.0, 20.0, 10.0)],
    );
}

#[test]
fn align_items_positions_items_across_the_line() {
    let items = "<div class=\"c\"><div class=\"s\"></div><div class=\"t\" style=\"height: 40px\"></div></div>";
    let base = ".c { height: 100px } .c > div { width: 10px } .c > .s { height: 20px }";
    let cases: [(&str, f32, f32); 4] = [
        ("flex-start", 0.0, 0.0),
        ("flex-end", 80.0, 60.0),
        ("center", 40.0, 30.0),
        ("baseline", 0.0, 0.0),
    ];
    for (value, first_y, second_y) in cases {
        let css_text = format!("{} .c {{ align-items: {} }}", base, value);
        run(
            &css_text,
            items,
            100.0,
            &[(0.0, first_y, 10.0, 20.0), (10.0, second_y, 10.0, 40.0)],
        );
    }
}

#[test]
fn default_align_stretches_items_without_a_height() {
    run(
        ".c { height: 60px } .c > div { width: 10px; height: auto }",
        "<div class=\"c\"><div></div><div style=\"height: 20px\"></div></div>",
        100.0,
        &[(0.0, 0.0, 10.0, 60.0), (10.0, 0.0, 10.0, 20.0)],
    );
}

#[test]
fn container_height_follows_the_tallest_item() {
    with_layout(
        &flex_page(
            ".c > div { width: 10px; height: auto } .c > .t { height: 35px }",
            "<div class=\"c\"><div></div><div class=\"t\"></div></div><div class=\"after\"></div>",
        ),
        100.0,
        |root| {
            assert_eq!(container(root).dimensions.content.height, 35.0);
            let items = boxes(root);
            assert_eq!(items[0].height, 35.0);
            assert_eq!(root.children[0].children[1].dimensions.content.y, 35.0);
        },
    );
}

#[test]
fn align_self_overrides_align_items() {
    run(
        ".c { height: 50px; align-items: flex-start } .c > div { width: 10px; height: 10px } .e { align-self: flex-end } .m { align-self: center }",
        "<div class=\"c\"><div></div><div class=\"e\"></div><div class=\"m\"></div></div>",
        100.0,
        &[(0.0, 0.0, 10.0, 10.0), (10.0, 40.0, 10.0, 10.0), (20.0, 20.0, 10.0, 10.0)],
    );
}

#[test]
fn wrap_breaks_items_into_lines() {
    run(
        ".c { flex-wrap: wrap } .c > div { width: 40px }",
        "<div class=\"c\"><div></div><div></div><div></div><div></div><div></div></div>",
        100.0,
        &[
            (0.0, 0.0, 40.0, 10.0),
            (40.0, 0.0, 40.0, 10.0),
            (0.0, 10.0, 40.0, 10.0),
            (40.0, 10.0, 40.0, 10.0),
            (0.0, 20.0, 40.0, 10.0),
        ],
    );
}

#[test]
fn wrap_with_gaps_and_grow_fills_each_line() {
    run(
        ".c { flex-wrap: wrap; gap: 10px } .c > div { width: 40px; flex-grow: 1 }",
        "<div class=\"c\"><div></div><div></div><div></div></div>",
        100.0,
        &[
            (0.0, 0.0, 45.0, 10.0),
            (55.0, 0.0, 45.0, 10.0),
            (0.0, 20.0, 100.0, 10.0),
        ],
    );
}

#[test]
fn wrap_reverse_mirrors_the_cross_axis() {
    run(
        ".c { flex-wrap: wrap-reverse; height: 40px; align-content: flex-start } .c > div { width: 60px }",
        "<div class=\"c\"><div></div><div></div></div>",
        100.0,
        &[(0.0, 30.0, 60.0, 10.0), (0.0, 20.0, 60.0, 10.0)],
    );
}

#[test]
fn align_content_places_wrapped_lines() {
    let items = "<div class=\"c\"><div></div><div></div></div>";
    let base = ".c { flex-wrap: wrap; height: 100px } .c > div { width: 60px }";
    let cases: [(&str, [f32; 2]); 6] = [
        ("flex-start", [0.0, 10.0]),
        ("flex-end", [80.0, 90.0]),
        ("center", [40.0, 50.0]),
        ("space-between", [0.0, 90.0]),
        ("space-around", [20.0, 70.0]),
        ("space-evenly", [26.666_666, 63.333_332]),
    ];
    for (value, ys) in cases {
        let css_text = format!("{} .c {{ align-content: {} }}", base, value);
        run(
            &css_text,
            items,
            100.0,
            &[(0.0, ys[0], 60.0, 10.0), (0.0, ys[1], 60.0, 10.0)],
        );
    }
    run(
        &format!("{} .c > div {{ height: auto }}", base),
        items,
        100.0,
        &[(0.0, 0.0, 60.0, 50.0), (0.0, 50.0, 60.0, 50.0)],
    );
}

#[test]
fn column_direction_stacks_items_with_their_content_height() {
    run(
        ".c { flex-direction: column } .c > div { width: 30px }",
        "<div class=\"c\"><div></div><div style=\"height: 25px\"></div></div>",
        100.0,
        &[(0.0, 0.0, 30.0, 10.0), (0.0, 10.0, 30.0, 25.0)],
    );
}

#[test]
fn column_items_stretch_across_the_width() {
    run(
        ".c { flex-direction: column }",
        "<div class=\"c\"><div></div></div>",
        100.0,
        &[(0.0, 0.0, 100.0, 10.0)],
    );
    run(
        ".c { flex-direction: column; align-items: center } .c > div { width: 40px }",
        "<div class=\"c\"><div></div></div>",
        100.0,
        &[(30.0, 0.0, 40.0, 10.0)],
    );
}

#[test]
fn column_grow_uses_a_definite_height() {
    run(
        ".c { flex-direction: column; height: 100px } .c > div { height: auto; flex: 1 }",
        "<div class=\"c\"><div></div><div></div></div>",
        100.0,
        &[(0.0, 0.0, 100.0, 50.0), (0.0, 50.0, 100.0, 50.0)],
    );
    run(
        ".c { flex-direction: column; height: 100px; justify-content: flex-end } .c > div { width: 10px }",
        "<div class=\"c\"><div></div><div></div></div>",
        100.0,
        &[(0.0, 80.0, 10.0, 10.0), (0.0, 90.0, 10.0, 10.0)],
    );
    run(
        ".c { flex-direction: column-reverse; height: 100px } .c > div { width: 10px }",
        "<div class=\"c\"><div></div><div></div></div>",
        100.0,
        &[(0.0, 90.0, 10.0, 10.0), (0.0, 80.0, 10.0, 10.0)],
    );
}

#[test]
fn column_without_height_grows_the_container_and_ignores_grow() {
    with_layout(
        &flex_page(
            ".c { flex-direction: column; gap: 5px } .c > div { flex-grow: 1 }",
            "<div class=\"c\"><div></div><div></div></div>",
        ),
        100.0,
        |root| {
            assert_eq!(container(root).dimensions.content.height, 25.0);
            check_rects(
                &boxes(root),
                &[(0.0, 0.0, 100.0, 10.0), (0.0, 15.0, 100.0, 10.0)],
            );
        },
    );
}

#[test]
fn text_directly_inside_the_container_becomes_an_item() {
    let font = text::default_font();
    let width = text::text_width(&font, "hello", 16.0);
    with_layout(
        &flex_page(
            ".c { justify-content: flex-end }",
            "<div class=\"c\">hello</div>",
        ),
        100.0,
        |root| {
            let item = &container(root).children[0];
            assert!(matches!(item.kind, BoxKind::Anonymous(_)));
            assert!((item.fragments[0].x - (100.0 - width)).abs() < 0.01);
        },
    );
}

#[test]
fn item_margins_padding_and_borders_count_in_the_sizes() {
    run(
        ".c > div { width: 20px; padding: 5px; border-width: 2px; margin: 3px }",
        "<div class=\"c\"><div></div><div></div></div>",
        200.0,
        &[(3.0, 3.0, 34.0, 10.0 + 14.0), (43.0, 3.0, 34.0, 24.0)],
    );
}

#[test]
fn flex_container_takes_shrink_to_fit_width_when_floated() {
    with_layout(
        &flex_page(
            ".c { float: left; gap: 4px } .c > div { width: 10px }",
            "<div class=\"c\"><div></div><div></div></div>",
        ),
        200.0,
        |root| {
            assert_eq!(container(root).dimensions.content.width, 24.0);
        },
    );
}

#[test]
fn absolute_children_are_left_out_of_the_flex_line() {
    run(
        ".c { position: relative } .c > div { width: 20px } .c > .a { position: absolute; left: 50px; top: 5px }",
        "<div class=\"c\"><div class=\"a\"></div><div></div><div></div></div>",
        200.0,
        &[(50.0, 5.0, 20.0, 10.0), (0.0, 0.0, 20.0, 10.0), (20.0, 0.0, 20.0, 10.0)],
    );
}

#[test]
fn nested_flex_containers_lay_out_recursively() {
    with_layout(
        &flex_page(
            ".c > div { flex: 1 } .c > .in { display: flex; flex-direction: column; height: auto } .in > span { display: block; height: 7px }",
            "<div class=\"c\"><div class=\"in\"><span></span><span></span></div><div></div></div>",
        ),
        100.0,
        |root| {
            let outer = boxes(root);
            assert_eq!(outer[0].width, 50.0);
            assert_eq!(outer[0].height, 14.0);
            let inner = &container(root).children[0].children;
            assert_eq!(inner[1].dimensions.content.y, 7.0);
            assert_eq!(inner[1].dimensions.content.width, 50.0);
        },
    );
}

#[test]
fn flex_items_do_not_collapse_margins_with_their_children() {
    with_layout(
        &flex_page(
            ".c > div { width: 20px } .in { margin-top: 10px; height: 5px }",
            "<div class=\"c\"><div><div class=\"in\"></div></div></div>",
        ),
        100.0,
        |root| {
            let item = &container(root).children[0];
            assert_eq!(item.dimensions.margin.top, 0.0);
            assert_eq!(item.children[0].dimensions.content.y, 10.0);
        },
    );
}

#[test]
fn render_draws_flex_items_in_place() {
    let page = Page::new(
        "<html><style>body { margin: 0 } .c { display: flex; justify-content: space-between; align-items: center; height: 20px } .c > div { width: 5px; height: 5px } .r { background-color: red } .g { background-color: lime } .b { background-color: blue }</style><body><div class=\"c\"><div class=\"r\"></div><div class=\"g\"></div><div class=\"b\"></div></div></body></html>",
    );
    let canvas = page.render(30, 20, 0.0).canvas;
    assert_eq!(canvas.pixel(0, 8), RED);
    assert_eq!(canvas.pixel(4, 12), RED);
    assert_eq!(canvas.pixel(0, 2), WHITE);
    assert_eq!(canvas.pixel(14, 10), GREEN);
    assert_eq!(canvas.pixel(25, 10), BLUE);
    assert_eq!(canvas.pixel(29, 12), BLUE);
    assert_eq!(canvas.pixel(10, 10), WHITE);
}

#[test]
fn inline_children_of_a_flex_container_become_separate_items() {
    let font = text::default_font();
    let first = text::text_width(&font, "ab", 16.0) + 10.0;
    let second = text::text_width(&font, "cd", 16.0) + 10.0;
    with_layout(
        &flex_page(
            ".c { gap: 7px } .c > span { padding: 0 5px }",
            "<div class=\"c\"><span>ab</span><span>cd</span></div>",
        ),
        400.0,
        |root| {
            let items = boxes(root);
            assert_eq!(items.len(), 2);
            assert!((items[0].width - first).abs() < 0.01);
            assert!((items[1].width - second).abs() < 0.01);
            assert!((items[1].x - (first + 7.0)).abs() < 0.01);
        },
    );
}
