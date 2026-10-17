// SPDX-License-Identifier: GPL-3.0-only

use std::rc::Rc;

use fontdue::Font;

use crate::css::Color;
use crate::float::FloatContext;
use crate::layout::Rect;
use crate::picture::Picture;
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
pub struct ImageRun {
    pub picture: Option<Rc<Picture>>,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineItem<'a> {
    Text(Run<'a>),
    Image(ImageRun),
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineContent<'a> {
    pub items: Vec<InlineItem<'a>>,
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

#[derive(Debug, Clone, PartialEq)]
pub struct ImageFragment {
    pub rect: Rect,
    pub picture: Rc<Picture>,
}

#[derive(Debug, Default)]
pub struct Lines {
    pub fragments: Vec<Fragment>,
    pub images: Vec<ImageFragment>,
    pub height: f32,
}

enum Piece<'a> {
    Text { text: &'a str, run: &'a Run<'a> },
    Image(&'a ImageRun),
}

struct Word<'a> {
    piece: Piece<'a>,
    space_before: bool,
    break_before: bool,
}

struct PlacedWord<'a, 'b> {
    word: &'b Word<'a>,
    offset: f32,
    width: f32,
}

struct Area {
    width: f32,
    origin_x: f32,
    origin_y: f32,
}

impl Word<'_> {
    fn font_size(&self) -> f32 {
        match &self.piece {
            Piece::Text { run, .. } => run.font_size,
            Piece::Image(image) => image.font_size,
        }
    }

    fn width(&self, font: &Font) -> f32 {
        match &self.piece {
            Piece::Text { text, run } => text::text_width(font, text, run.font_size),
            Piece::Image(image) => image.width,
        }
    }

    fn background(&self) -> Option<Color> {
        match &self.piece {
            Piece::Text { run, .. } => run.background,
            Piece::Image(_) => None,
        }
    }
}

fn collect_words<'a>(items: &'a [InlineItem<'a>]) -> Vec<Word<'a>> {
    let mut words = Vec::new();
    let mut carry_space = false;
    for item in items {
        match item {
            InlineItem::Text(run) => {
                let mut first = true;
                for text in run.text.split_whitespace() {
                    let space_before = if first {
                        carry_space || run.text.starts_with(char::is_whitespace)
                    } else {
                        true
                    };
                    first = false;
                    words.push(Word {
                        piece: Piece::Text { text, run },
                        space_before,
                        break_before: false,
                    });
                }
                carry_space = run.text.ends_with(char::is_whitespace);
            }
            InlineItem::Image(image) => {
                words.push(Word {
                    piece: Piece::Image(image),
                    space_before: carry_space,
                    break_before: false,
                });
                carry_space = false;
            }
        }
    }
    words
}

fn split_long_word<'a>(
    text: &'a str,
    run: &'a Run<'a>,
    space_before: bool,
    width: f32,
    font: &Font,
) -> Vec<Word<'a>> {
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut piece_width = 0.0;
    for (index, c) in text.char_indices() {
        let advance = font.metrics(c, run.font_size).advance_width;
        if piece_width + advance > width && index > start {
            pieces.push(&text[start..index]);
            start = index;
            piece_width = 0.0;
        }
        piece_width += advance;
    }
    pieces.push(&text[start..]);

    let mut result = Vec::new();
    for (index, piece) in pieces.into_iter().enumerate() {
        result.push(Word {
            piece: Piece::Text { text: piece, run },
            space_before: index == 0 && space_before,
            break_before: true,
        });
    }
    result
}

fn fit_words<'a>(words: Vec<Word<'a>>, width: f32, font: &Font) -> Vec<Word<'a>> {
    let mut fitted = Vec::new();
    for word in words {
        match word.piece {
            Piece::Text { text, run } if word.width(font) > width => {
                fitted.extend(split_long_word(text, run, word.space_before, width, font));
            }
            _ => fitted.push(word),
        }
    }
    fitted
}

pub fn max_content_width(content: &InlineContent, font: &Font) -> f32 {
    let words = collect_words(&content.items);
    let mut total = 0.0;
    for word in &words {
        if total > 0.0 && word.space_before {
            total += text::text_width(font, " ", word.font_size());
        }
        total += word.width(font);
    }
    total
}

pub fn layout_lines(
    content: &InlineContent,
    width: f32,
    origin_x: f32,
    origin_y: f32,
    font: &Font,
    floats: &FloatContext,
) -> Lines {
    let words = fit_words(collect_words(&content.items), width, font);
    let mut lines = Lines::default();
    let mut line: Vec<PlacedWord> = Vec::new();
    let mut area = Area {
        width,
        origin_x,
        origin_y,
    };
    let mut x = 0.0;
    let mut top = 0.0;

    for word in &words {
        let word_width = word.width(font);
        let mut space = 0.0;
        if x > 0.0 && word.space_before {
            space = text::text_width(font, " ", word.font_size());
        }
        let overflows = x + space + word_width > area.width;
        if !line.is_empty() && (overflows || word.break_before) {
            top += finish_line(&line, content.align, &area, top, font, &mut lines);
            line.clear();
            x = 0.0;
            space = 0.0;
        }
        if line.is_empty() {
            loop {
                let y = origin_y + top;
                let (left, right) = floats.bounds_at(y, origin_x, origin_x + width);
                if word_width <= right - left || !floats.is_blocked_at(y) {
                    area.origin_x = left;
                    area.width = right - left;
                    break;
                }
                top = floats.next_clear_after(y) - origin_y;
            }
        }
        line.push(PlacedWord {
            word,
            offset: x + space,
            width: word_width,
        });
        x += space + word_width;
    }
    if !line.is_empty() {
        top += finish_line(&line, content.align, &area, top, font, &mut lines);
    }
    lines.height = top;
    lines
}

fn finish_line(
    line: &[PlacedWord],
    align: TextAlign,
    area: &Area,
    top: f32,
    font: &Font,
    lines: &mut Lines,
) -> f32 {
    let mut above: f32 = 0.0;
    let mut below: f32 = 0.0;
    for placed in line {
        match &placed.word.piece {
            Piece::Text { run, .. } => {
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
            Piece::Image(image) => above = above.max(image.height),
        }
    }

    let last = &line[line.len() - 1];
    let line_width = last.offset + last.width;
    let shift = match align {
        TextAlign::Left => 0.0,
        TextAlign::Center => (area.width - line_width) / 2.0,
        TextAlign::Right => area.width - line_width,
    }
    .max(0.0);

    let baseline = area.origin_y + top + above;
    for (index, placed) in line.iter().enumerate() {
        let x = area.origin_x + shift + placed.offset;
        match &placed.word.piece {
            Piece::Text {
                text: word_text,
                run,
            } => {
                let background = match run.background {
                    Some(color) => {
                        let mut right = x + placed.width;
                        if let Some(next) = line.get(index + 1) {
                            if next.word.background() == Some(color) {
                                right = area.origin_x + shift + next.offset;
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
                lines.fragments.push(Fragment {
                    x,
                    baseline,
                    text: word_text.to_string(),
                    color: run.color,
                    font_size: run.font_size,
                    background,
                });
            }
            Piece::Image(image) => {
                if let Some(picture) = &image.picture {
                    lines.images.push(ImageFragment {
                        rect: Rect {
                            x,
                            y: baseline - image.height,
                            width: image.width,
                            height: image.height,
                        },
                        picture: picture.clone(),
                    });
                }
            }
        }
    }
    above + below
}
