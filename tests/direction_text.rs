// SPDX-License-Identifier: GPL-3.0-only

use foundation::layout::{self, LayoutBox};
use foundation::{html, style, text};

fn fragments(style_text: &str, body: &str) -> Vec<(f32, f32)> {
    fn walk(layout_box: &LayoutBox, out: &mut Vec<(f32, f32, f32)>, fonts: &text::Fonts) {
        for fragment in &layout_box.fragments {
            let width = fonts.width(fragment.face, &fragment.text, fragment.font_size);
            out.push((fragment.baseline, fragment.x, fragment.x + width));
        }
        for child in &layout_box.children {
            walk(child, out, fonts);
        }
    }
    let root = html::parse(&format!(
        "<html><head><style>body {{ margin: 0 }}</style></head><body><div style=\"width: 100px; {}\">{}</div></body></html>",
        style_text, body
    ));
    let sheet = style::stylesheet_for_document(&root);
    let viewport = style::Viewport::new(400.0, 400.0);
    let styled = style::style_tree(&root, &sheet, viewport).unwrap();
    let fonts = text::default_font();
    let layout_root = layout::layout_tree(&styled, viewport, &fonts, &layout::ImageMap::new());
    let mut words = Vec::new();
    walk(&layout_root, &mut words, &fonts);
    let mut lines: Vec<(f32, f32, f32)> = Vec::new();
    for (baseline, left, right) in words {
        match lines.last_mut() {
            Some(line) if close(line.0, baseline) => {
                line.1 = line.1.min(left);
                line.2 = line.2.max(right);
            }
            _ => lines.push((baseline, left, right)),
        }
    }
    lines
        .into_iter()
        .map(|(_, left, right)| (left, right))
        .collect()
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 0.01
}

const LINES: &str = "aaaa bbbb cccc dddd eeee";

#[test]
fn ltr_indent_shifts_the_first_line_right() {
    let found = fragments("text-indent: 20px", LINES);
    assert!(close(found[0].0, 20.0));
    let second = found.iter().find(|(x, _)| close(*x, 0.0));
    assert!(second.is_some());
}

#[test]
fn rtl_indent_shortens_the_first_line_from_the_right() {
    let found = fragments("direction: rtl; text-indent: 20px", LINES);
    assert!(close(found[0].1, 80.0), "{:?}", found);
    assert!(found.iter().skip(1).any(|(_, right)| close(*right, 100.0)));
}

#[test]
fn negative_rtl_indent_hangs_to_the_right() {
    let found = fragments("direction: rtl; text-indent: -20px", LINES);
    assert!(close(found[0].1, 120.0), "{:?}", found);
}

#[test]
fn percentage_indents_use_the_width() {
    let found = fragments("direction: rtl; text-indent: 10%", LINES);
    assert!(close(found[0].1, 90.0), "{:?}", found);
}

#[test]
fn only_the_first_line_is_indented() {
    let found = fragments(
        "direction: rtl; text-indent: 20px",
        "aaaa bbbb cccc dddd eeee ffff",
    );
    let rights: Vec<f32> = found.iter().map(|(_, right)| *right).collect();
    assert!(close(rights[0], 80.0));
    assert!(rights[1..].iter().all(|right| close(*right, 100.0)));
}

#[test]
fn the_last_justified_line_follows_the_direction() {
    let rtl = fragments("direction: rtl; text-align: justify", "aaaa bbbb cccc");
    let last = rtl.last().unwrap();
    assert!(close(last.1, 100.0), "{:?}", rtl);
    let ltr = fragments("text-align: justify", "aaaa bbbb cccc");
    assert!(close(ltr.last().unwrap().0, 0.0), "{:?}", ltr);
}

#[test]
fn justified_full_lines_fill_the_width_in_both_directions() {
    let rtl = fragments(
        "direction: rtl; text-align: justify",
        "aaaa bbbb cccc dddd eeee",
    );
    let first_line: Vec<&(f32, f32)> = rtl.iter().take(2).collect();
    assert!(close(
        first_line.iter().map(|f| f.0).fold(f32::MAX, f32::min),
        0.0
    ));
    assert!(close(
        first_line.iter().map(|f| f.1).fold(f32::MIN, f32::max),
        100.0
    ));
}
