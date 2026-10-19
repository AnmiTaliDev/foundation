// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::{self, Value};
use foundation::gridspec::{parse_areas, parse_line, parse_tracks, Line, Size, Track};
use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text};

fn rects(body: &str, tag: &str, width: f32) -> Vec<Rect> {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} section {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut found = Vec::new();
    find(&layout_root, tag, &mut found);
    found
}

fn find(layout_box: &LayoutBox, tag: &str, found: &mut Vec<Rect>) {
    if let BoxKind::Block(style) = &layout_box.kind {
        if let Content::Element(element) = style.content {
            if element.tag_name == tag {
                found.push(layout_box.dimensions.border_box());
            }
        }
    }
    for child in &layout_box.children {
        find(child, tag, found);
    }
}

fn items(container_style: &str, count: usize) -> Vec<Rect> {
    items_with(container_style, &vec![""; count], 400.0)
}

fn items_with(container_style: &str, item_styles: &[&str], width: f32) -> Vec<Rect> {
    let mut body = format!("<div style=\"display:grid;{}\">", container_style);
    for style in item_styles {
        body.push_str(&format!(
            "<section style=\"height:20px;{}\"></section>",
            style
        ));
    }
    body.push_str("</div>");
    rects(&body, "section", width)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn fixed_tracks_and_gaps() {
    let cells = items("grid-template-columns:100px 50px; gap:10px 20px", 4);
    assert!(close(cells[0].x, 0.0) && close(cells[0].width, 100.0));
    assert!(close(cells[1].x, 120.0) && close(cells[1].width, 50.0));
    assert!(close(cells[2].x, 0.0) && close(cells[2].y, 30.0));
    assert!(close(cells[3].x, 120.0) && close(cells[3].y, 30.0));
}

#[test]
fn fr_tracks_share_the_free_space() {
    let cells = items("grid-template-columns:1fr 2fr", 2);
    assert!(close(cells[0].width, 400.0 / 3.0));
    assert!(close(cells[1].width, 800.0 / 3.0));
    let cells = items("grid-template-columns:1fr 2fr; column-gap:30px", 2);
    assert!(close(cells[0].width, 370.0 / 3.0));
    assert!(close(cells[1].x, 370.0 / 3.0 + 30.0));
    let cells = items("grid-template-columns:100px 1fr", 2);
    assert!(close(cells[1].width, 300.0));
    let cells = items("grid-template-columns:repeat(3, 1fr)", 3);
    assert!(close(cells[1].x, 400.0 / 3.0));
}

#[test]
fn percentages_and_em_units() {
    let cells = items("grid-template-columns:25% 5em", 2);
    assert!(close(cells[0].width, 100.0));
    assert!(close(cells[1].width, 80.0));
}

#[test]
fn auto_tracks_take_their_content_width() {
    let body = "<div style=\"display:grid;grid-template-columns:auto 1fr\"><section>hello</section><section></section></div>";
    let cells = rects(body, "section", 400.0);
    let word = text::text_width(&text::default_font(), "hello", 16.0);
    assert!(close(cells[0].width, word));
    assert!(close(cells[1].width, 400.0 - word));
}

#[test]
fn auto_tracks_share_leftover_space_without_fr() {
    let cells = items("grid-template-columns:100px auto auto", 3);
    assert!(close(cells[1].width, 150.0));
    assert!(close(cells[2].width, 150.0));
}

#[test]
fn minmax_clamps_between_its_bounds() {
    let cells = items(
        "grid-template-columns:minmax(100px, 1fr) minmax(50px, 80px)",
        2,
    );
    assert!(close(cells[1].width, 80.0));
    assert!(close(cells[0].width, 320.0));
    let narrow = items_with(
        "grid-template-columns:minmax(100px, 1fr) minmax(100px, 1fr)",
        &["", ""],
        150.0,
    );
    assert!(close(narrow[0].width, 100.0));
    assert!(close(narrow[1].x, 100.0));
}

#[test]
fn auto_fill_counts_tracks_that_fit() {
    let cells = items("grid-template-columns:repeat(auto-fill, 50px); gap:5px", 4);
    assert!(close(cells[0].width, 50.0));
    assert!(close(cells[1].x, 55.0));
    assert!(close(cells[3].x, 165.0));
    let cells = items_with(
        "grid-template-columns:repeat(auto-fill, 50px); gap:5px",
        &["", "", "", ""],
        175.0,
    );
    assert!(close(cells[3].x, 0.0));
    assert!(cells[3].y > 0.0);
}

#[test]
fn auto_fit_collapses_empty_tracks() {
    let fit = items_with(
        "grid-template-columns:repeat(auto-fit, minmax(50px, 1fr))",
        &["", ""],
        200.0,
    );
    assert!(close(fit[0].width, 100.0));
    assert!(close(fit[1].x, 100.0));
    let fill = items_with(
        "grid-template-columns:repeat(auto-fill, minmax(50px, 1fr))",
        &["", ""],
        200.0,
    );
    assert!(close(fill[0].width, 50.0));
}

#[test]
fn items_auto_place_row_by_row() {
    let cells = items("grid-template-columns:100px 100px", 5);
    assert!(close(cells[2].x, 0.0) && close(cells[2].y, 20.0));
    assert!(close(cells[3].x, 100.0) && close(cells[3].y, 20.0));
    assert!(close(cells[4].x, 0.0) && close(cells[4].y, 40.0));
}

#[test]
fn implicit_tracks_follow_grid_auto() {
    let cells = items_with(
        "grid-template-columns:100px; grid-auto-rows:30px",
        &["height:auto", "height:auto"],
        400.0,
    );
    assert!(close(cells[0].height, 30.0));
    assert!(close(cells[1].y, 30.0));
    let cells = items("grid-auto-flow:column; grid-auto-columns:40px", 3);
    assert!(close(cells[0].width, 40.0));
    assert!(close(cells[2].x, 80.0));
    let cells = items("grid-template-columns:50px; grid-auto-columns:30px 60px", 1);
    assert!(close(cells[0].width, 50.0));
}

#[test]
fn explicit_lines_and_spans() {
    let cells = items_with(
        "grid-template-columns:repeat(4, 50px)",
        &[
            "grid-column:2",
            "grid-column:1 / 3",
            "grid-column:span 2",
            "grid-column:-2 / -1",
            "grid-column-start:4; grid-column-end:5",
            "grid-column:2 / span 2",
        ],
        400.0,
    );
    assert!(close(cells[0].x, 50.0) && close(cells[0].width, 50.0));
    assert!(close(cells[1].x, 0.0) && close(cells[1].width, 100.0));
    assert!(close(cells[2].width, 100.0));
    assert!(close(cells[3].x, 150.0) && close(cells[3].width, 50.0));
    assert!(close(cells[4].x, 150.0));
    assert!(close(cells[5].x, 50.0) && close(cells[5].width, 100.0));
}

#[test]
fn rows_can_be_placed_and_spanned() {
    let cells = items_with(
        "grid-template-columns:50px 50px; grid-template-rows:30px 30px 30px",
        &[
            "height:auto; grid-row:2 / 4",
            "grid-row:1",
            "grid-row:3; grid-column:2",
        ],
        400.0,
    );
    assert!(close(cells[0].y, 30.0) && close(cells[0].height, 60.0));
    assert!(close(cells[1].y, 0.0));
    assert!(close(cells[2].y, 60.0) && close(cells[2].x, 50.0));
}

#[test]
fn named_areas_position_items() {
    let body = "<div style=\"display:grid;grid-template-areas:'h h' 's m' 'f f';grid-template-columns:60px 1fr;grid-template-rows:10px 20px 30px\"><section style=\"grid-area:f\"></section><section style=\"grid-area:h\"></section><section style=\"grid-area:s\"></section><section style=\"grid-area:m\"></section></div>";
    let cells = rects(body, "section", 400.0);
    assert!(
        close(cells[0].y, 30.0) && close(cells[0].width, 400.0) && close(cells[0].height, 30.0)
    );
    assert!(close(cells[1].y, 0.0) && close(cells[1].width, 400.0));
    assert!(close(cells[2].x, 0.0) && close(cells[2].width, 60.0) && close(cells[2].y, 10.0));
    assert!(close(cells[3].x, 60.0) && close(cells[3].width, 340.0));
}

#[test]
fn grid_area_with_numbers() {
    let cells = items_with(
        "grid-template-columns:repeat(3, 50px); grid-template-rows:repeat(3, 20px)",
        &["height:auto; grid-area:2 / 2 / 4 / 4"],
        400.0,
    );
    assert!(close(cells[0].x, 50.0) && close(cells[0].y, 20.0));
    assert!(close(cells[0].width, 100.0) && close(cells[0].height, 40.0));
}

#[test]
fn column_flow_fills_columns_first() {
    let cells = items("grid-auto-flow:column; grid-template-rows:20px 20px", 5);
    assert!(close(cells[0].y, 0.0) && close(cells[1].y, 20.0));
    assert!(close(cells[2].y, 0.0));
    assert!(cells[2].x > cells[0].x);
    assert!(cells[4].x > cells[2].x);
}

#[test]
fn dense_packing_fills_earlier_gaps() {
    let styles = ["grid-column:span 2", "grid-column:span 2", ""];
    let sparse = items_with("grid-template-columns:repeat(3, 50px)", &styles, 400.0);
    assert!(sparse[2].y > 0.0);
    let dense = items_with(
        "grid-template-columns:repeat(3, 50px); grid-auto-flow:row dense",
        &styles,
        400.0,
    );
    assert!(close(dense[2].y, 0.0) && close(dense[2].x, 100.0));
}

#[test]
fn rows_size_to_their_tallest_item() {
    let cells = items_with(
        "grid-template-columns:50px 50px",
        &["height:50px", "height:30px", "height:10px"],
        400.0,
    );
    assert!(close(cells[0].height, 50.0) && close(cells[1].height, 30.0));
    assert!(close(cells[2].y, 50.0));
}

#[test]
fn items_stretch_to_the_area_unless_aligned() {
    let body = "<div style=\"display:grid;grid-template-columns:100px 100px;grid-template-rows:60px\"><section></section><section style=\"height:20px\"></section></div>";
    let cells = rects(body, "section", 400.0);
    assert!(close(cells[0].height, 60.0));
    assert!(close(cells[1].height, 20.0));
}

fn sized(container: &str, item: &str) -> Rect {
    let body = format!(
        "<div style=\"display:grid;grid-template-columns:200px;grid-template-rows:100px;{}\"><section style=\"width:50px;height:20px;{}\"></section></div>",
        container, item
    );
    rects(&body, "section", 400.0)[0]
}

#[test]
fn alignment_places_items_inside_their_area() {
    let start = sized("", "");
    assert!(close(start.x, 0.0) && close(start.y, 0.0));
    let center = sized("justify-items:center; align-items:center", "");
    assert!(close(center.x, 75.0) && close(center.y, 40.0));
    let end = sized("justify-items:end; align-items:end", "");
    assert!(close(end.x, 150.0) && close(end.y, 80.0));
    let own = sized("", "justify-self:end; align-self:center");
    assert!(close(own.x, 150.0) && close(own.y, 40.0));
    let place = sized("place-items:center end", "");
    assert!(close(place.x, 150.0) && close(place.y, 40.0));
}

#[test]
fn auto_margins_center_items() {
    let item = sized("", "margin:auto");
    assert!(close(item.x, 75.0) && close(item.y, 40.0));
    let item = sized("", "margin-left:auto");
    assert!(close(item.x, 150.0));
}

#[test]
fn shrink_to_fit_when_not_stretched() {
    let body = "<div style=\"display:grid;grid-template-columns:200px;justify-items:start\"><section>abc</section></div>";
    let cells = rects(body, "section", 400.0);
    assert!(close(
        cells[0].width,
        text::text_width(&text::default_font(), "abc", 16.0)
    ));
    let body =
        "<div style=\"display:grid;grid-template-columns:200px\"><section>abc</section></div>";
    let cells = rects(body, "section", 400.0);
    assert!(close(cells[0].width, 200.0));
}

#[test]
fn justify_content_distributes_tracks() {
    let cells = items("grid-template-columns:50px 50px; justify-content:center", 2);
    assert!(close(cells[0].x, 150.0));
    let cells = items("grid-template-columns:50px 50px; justify-content:end", 2);
    assert!(close(cells[1].x, 350.0));
    let cells = items(
        "grid-template-columns:50px 50px; justify-content:space-between",
        2,
    );
    assert!(close(cells[0].x, 0.0) && close(cells[1].x, 350.0));
    let cells = items(
        "grid-template-columns:50px 50px; justify-content:space-evenly",
        2,
    );
    assert!(close(cells[0].x, 100.0) && close(cells[1].x, 250.0));
    let cells = items(
        "grid-template-columns:50px 50px; justify-content:space-around",
        2,
    );
    assert!(close(cells[0].x, 75.0) && close(cells[1].x, 275.0));
}

#[test]
fn a_definite_height_shares_fr_rows() {
    let cells = items_with(
        "grid-template-rows:1fr 3fr; height:200px",
        &["height:auto", "height:auto"],
        400.0,
    );
    assert!(close(cells[0].height, 50.0));
    assert!(close(cells[1].y, 50.0) && close(cells[1].height, 150.0));
}

#[test]
fn the_container_height_follows_its_rows() {
    let body = "<div style=\"display:grid;grid-template-columns:50px;gap:10px\"><section style=\"height:20px\"></section><section style=\"height:30px\"></section></div>";
    let boxes = rects(body, "div", 400.0);
    assert!(close(boxes[0].height, 60.0));
}

#[test]
fn legacy_gap_names_and_alias() {
    let cells = items("grid-template-columns:50px 50px; grid-gap:10px", 4);
    assert!(close(cells[1].x, 60.0) && close(cells[2].y, 30.0));
    let cells = items(
        "grid-template-columns:50px 50px; grid-column-gap:7px; grid-row-gap:3px",
        4,
    );
    assert!(close(cells[1].x, 57.0) && close(cells[2].y, 23.0));
}

#[test]
fn order_changes_auto_placement() {
    let cells = items_with(
        "grid-template-columns:100px 100px",
        &["order:2", "order:1"],
        400.0,
    );
    assert!(close(cells[1].x, 0.0));
    assert!(close(cells[0].x, 100.0));
}

#[test]
fn absolute_children_do_not_take_part() {
    let cells = items_with(
        "grid-template-columns:100px 100px; position:relative",
        &["position:absolute", ""],
        400.0,
    );
    assert!(close(cells[1].x, 0.0));
}

#[test]
fn text_runs_become_anonymous_items() {
    let body = "<div style=\"display:grid;grid-template-columns:100px 100px\">text<section style=\"height:20px\"></section></div>";
    let cells = rects(body, "section", 400.0);
    assert!(close(cells[0].x, 100.0));
}

#[test]
fn grids_nest_and_size_by_content() {
    let body = "<div style=\"display:grid;grid-template-columns:1fr 1fr\"><div style=\"display:grid;grid-template-columns:30px 30px\"><section style=\"height:10px\"></section><section style=\"height:10px\"></section></div><section style=\"height:10px\"></section></div>";
    let cells = rects(body, "section", 400.0);
    assert!(close(cells[1].x, 30.0));
    assert!(close(cells[2].x, 200.0));
}

#[test]
fn a_floated_grid_shrinks_to_its_tracks() {
    let body = "<div style=\"float:left;display:grid;grid-template-columns:30px 40px;gap:5px\"><section style=\"height:10px\"></section><section style=\"height:10px\"></section></div>";
    let boxes = rects(body, "div", 400.0);
    assert!(close(boxes[0].width, 75.0));
}

#[test]
fn track_lists_are_parsed() {
    let list =
        parse_tracks("100px minmax(10px, 1fr) [line] repeat(2, auto 20%) min-content").unwrap();
    assert_eq!(list.tracks.len(), 7);
    assert_eq!(
        list.tracks[0],
        Track {
            min: Size::Px(100.0),
            max: Size::Px(100.0)
        }
    );
    assert_eq!(
        list.tracks[1],
        Track {
            min: Size::Px(10.0),
            max: Size::Fr(1.0)
        }
    );
    assert_eq!(
        list.tracks[2],
        Track {
            min: Size::Auto,
            max: Size::Auto
        }
    );
    assert_eq!(
        list.tracks[6],
        Track {
            min: Size::MinContent,
            max: Size::MinContent
        }
    );
    let repeat = parse_tracks("50px repeat(auto-fit, 100px) 1fr").unwrap();
    let auto = repeat.repeat.unwrap();
    assert_eq!((auto.at, auto.fit, auto.tracks.len()), (1, true, 1));
    assert_eq!(repeat.tracks.len(), 2);
    assert!(parse_tracks("").is_none());
    assert!(parse_tracks("minmax(1fr, 2fr)").is_none());
    assert!(parse_tracks("repeat(0, 1fr)").is_none());
    assert!(parse_tracks("repeat(100000, 1fr)").is_none());
    assert!(parse_tracks("banana").is_none());
}

#[test]
fn lines_and_areas_are_parsed() {
    assert_eq!(parse_line("auto"), Some(Line::Auto));
    assert_eq!(parse_line("3"), Some(Line::Number(3)));
    assert_eq!(parse_line("-1"), Some(Line::Number(-1)));
    assert_eq!(parse_line("span 2"), Some(Line::Span(2)));
    assert_eq!(parse_line("span"), Some(Line::Span(1)));
    assert_eq!(parse_line("0"), None);
    assert_eq!(parse_line("Header"), Some(Line::Area("header".to_string())));
    let areas = parse_areas("\"a b\" 'c .'").unwrap();
    assert_eq!(areas, vec![vec!["a", "b"], vec!["c", "."]]);
    assert!(parse_areas("\"a b\" \"c\"").is_none());
    assert!(parse_areas("nothing").is_none());
}

#[test]
fn grid_declarations_expand() {
    let shorthand = css::parse_declarations("grid-column: 1 / span 2");
    assert_eq!(shorthand[0].name, "grid-column-start");
    assert_eq!(shorthand[0].value, Value::GridLine(Line::Number(1)));
    assert_eq!(shorthand[1].value, Value::GridLine(Line::Span(2)));
    let area = css::parse_declarations("grid-area: main");
    assert_eq!(area.len(), 4);
    let flow = css::parse_declarations("grid-auto-flow: column dense");
    assert_eq!(flow.len(), 2);
    let place = css::parse_declarations("place-items: center");
    assert_eq!(place.len(), 2);
}
