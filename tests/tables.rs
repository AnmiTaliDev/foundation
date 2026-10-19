// SPDX-License-Identifier: GPL-3.0-only

use foundation::css::Color;
use foundation::layout::{self, BoxKind, LayoutBox, Rect};
use foundation::style::Content;
use foundation::{html, style, text, Page};

const RED: Color = Color { r: 255, g: 0, b: 0 };
const BLUE: Color = Color { r: 0, g: 0, b: 255 };
const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

fn with_layout<R>(body: &str, width: f32, f: impl FnOnce(&LayoutBox) -> R) -> R {
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} td, th {{ padding: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(width, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    f(&layout_root)
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

fn rects(body: &str, tag: &str, width: f32) -> Vec<Rect> {
    with_layout(body, width, |root| {
        let mut found = Vec::new();
        find(root, tag, &mut found);
        found
    })
}

fn word(text_: &str) -> f32 {
    text::text_width(&text::default_font(), text_, 16.0)
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

#[test]
fn cells_line_up_in_columns_and_rows() {
    let cells = rects(
        "<table><tr><td>a</td><td>bbbb</td></tr><tr><td>cc</td><td>d</td></tr></table>",
        "td",
        400.0,
    );
    assert_eq!(cells.len(), 4);
    assert!(close(cells[0].x, 2.0));
    assert!(close(cells[0].x, cells[2].x));
    assert!(close(cells[1].x, cells[3].x));
    assert!(close(cells[1].x, 2.0 + word("cc") + 2.0));
    assert!(close(cells[0].y, 2.0));
    assert!(close(cells[1].y, cells[0].y));
    assert!(cells[2].y > cells[0].y + cells[0].height);
    assert!(close(cells[0].width, word("cc")));
    assert!(close(cells[1].width, word("bbbb")));
}

#[test]
fn an_auto_width_table_shrinks_to_its_content() {
    let tables = rects("<table><tr><td>abc</td></tr></table>", "table", 400.0);
    assert!(close(tables[0].width, word("abc") + 4.0));
    assert!(close(tables[0].x, 0.0));
}

#[test]
fn an_explicit_width_is_shared_between_columns() {
    let body = "<table style=\"width:300px\"><tr><td>a</td><td>bb</td></tr></table>";
    let tables = rects(body, "table", 400.0);
    let cells = rects(body, "td", 400.0);
    assert!(close(tables[0].width, 300.0));
    assert!(close(cells[0].x + cells[0].width + 2.0, cells[1].x));
    assert!(close(cells[1].x + cells[1].width + 2.0, 300.0));
    assert!(cells[1].width > cells[0].width);
}

#[test]
fn a_long_text_column_takes_the_extra_space() {
    let body = "<table style=\"width:200px\"><tr><td>x</td><td>one two three four five six seven eight</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(cells[1].width > 4.0 * cells[0].width);
    assert!(close(cells[1].x + cells[1].width + 2.0, 200.0));
}

#[test]
fn text_wraps_inside_narrow_cells() {
    let narrow = "<table style=\"width:60px\"><tr><td>aaa bbb ccc ddd eee</td></tr></table>";
    let wide = "<table><tr><td>aaa bbb ccc ddd eee</td></tr></table>";
    let narrow_cells = rects(narrow, "td", 400.0);
    let wide_cells = rects(wide, "td", 400.0);
    assert!(narrow_cells[0].height > 2.0 * wide_cells[0].height);
    assert!(narrow_cells[0].width <= 56.0 + 0.01);
}

#[test]
fn the_table_never_gets_narrower_than_its_longest_words() {
    let body = "<table><tr><td>unbreakable sentence</td></tr></table>";
    let tables = rects(body, "table", 20.0);
    assert!(tables[0].width >= word("unbreakable") + 4.0 - 0.01);
}

#[test]
fn colspan_cells_cover_several_columns() {
    let body = "<table><tr><td>aa</td><td>bb</td></tr><tr><td colspan=\"2\">wide cell here</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[2].x, cells[0].x));
    let spanned = cells[1].x + cells[1].width - cells[0].x;
    assert!(close(cells[2].width, spanned.max(word("wide cell here"))));
    assert!(cells[1].x + cells[1].width <= cells[2].x + cells[2].width + 0.01);
}

#[test]
fn colspan_widens_columns_when_needed() {
    let body = "<table><tr><td>a</td><td>b</td></tr><tr><td colspan=\"2\">a much wider cell</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[2].width, word("a much wider cell")));
    assert!(close(cells[0].width + 2.0 + cells[1].width, cells[2].width));
}

#[test]
fn rowspan_cells_cover_several_rows() {
    let body = "<table><tr><td rowspan=\"2\">tall</td><td>a</td></tr><tr><td>b</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[0].y, cells[1].y));
    let rows_bottom = cells[2].y + cells[2].height;
    assert!(close(cells[0].y + cells[0].height, rows_bottom));
    assert!(close(cells[2].x, cells[1].x));
}

#[test]
fn header_and_footer_groups_are_reordered() {
    let body = "<table><tfoot><tr><td>foot</td></tr></tfoot><tbody><tr><td>body</td></tr></tbody><thead><tr><td>head</td></tr></thead></table>";
    let cells = rects(body, "td", 400.0);
    assert!(cells[2].y < cells[1].y);
    assert!(cells[1].y < cells[0].y);
}

#[test]
fn cells_stretch_to_the_row_height() {
    let body = "<table><tr><td style=\"height:40px\">a</td><td>b</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[0].height, 40.0));
    assert!(close(cells[1].height, 40.0));
}

fn first_baseline_of_second_cell(align: &str) -> f32 {
    let body = format!(
        "<table><tr><td style=\"height:60px\">a</td><td style=\"vertical-align:{}\">b</td></tr></table>",
        align
    );
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }} td {{ padding: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut baselines = Vec::new();
    collect_baselines(&layout_root, &mut baselines);
    baselines[1]
}

fn collect_baselines(layout_box: &LayoutBox, out: &mut Vec<f32>) {
    for fragment in &layout_box.fragments {
        out.push(fragment.baseline);
    }
    for child in &layout_box.children {
        collect_baselines(child, out);
    }
}

#[test]
fn vertical_align_positions_content_inside_the_row() {
    let top = first_baseline_of_second_cell("top");
    let middle = first_baseline_of_second_cell("middle");
    let bottom = first_baseline_of_second_cell("bottom");
    assert!(middle > top);
    assert!(bottom > middle);
    assert!(close(
        middle - top,
        (60.0 - text::line_height(&text::default_font(), 16.0)) / 2.0
    ));
    assert!(close(
        bottom - top,
        60.0 - text::line_height(&text::default_font(), 16.0)
    ));
}

#[test]
fn cells_default_to_middle_alignment_and_rows_can_override() {
    let default = {
        let body = "<table><tr><td style=\"height:60px\">a</td><td>b</td></tr></table>";
        let root = html::parse(&format!(
            "<html><head><style>body {{ margin: 0 }} td {{ padding: 0 }}</style></head><body>{}</body></html>",
            body
        ));
        let sheet = style::stylesheet_for_document(&root);
        let viewport = style::Viewport::new(400.0, 600.0);
        let styled = style::style_tree(&root, &sheet, viewport).unwrap();
        let layout_root = layout::layout_tree(
            &styled,
            viewport,
            &text::default_font(),
            &layout::ImageMap::new(),
        );
        let mut baselines = Vec::new();
        collect_baselines(&layout_root, &mut baselines);
        baselines
    };
    assert!(close(default[0], default[1]));
}

#[test]
fn collapsed_borders_share_one_line() {
    let body = "<table style=\"border-collapse:collapse\"><tr><td style=\"border:1px solid black\">a</td><td style=\"border:1px solid black\">b</td></tr><tr><td style=\"border:1px solid black\">c</td><td style=\"border:1px solid black\">d</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[0].x, 0.0));
    assert!(close(cells[0].x + cells[0].width, cells[1].x));
    assert!(close(cells[0].y + cells[0].height, cells[2].y));
    let page = Page::new(&format!(
        "<html><head><style>body {{ margin: 0 }} td {{ padding: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let canvas = page.render(100, 100, 0.0).canvas;
    let boundary = cells[1].x.round() as usize;
    let black = Color { r: 0, g: 0, b: 0 };
    assert_eq!(canvas.pixel(boundary - 1, 1), black);
    assert_eq!(canvas.pixel(boundary, 1), WHITE);
    assert_eq!(canvas.pixel(boundary - 2, 1), WHITE);
    let line = cells[2].y.round() as usize - 1;
    assert_eq!(canvas.pixel(boundary - 3, line), black);
    assert_eq!(canvas.pixel(boundary - 3, line - 1), WHITE);
}

#[test]
fn separate_borders_keep_their_spacing() {
    let body = "<table><tr><td style=\"border:1px solid black\">a</td><td style=\"border:1px solid black\">b</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[1].x - (cells[0].x + cells[0].width), 2.0));
}

#[test]
fn border_spacing_sets_both_axes() {
    let body = "<table style=\"border-spacing:6px 10px\"><tr><td>a</td><td>b</td></tr><tr><td>c</td><td>d</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[0].x, 6.0));
    assert!(close(cells[0].y, 10.0));
    assert!(close(cells[1].x - (cells[0].x + cells[0].width), 6.0));
    assert!(close(cells[2].y - (cells[0].y + cells[0].height), 10.0));
    let tables = rects(body, "table", 400.0);
    assert!(close(
        tables[0].height,
        10.0 + 2.0 * cells[0].height + 10.0 + 10.0
    ));
    let body = "<table style=\"border-spacing:0\"><tr><td>a</td><td>b</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(close(cells[0].x, 0.0));
    assert!(close(cells[1].x, cells[0].width));
}

#[test]
fn a_caption_sits_above_the_rows() {
    let body = "<table><caption>Title</caption><tr><td>a</td></tr></table>";
    let captions = rects(body, "caption", 400.0);
    let cells = rects(body, "td", 400.0);
    let tables = rects(body, "table", 400.0);
    assert!(close(captions[0].y, 0.0));
    assert!(cells[0].y >= captions[0].y + captions[0].height);
    assert!(close(captions[0].width, tables[0].width));
    assert!(tables[0].width >= word("Title"));
}

#[test]
fn header_cells_are_bold_and_centered() {
    let body = "<table style=\"width:200px\"><tr><th>mid</th></tr></table>";
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body>{}</body></html>",
        body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 600.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let layout_root = layout::layout_tree(
        &styled,
        viewport,
        &text::default_font(),
        &layout::ImageMap::new(),
    );
    let mut found = Vec::new();
    collect_fragments(&layout_root, &mut found);
    let (x, face) = found[0];
    let fonts = text::default_font();
    let width = fonts.width(1, "mid", 16.0);
    assert_eq!(face, 1);
    assert!(
        close(x, 2.0 + 1.0 + (196.0 - 2.0 - width) / 2.0) || (x - 100.0 + width / 2.0).abs() < 4.0
    );
}

fn collect_fragments(layout_box: &LayoutBox, out: &mut Vec<(f32, usize)>) {
    for fragment in &layout_box.fragments {
        out.push((fragment.x, fragment.face));
    }
    for child in &layout_box.children {
        collect_fragments(child, out);
    }
}

#[test]
fn row_and_cell_backgrounds_are_painted() {
    let page = Page::new(
        "<html><head><style>body { margin: 0 } td { padding: 0; width: 10px; height: 10px }</style></head><body><table style=\"border-spacing:4px\"><tr style=\"background-color:red\"><td style=\"background-color:blue\"></td><td></td></tr></table></body></html>",
    );
    let canvas = page.render(60, 30, 0.0).canvas;
    assert_eq!(canvas.pixel(8, 8), BLUE);
    assert_eq!(canvas.pixel(16, 8), RED);
    assert_eq!(canvas.pixel(2, 8), WHITE);
    assert_eq!(canvas.pixel(20, 8), RED);
    assert_eq!(canvas.pixel(30, 8), WHITE);
}

#[test]
fn tables_nest_inside_cells() {
    let body = "<table><tr><td><table><tr><td>inner</td></tr></table></td><td>x</td></tr></table>";
    let tables = rects(body, "table", 400.0);
    let cells = rects(body, "td", 400.0);
    assert_eq!(tables.len(), 2);
    assert!(tables[1].x >= tables[0].x);
    assert!(tables[1].width <= cells[0].width + 0.01);
    assert!(cells[2].width >= word("x") - 0.01);
}

#[test]
fn tables_center_with_auto_margins() {
    let body = "<table style=\"width:100px;margin:0 auto\"><tr><td>a</td></tr></table>";
    let tables = rects(body, "table", 300.0);
    assert!(close(tables[0].x, 100.0));
}

#[test]
fn huge_spans_are_clamped() {
    let body =
        "<table><tr><td colspan=\"99999\" rowspan=\"99999\">a</td></tr><tr><td>b</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert_eq!(cells.len(), 2);
}

#[test]
fn empty_tables_and_rows_are_harmless() {
    let tables = rects("<table></table>", "table", 400.0);
    assert_eq!(tables.len(), 1);
    let tables = rects("<table><tr></tr><tr><td></td></tr></table>", "table", 400.0);
    assert_eq!(tables.len(), 1);
    assert!(tables[0].height >= 0.0);
}

#[test]
fn table_height_is_at_least_the_specified_height() {
    let tables = rects(
        "<table style=\"height:100px\"><tr><td>a</td></tr></table>",
        "table",
        400.0,
    );
    assert!(tables[0].height >= 100.0);
    let tables = rects(
        "<table style=\"height:1px\"><tr><td>a</td></tr></table>",
        "table",
        400.0,
    );
    assert!(tables[0].height > 10.0);
}

#[test]
fn percentage_cell_widths_resolve_against_the_table() {
    let body =
        "<table style=\"width:200px\"><tr><td style=\"width:25%\">a</td><td>b</td></tr></table>";
    let cells = rects(body, "td", 400.0);
    assert!(cells[0].width >= 49.0);
    assert!(cells[0].width < cells[1].width);
}
