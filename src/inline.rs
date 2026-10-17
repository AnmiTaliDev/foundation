// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::css::Color;
use crate::layout::Rect;
use crate::style::TextAlign;
use crate::text;

#[derive(Debug, Clone, PartialEq)]
pub struct Run<'a> {
    pub text: &'a str,
    pub color: Color,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub background: Option<Color>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineContent<'a> {
    pub runs: Vec<Run<'a>>,
    pub align: TextAlign,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Fragment {
    pub x: f32,
    pub baseline: f32,
    pub text: String,
    pub color: Color,
    pub font_size: f32,
    pub background: Option<(Color, Rect)>,
}

struct Word<'a> {
    text: &'a str,
    run: &'a Run<'a>,
    space_before: bool,
    break_before: bool,
}

struct PlacedWord<'a, 'b> {
    word: &'b Word<'a>,
    offset: f32,
    width: f32,
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
                break_before: false,
            });
        }
        carry_space = run.text.ends_with(char::is_whitespace);
    }
    words
}

fn split_long_word<'a>(word: Word<'a>, width: f32, font: &Font) -> Vec<Word<'a>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut piece_width = 0.0;
    for (index, c) in word.text.char_indices() {
        let advance = font.metrics(c, word.run.font_size).advance_width;
        if piece_width + advance > width && index > start {
            pieces.push(&word.text[start..index]);
            start = index;
            piece_width = 0.0;
        }
        piece_width += advance;
    }
    pieces.push(&word.text[start..]);

    let mut result = Vec::new();
    for (index, text) in pieces.into_iter().enumerate() {
        result.push(Word {
            text,
            run: word.run,
            space_before: index == 0 && word.space_before,
            break_before: true,
        });
    }
    result
}

fn fit_words<'a>(words: Vec<Word<'a>>, width: f32, font: &Font) -> Vec<Word<'a>> {
    let mut fitted = Vec::new();
    for word in words {
        if text::text_width(font, word.text, word.run.font_size) > width {
            fitted.extend(split_long_word(word, width, font));
        } else {
            fitted.push(word);
        }
    }
    fitted
}

pub fn layout_lines(
    content: &InlineContent,
    width: f32,
    origin_x: f32,
    origin_y: f32,
    font: &Font,
) -> (Vec<Fragment>, f32) {
    let words = fit_words(collect_words(&content.runs), width, font);
    let mut fragments = Vec::new();
    let mut line: Vec<PlacedWord> = Vec::new();
    let mut x = 0.0;
    let mut top = 0.0;

    for word in &words {
        let word_width = text::text_width(font, word.text, word.run.font_size);
        let mut space = 0.0;
        if x > 0.0 && word.space_before {
            space = text::text_width(font, " ", word.run.font_size);
        }
        let overflows = x + space + word_width > width;
        if !line.is_empty() && (overflows || word.break_before) {
            top += finish_line(
                &line,
                top,
                content.align,
                width,
                origin_x,
                origin_y,
                font,
                &mut fragments,
            );
            line.clear();
            x = 0.0;
            space = 0.0;
        }
        line.push(PlacedWord {
            word,
            offset: x + space,
            width: word_width,
        });
        x += space + word_width;
    }
    if !line.is_empty() {
        top += finish_line(
            &line,
            top,
            content.align,
            width,
            origin_x,
            origin_y,
            font,
            &mut fragments,
        );
    }
    (fragments, top)
}

#[allow(clippy::too_many_arguments)]
fn finish_line(
    line: &[PlacedWord],
    top: f32,
    align: TextAlign,
    width: f32,
    origin_x: f32,
    origin_y: f32,
    font: &Font,
    fragments: &mut Vec<Fragment>,
) -> f32 {
    let mut above: f32 = 0.0;
    let mut below: f32 = 0.0;
    for placed in line {
        let run = placed.word.run;
        let ascent = text::ascent(font, run.font_size);
        let descent = text::descent(font, run.font_size);
        let line_height = match run.line_height {
            Some(height) => height,
            None => text::line_height(font, run.font_size),
        };
        let word_above = (line_height - ascent - descent) / 2.0 + ascent;
        above = above.max(word_above);
        below = below.max(line_height - word_above);
    }

    let last = &line[line.len() - 1];
    let line_width = last.offset + last.width;
    let shift = match align {
        TextAlign::Left => 0.0,
        TextAlign::Center => (width - line_width) / 2.0,
        TextAlign::Right => width - line_width,
    }
    .max(0.0);

    let baseline = origin_y + top + above;
    for (index, placed) in line.iter().enumerate() {
        let run = placed.word.run;
        let x = origin_x + shift + placed.offset;
        let background = match run.background {
            Some(color) => {
                let mut right = x + placed.width;
                if let Some(next) = line.get(index + 1) {
                    if next.word.run.background == Some(color) {
                        right = origin_x + shift + next.offset;
                    }
                }
                let ascent = text::ascent(font, run.font_size);
                let descent = text::descent(font, run.font_size);
                Some((
                    color,
                    Rect {
                        x,
                        y: baseline - ascent,
                        width: right - x,
                        height: ascent + descent,
                    },
                ))
            }
            None => None,
        };
        fragments.push(Fragment {
            x,
            baseline,
            text: placed.word.text.to_string(),
            color: run.color,
            font_size: run.font_size,
            background,
        });
    }
    above + below
}
