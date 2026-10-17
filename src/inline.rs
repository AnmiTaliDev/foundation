// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::css::Color;
use crate::text;

#[derive(Debug, Clone, PartialEq)]
pub struct Run<'a> {
    pub text: &'a str,
    pub color: Color,
    pub font_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fragment {
    pub x: f32,
    pub baseline: f32,
    pub text: String,
    pub color: Color,
    pub font_size: f32,
}

struct Word<'a> {
    text: &'a str,
    run: &'a Run<'a>,
    space_before: bool,
}

fn collect_words<'a>(runs: &'a [Run<'a>]) -> Vec<Word<'a>> {
    let mut words = Vec::new();
    let mut carry_space = false;
    for run in runs {
        let mut first = true;
        for text in run.text.split_whitespace() {
            let space_before = if first {
                carry_space || run.text.starts_with(char::is_whitespace)
            } else {
                true
            };
            first = false;
            words.push(Word {
                text,
                run,
                space_before,
            });
        }
        carry_space = run.text.ends_with(char::is_whitespace);
    }
    words
}

pub fn layout_lines(
    runs: &[Run],
    width: f32,
    origin_x: f32,
    origin_y: f32,
    font: &Font,
) -> (Vec<Fragment>, f32) {
    let words = collect_words(runs);
    let mut fragments = Vec::new();
    let mut line: Vec<(f32, &Word)> = Vec::new();
    let mut x = 0.0;
    let mut top = 0.0;

    for word in &words {
        let word_width = text::text_width(font, word.text, word.run.font_size);
        let mut space = 0.0;
        if x > 0.0 && word.space_before {
            space = text::text_width(font, " ", word.run.font_size);
        }
        if !line.is_empty() && x + space + word_width > width {
            top += finish_line(&line, top, origin_x, origin_y, font, &mut fragments);
            line.clear();
            x = 0.0;
            space = 0.0;
        }
        line.push((x + space, word));
        x += space + word_width;
    }
    if !line.is_empty() {
        top += finish_line(&line, top, origin_x, origin_y, font, &mut fragments);
    }
    (fragments, top)
}

fn finish_line(
    line: &[(f32, &Word)],
    top: f32,
    origin_x: f32,
    origin_y: f32,
    font: &Font,
    fragments: &mut Vec<Fragment>,
) -> f32 {
    let mut height: f32 = 0.0;
    let mut ascent: f32 = 0.0;
    for (_, word) in line {
        height = height.max(text::line_height(font, word.run.font_size));
        ascent = ascent.max(text::ascent(font, word.run.font_size));
    }
    for (offset, word) in line {
        fragments.push(Fragment {
            x: origin_x + offset,
            baseline: origin_y + top + ascent,
            text: word.text.to_string(),
            color: word.run.color,
            font_size: word.run.font_size,
        });
    }
    height
}
