// SPDX-License-Identifier: GPL-3.0-only

use std::rc::Rc;

use crate::css::Color;
use crate::decoration::{Decoration, Line, LineStyle};
use crate::float::FloatContext;
use crate::layout::Rect;
use crate::picture::Picture;
use crate::style::{StyledNode, TextAlign, WhiteSpace};
use crate::text::Fonts;

#[derive(Debug, Clone, PartialEq)]
pub struct Run<'a> {
    pub text: &'a str,
    pub color: Color,
    pub alpha: u8,
    pub font_size: f32,
    pub line_height: Option<f32>,
    pub families: &'a [String],
    pub bold: bool,
    pub italic: bool,
    pub white_space: WhiteSpace,
    pub decorations: &'a [Decoration],
}

#[derive(Debug, Clone)]
pub struct BlockRun<'a> {
    pub node: &'a StyledNode<'a>,
    pub font_size: f32,
}

impl PartialEq for BlockRun<'_> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.node, other.node)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockMetrics {
    pub width: f32,
    pub height: f32,
    pub baseline: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlacedBlock {
    pub index: usize,
    pub x: f32,
    pub baseline: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageRun {
    pub picture: Option<Rc<Picture>>,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineBoxStyle {
    pub id: usize,
    pub background: Option<Color>,
    pub border_color: Option<Color>,
    pub padding: [f32; 4],
    pub border: [f32; 4],
    pub margin_left: f32,
    pub margin_right: f32,
}

impl InlineBoxStyle {
    fn left_extra(&self) -> f32 {
        self.margin_left + self.border[3] + self.padding[3]
    }

    fn right_extra(&self) -> f32 {
        self.margin_right + self.border[1] + self.padding[1]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InlineItem<'a> {
    Text(Run<'a>),
    Image(ImageRun),
    Block(BlockRun<'a>),
    Open(Rc<InlineBoxStyle>),
    Close(usize),
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
    pub alpha: u8,
    pub font_size: f32,
    pub face: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuleFragment {
    pub rect: Rect,
    pub color: Color,
    pub alpha: u8,
    pub over_text: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InlineBoxFragment {
    pub rect: Rect,
    pub background: Option<Color>,
    pub border: [f32; 4],
    pub border_color: Option<Color>,
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
    pub boxes: Vec<InlineBoxFragment>,
    pub blocks: Vec<PlacedBlock>,
    pub rules: Vec<RuleFragment>,
    pub height: f32,
}

#[derive(Clone, Copy)]
enum Piece<'a> {
    Text {
        text: &'a str,
        run: &'a Run<'a>,
        face: usize,
    },
    Image(&'a ImageRun),
    Block {
        run: &'a BlockRun<'a>,
        metrics: &'a BlockMetrics,
        index: usize,
    },
}

struct Word<'a> {
    piece: Piece<'a>,
    space_before: bool,
    break_before: bool,
    opens: Vec<Rc<InlineBoxStyle>>,
    closes: Vec<Rc<InlineBoxStyle>>,
    stack: Vec<Rc<InlineBoxStyle>>,
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
            Piece::Block { run, .. } => run.font_size,
        }
    }

    fn face(&self) -> usize {
        match &self.piece {
            Piece::Text { face, .. } => *face,
            Piece::Image(_) | Piece::Block { .. } => 0,
        }
    }

    fn wraps(&self) -> bool {
        match &self.piece {
            Piece::Text { run, .. } => run.white_space.wraps(),
            Piece::Image(_) | Piece::Block { .. } => true,
        }
    }

    fn lead(&self) -> f32 {
        self.opens.iter().map(|style| style.left_extra()).sum()
    }

    fn trail(&self) -> f32 {
        self.closes.iter().map(|style| style.right_extra()).sum()
    }

    fn width(&self, fonts: &Fonts) -> f32 {
        let content = match &self.piece {
            Piece::Text { text, run, face } => fonts.width(*face, text, run.font_size),
            Piece::Image(image) => image.width,
            Piece::Block { metrics, .. } => metrics.width,
        };
        self.lead() + content + self.trail()
    }
}

fn text_words<'a>(
    run: &'a Run<'a>,
    carry_space: bool,
    pending_break: &mut bool,
) -> Vec<(&'a str, bool, bool)> {
    let mut words = Vec::new();
    if !run.white_space.keeps_breaks() {
        for (index, text) in run.text.split_whitespace().enumerate() {
            let space_before = if index == 0 {
                carry_space || run.text.starts_with(char::is_whitespace)
            } else {
                true
            };
            words.push((text, space_before, false));
        }
        return words;
    }

    let lines: Vec<&str> = run.text.split('\n').collect();
    let last = lines.len() - 1;
    for (number, line) in lines.into_iter().enumerate() {
        let mut breaks = number > 0 || std::mem::take(pending_break);
        let before = words.len();
        let line = line.strip_suffix('\r').unwrap_or(line);
        if run.white_space.keeps_spaces() {
            let mut start = 0;
            let mut previous_space = false;
            for (index, c) in line.char_indices() {
                let space = c == ' ' || c == '\t';
                if previous_space && !space {
                    words.push((&line[start..index], false, std::mem::take(&mut breaks)));
                    start = index;
                }
                previous_space = space;
            }
            if start < line.len() {
                words.push((&line[start..], false, std::mem::take(&mut breaks)));
            }
        } else {
            for (index, text) in line.split_whitespace().enumerate() {
                let space_before = if index == 0 {
                    number == 0 && (carry_space || line.starts_with(char::is_whitespace))
                } else {
                    true
                };
                words.push((text, space_before, std::mem::take(&mut breaks)));
            }
        }
        if words.len() == before {
            if number < last {
                words.push(("", false, breaks));
            } else if breaks {
                *pending_break = true;
            }
        }
    }
    words
}

fn collect_words<'a>(
    items: &'a [InlineItem<'a>],
    fonts: &Fonts,
    blocks: &'a [BlockMetrics],
) -> Vec<Word<'a>> {
    let mut block_index = 0;
    let mut words: Vec<Word<'a>> = Vec::new();
    let mut carry_space = false;
    let mut pending_break = false;
    let mut stack: Vec<Rc<InlineBoxStyle>> = Vec::new();
    let mut pending: Vec<Rc<InlineBoxStyle>> = Vec::new();
    for item in items {
        match item {
            InlineItem::Text(run) => {
                let face = fonts.select(run.families, run.bold, run.italic);
                for (text, space_before, break_before) in
                    text_words(run, carry_space, &mut pending_break)
                {
                    words.push(Word {
                        piece: Piece::Text { text, run, face },
                        space_before,
                        break_before,
                        opens: std::mem::take(&mut pending),
                        closes: Vec::new(),
                        stack: stack.clone(),
                    });
                }
                carry_space = !run.white_space.keeps_spaces()
                    && !pending_break
                    && run.text.ends_with(char::is_whitespace);
            }
            InlineItem::Image(image) => {
                words.push(Word {
                    piece: Piece::Image(image),
                    space_before: carry_space,
                    break_before: std::mem::take(&mut pending_break),
                    opens: std::mem::take(&mut pending),
                    closes: Vec::new(),
                    stack: stack.clone(),
                });
                carry_space = false;
            }
            InlineItem::Block(run) => {
                words.push(Word {
                    piece: Piece::Block {
                        run,
                        metrics: &blocks[block_index],
                        index: block_index,
                    },
                    space_before: carry_space,
                    break_before: std::mem::take(&mut pending_break),
                    opens: std::mem::take(&mut pending),
                    closes: Vec::new(),
                    stack: stack.clone(),
                });
                block_index += 1;
                carry_space = false;
            }
            InlineItem::Open(style) => {
                stack.push(style.clone());
                pending.push(style.clone());
            }
            InlineItem::Close(id) => {
                if let Some(position) = pending.iter().position(|style| style.id == *id) {
                    pending.remove(position);
                } else if let Some(word) = words.last_mut() {
                    if let Some(style) = stack.iter().find(|style| style.id == *id) {
                        word.closes.push(style.clone());
                    }
                }
                if let Some(position) = stack.iter().rposition(|style| style.id == *id) {
                    stack.remove(position);
                }
            }
        }
    }
    words
}

fn split_long_word<'a>(word: Word<'a>, width: f32, fonts: &Fonts) -> Vec<Word<'a>> {
    let Piece::Text { text, run, face } = word.piece else {
        return vec![word];
    };
    let mut pieces = Vec::new();
    let mut start = 0;
    let mut piece_width = 0.0;
    for (index, c) in text.char_indices() {
        let advance = fonts.advance(face, c, run.font_size);
        if piece_width + advance > width && index > start {
            pieces.push(&text[start..index]);
            start = index;
            piece_width = 0.0;
        }
        piece_width += advance;
    }
    pieces.push(&text[start..]);

    let count = pieces.len();
    let mut result = Vec::new();
    for (index, piece) in pieces.into_iter().enumerate() {
        result.push(Word {
            piece: Piece::Text {
                text: piece,
                run,
                face,
            },
            space_before: index == 0 && word.space_before,
            break_before: true,
            opens: if index == 0 {
                word.opens.clone()
            } else {
                Vec::new()
            },
            closes: if index + 1 == count {
                word.closes.clone()
            } else {
                Vec::new()
            },
            stack: word.stack.clone(),
        });
    }
    result
}

fn fit_words<'a>(words: Vec<Word<'a>>, width: f32, fonts: &Fonts) -> Vec<Word<'a>> {
    let mut fitted = Vec::new();
    for word in words {
        if word.wraps() && matches!(word.piece, Piece::Text { .. }) && word.width(fonts) > width {
            fitted.extend(split_long_word(word, width, fonts));
        } else {
            fitted.push(word);
        }
    }
    fitted
}

pub fn max_content_width(content: &InlineContent, fonts: &Fonts, blocks: &[BlockMetrics]) -> f32 {
    let words = collect_words(&content.items, fonts, blocks);
    let mut widest: f32 = 0.0;
    let mut total = 0.0;
    for word in &words {
        if word.break_before {
            total = 0.0;
        }
        if total > 0.0 && word.space_before {
            total += fonts.width(word.face(), " ", word.font_size());
        }
        total += word.width(fonts);
        widest = widest.max(total);
    }
    widest
}

pub fn min_content_width(content: &InlineContent, fonts: &Fonts, blocks: &[BlockMetrics]) -> f32 {
    let words = collect_words(&content.items, fonts, blocks);
    let mut widest: f32 = 0.0;
    let mut current = 0.0;
    for word in &words {
        let mut width = word.width(fonts);
        if !word.wraps() && !word.break_before && current > 0.0 {
            if word.space_before {
                width += fonts.width(word.face(), " ", word.font_size());
            }
            current += width;
        } else {
            current = width;
        }
        widest = widest.max(current);
    }
    widest
}

pub fn layout_lines(
    content: &InlineContent,
    width: f32,
    origin_x: f32,
    origin_y: f32,
    fonts: &Fonts,
    floats: &FloatContext,
    blocks: &[BlockMetrics],
) -> Lines {
    let words = fit_words(collect_words(&content.items, fonts, blocks), width, fonts);
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
        let word_width = word.width(fonts);
        let mut space = 0.0;
        if x > 0.0 && word.space_before {
            space = fonts.width(word.face(), " ", word.font_size());
        }
        let overflows = word.wraps() && x + space + word_width > area.width;
        if !line.is_empty() && (overflows || word.break_before) {
            top += finish_line(&line, content.align, &area, top, fonts, &mut lines);
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
        top += finish_line(&line, content.align, &area, top, fonts, &mut lines);
    }
    lines.height = top;
    lines
}

fn finish_line(
    line: &[PlacedWord],
    align: TextAlign,
    area: &Area,
    top: f32,
    fonts: &Fonts,
    lines: &mut Lines,
) -> f32 {
    let mut above: f32 = 0.0;
    let mut below: f32 = 0.0;
    for placed in line {
        match &placed.word.piece {
            Piece::Text { run, face, .. } => {
                let ascent = fonts.ascent(*face, run.font_size);
                let descent = fonts.descent(*face, run.font_size);
                let line_height = match run.line_height {
                    Some(height) => height,
                    None => fonts.line_height(*face, run.font_size),
                };
                let word_above = (line_height - ascent - descent) / 2.0 + ascent;
                above = above.max(word_above);
                below = below.max(line_height - word_above);
            }
            Piece::Image(image) => above = above.max(image.height),
            Piece::Block { metrics, .. } => {
                above = above.max(metrics.baseline);
                below = below.max(metrics.height - metrics.baseline);
            }
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

    let origin = area.origin_x + shift;
    let baseline = area.origin_y + top + above;
    for placed in line {
        let x = origin + placed.offset + placed.word.lead();
        match &placed.word.piece {
            Piece::Text {
                text: word_text,
                run,
                face,
            } => {
                lines.fragments.push(Fragment {
                    x,
                    baseline,
                    text: word_text.to_string(),
                    color: run.color,
                    alpha: run.alpha,
                    font_size: run.font_size,
                    face: *face,
                });
            }
            Piece::Block { index, .. } => lines.blocks.push(PlacedBlock {
                index: *index,
                x,
                baseline,
            }),
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
    push_boxes(line, origin, baseline, fonts, lines);
    push_rules(line, origin, baseline, fonts, lines);
    above + below
}

struct Span {
    decoration: Decoration,
    last_word: usize,
    x0: f32,
    x1: f32,
    face: usize,
    font_size: f32,
}

fn push_rules(line: &[PlacedWord], origin: f32, baseline: f32, fonts: &Fonts, lines: &mut Lines) {
    let mut spans: Vec<Span> = Vec::new();
    for (index, placed) in line.iter().enumerate() {
        let Piece::Text { text, run, face } = &placed.word.piece else {
            continue;
        };
        let x0 = origin + placed.offset + placed.word.lead();
        let x1 = x0 + fonts.width(*face, text, run.font_size);
        for decoration in run.decorations {
            let joined = spans.iter_mut().find(|span| {
                span.decoration == *decoration
                    && span.font_size == run.font_size
                    && span.last_word + 1 == index
            });
            match joined {
                Some(span) => {
                    span.x1 = x1;
                    span.last_word = index;
                }
                None => spans.push(Span {
                    decoration: *decoration,
                    last_word: index,
                    x0,
                    x1,
                    face: *face,
                    font_size: run.font_size,
                }),
            }
        }
    }
    for span in &spans {
        emit_rules(span, baseline, fonts, lines);
    }
}

fn emit_rules(span: &Span, baseline: f32, fonts: &Fonts, lines: &mut Lines) {
    let size = span.font_size;
    let decoration = &span.decoration;
    let thickness = decoration.thickness.unwrap_or(size / 16.0).round().max(1.0);
    let top = match decoration.line {
        Line::Underline => baseline + size * 0.1,
        Line::Overline => baseline - fonts.ascent(span.face, size),
        Line::LineThrough => baseline - size * 0.3 - thickness / 2.0,
    }
    .round();
    let width = span.x1 - span.x0;
    let over_text = decoration.line == Line::LineThrough;
    let mut push = |x: f32, y: f32, width: f32| {
        lines.rules.push(RuleFragment {
            rect: Rect {
                x,
                y,
                width,
                height: thickness,
            },
            color: decoration.color.color,
            alpha: decoration.color.alpha,
            over_text,
        });
    };
    let (period, dash) = match decoration.style {
        LineStyle::Dotted => (thickness * 2.0, thickness),
        LineStyle::Dashed => (thickness * 5.0, thickness * 3.0),
        LineStyle::Solid | LineStyle::Double => (0.0, 0.0),
    };
    if period > 0.0 && width / period < 2000.0 {
        let mut x = span.x0;
        while x < span.x1 {
            push(x, top, dash.min(span.x1 - x));
            x += period;
        }
        return;
    }
    push(span.x0, top, width);
    if decoration.style == LineStyle::Double {
        push(span.x0, top + thickness * 2.0, width);
    }
}

fn push_boxes(line: &[PlacedWord], origin: f32, baseline: f32, fonts: &Fonts, lines: &mut Lines) {
    let mut styles: Vec<Rc<InlineBoxStyle>> = Vec::new();
    for placed in line {
        for style in &placed.word.stack {
            if !styles.iter().any(|known| known.id == style.id) {
                styles.push(style.clone());
            }
        }
    }
    for style in &styles {
        let inside = |placed: &PlacedWord| placed.word.stack.iter().any(|s| s.id == style.id);
        let Some(first) = line.iter().position(inside) else {
            continue;
        };
        let Some(last) = line.iter().rposition(inside) else {
            continue;
        };
        let first_word = line[first].word;
        let last_word = line[last].word;

        let opens_here = first_word.opens.iter().position(|s| s.id == style.id);
        let mut left = origin + line[first].offset;
        if let Some(index) = opens_here {
            for outer in &first_word.opens[..index] {
                left += outer.left_extra();
            }
            left += style.margin_left;
        }
        let closes_here = last_word.closes.iter().position(|s| s.id == style.id);
        let mut right = origin + line[last].offset + line[last].width;
        if let Some(index) = closes_here {
            for outer in &last_word.closes[index + 1..] {
                right -= outer.right_extra();
            }
            right -= style.margin_right;
        }

        let font_size = first_word.font_size();
        let ascent = fonts.ascent(first_word.face(), font_size);
        let descent = fonts.descent(first_word.face(), font_size);
        let above = style.padding[0] + style.border[0];
        let below = style.padding[2] + style.border[2];
        lines.boxes.push(InlineBoxFragment {
            rect: Rect {
                x: left,
                y: baseline - ascent - above,
                width: (right - left).max(0.0),
                height: ascent + descent + above + below,
            },
            background: style.background,
            border: [
                style.border[0],
                if closes_here.is_some() {
                    style.border[1]
                } else {
                    0.0
                },
                style.border[2],
                if opens_here.is_some() {
                    style.border[3]
                } else {
                    0.0
                },
            ],
            border_color: style.border_color,
        });
    }
}
