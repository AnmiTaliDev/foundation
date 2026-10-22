// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::layout::{BoxKind, LayoutBox, Rect};
use crate::style::{Content, Position};
use crate::text::Fonts;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrollbars {
    pub offset: (f32, f32),
    pub max: (f32, f32),
    pub show: (bool, bool),
}

const SCROLLBAR_WIDTH: f32 = 8.0;
const SCROLLBAR_MIN_THUMB: f32 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bar {
    pub track: Rect,
    pub thumb: Rect,
    pub vertical: bool,
}

impl Scrollbars {
    pub fn bars(&self, area: Rect) -> Vec<Bar> {
        let corner = if self.show.0 && self.show.1 {
            SCROLLBAR_WIDTH
        } else {
            0.0
        };
        let mut bars = Vec::new();
        if self.show.1 {
            let track = Rect {
                x: area.x + area.width - SCROLLBAR_WIDTH,
                y: area.y,
                width: SCROLLBAR_WIDTH,
                height: (area.height - corner).max(0.0),
            };
            bars.push(bar(track, true, area.height, self.offset.1, self.max.1));
        }
        if self.show.0 {
            let track = Rect {
                x: area.x,
                y: area.y + area.height - SCROLLBAR_WIDTH,
                width: (area.width - corner).max(0.0),
                height: SCROLLBAR_WIDTH,
            };
            bars.push(bar(track, false, area.width, self.offset.0, self.max.0));
        }
        bars
    }
}

fn bar(track: Rect, vertical: bool, view: f32, offset: f32, max: f32) -> Bar {
    let length = if vertical { track.height } else { track.width };
    let size = (length * view / (view + max))
        .max(SCROLLBAR_MIN_THUMB)
        .min(length);
    let travel = if max > 0.0 {
        offset / max * (length - size)
    } else {
        0.0
    };
    let thumb = if vertical {
        Rect {
            y: track.y + travel,
            height: size,
            ..track
        }
    } else {
        Rect {
            x: track.x + travel,
            width: size,
            ..track
        }
    };
    Bar {
        track,
        thumb,
        vertical,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Grab {
    pub index: usize,
    pub vertical: bool,
    anchor: f32,
}

impl Scrollbars {
    pub fn grab(&self, area: Rect, index: usize, x: f32, y: f32) -> Option<Grab> {
        self.bars(area).into_iter().find_map(|bar| {
            let thumb = bar.thumb;
            let inside = x >= thumb.x
                && x < thumb.x + thumb.width
                && y >= thumb.y
                && y < thumb.y + thumb.height;
            inside.then_some(Grab {
                index,
                vertical: bar.vertical,
                anchor: if bar.vertical {
                    y - thumb.y
                } else {
                    x - thumb.x
                },
            })
        })
    }

    pub fn dragged_offset(&self, area: Rect, grab: Grab, x: f32, y: f32) -> Option<f32> {
        let bar = self
            .bars(area)
            .into_iter()
            .find(|bar| bar.vertical == grab.vertical)?;
        let (track_start, track_length, thumb_length, pointer, max) = if grab.vertical {
            (
                bar.track.y,
                bar.track.height,
                bar.thumb.height,
                y,
                self.max.1,
            )
        } else {
            (bar.track.x, bar.track.width, bar.thumb.width, x, self.max.0)
        };
        let space = track_length - thumb_length;
        if space <= 0.0 {
            return Some(0.0);
        }
        let travel = (pointer - grab.anchor - track_start).clamp(0.0, space);
        Some(travel / space * max)
    }
}

pub type Offsets = HashMap<usize, (f32, f32)>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scroller {
    pub offset: (f32, f32),
    pub max: (f32, f32),
    pub axes: (bool, bool),
}

impl LayoutBox<'_> {
    pub fn scroll_size(&self, fonts: &Fonts) -> (f32, f32) {
        let padding = self.dimensions.padding_box();
        let content = self.dimensions.content;
        let mut extent = (padding.x + padding.width, content.y + content.height);
        self.extend_overflow(fonts, &mut extent);
        let (right, bottom) = extent;
        (
            right - padding.x,
            (bottom + self.dimensions.padding.bottom - padding.y).max(padding.height),
        )
    }

    pub fn apply_scrolls(&mut self, offsets: &Offsets, fonts: &Fonts) -> Vec<Scroller> {
        let mut scrollers = Vec::new();
        self.scroll_walk(offsets, fonts, &mut scrollers);
        scrollers
    }

    fn scroll_axes(&self) -> Option<(bool, bool)> {
        let BoxKind::Block(style) = self.kind else {
            return None;
        };
        let is_root = matches!(
            style.content,
            Content::Element(element) if element.tag_name == "html" || element.tag_name == "body"
        );
        let axes = style.scroll_axes();
        (!is_root && (axes.0 || axes.1)).then_some(axes)
    }

    fn scroll_walk(&mut self, offsets: &Offsets, fonts: &Fonts, scrollers: &mut Vec<Scroller>) {
        let container = self.scroll_axes().map(|axes| {
            let always = match self.kind {
                BoxKind::Block(style) => style.scrollbars_always(),
                BoxKind::Anonymous(_) => (false, false),
            };
            let padding = self.dimensions.padding_box();
            let (width, height) = self.scroll_size(fonts);
            let max = (
                if axes.0 {
                    (width - padding.width).max(0.0)
                } else {
                    0.0
                },
                if axes.1 {
                    (height - padding.height).max(0.0)
                } else {
                    0.0
                },
            );
            let index = scrollers.len();
            let wanted = offsets.get(&index).copied().unwrap_or((0.0, 0.0));
            let offset = (wanted.0.clamp(0.0, max.0), wanted.1.clamp(0.0, max.1));
            scrollers.push(Scroller { offset, max, axes });
            self.scrollbars = Some(Scrollbars {
                offset,
                max,
                show: (
                    axes.0 && (always.0 || max.0 > 0.0),
                    axes.1 && (always.1 || max.1 > 0.0),
                ),
            });
            offset
        });
        for child in &mut self.children {
            child.scroll_walk(offsets, fonts, scrollers);
        }
        if let Some((x, y)) = container {
            if x != 0.0 || y != 0.0 {
                self.shift_content(-x, -y);
            }
        }
    }

    fn shift_content(&mut self, dx: f32, dy: f32) {
        let content = self.dimensions.content;
        self.translate(dx, dy);
        self.dimensions.content = content;
    }

    pub fn scroller_bars(&self) -> Vec<(Rect, Scrollbars)> {
        let mut found = Vec::new();
        self.bars_walk(&mut found);
        found
    }

    fn bars_walk(&self, found: &mut Vec<(Rect, Scrollbars)>) {
        if self.scroll_axes().is_some() {
            if let Some(bars) = self.scrollbars {
                found.push((self.dimensions.padding_box(), bars));
            }
        }
        for child in &self.children {
            child.bars_walk(found);
        }
    }

    pub fn scrollers_at(&self, x: f32, y: f32) -> Vec<usize> {
        let mut hits = Vec::new();
        let mut index = 0;
        self.hit_walk(x, y, true, &mut index, &mut hits);
        hits
    }

    fn hit_walk(&self, x: f32, y: f32, visible: bool, index: &mut usize, hits: &mut Vec<usize>) {
        let mut visible = visible;
        if self.scroll_axes().is_some() {
            if visible && contains(self, x, y) {
                hits.push(*index);
            } else {
                visible = false;
            }
            *index += 1;
        } else if let BoxKind::Block(style) = self.kind {
            if style.establishes_clip() && !contains(self, x, y) {
                visible = false;
            }
        }
        for child in &self.children {
            child.hit_walk(x, y, visible, index, hits);
        }
    }

    fn extend_overflow(&self, fonts: &Fonts, extent: &mut (f32, f32)) {
        for fragment in &self.fragments {
            let width = fonts.width(fragment.face, &fragment.text, fragment.font_size)
                + fragment.letter_spacing * fragment.text.chars().count() as f32;
            reach(
                extent,
                fragment.x + width,
                fragment.baseline + fonts.descent(fragment.face, fragment.font_size),
            );
        }
        for image in &self.images {
            reach(
                extent,
                image.rect.x + image.rect.width,
                image.rect.y + image.rect.height,
            );
        }
        for inline_box in &self.inline_boxes {
            let rect = inline_box.rect;
            reach(extent, rect.x + rect.width, rect.y + rect.height);
        }
        for child in &self.children {
            let mut clipped = false;
            if let BoxKind::Block(style) = child.kind {
                if style.position() == Position::Fixed {
                    continue;
                }
                clipped = style.establishes_clip();
            }
            let border = child.dimensions.border_box();
            reach(extent, border.x + border.width, border.y + border.height);
            if !clipped {
                child.extend_overflow(fonts, extent);
            }
        }
    }
}

fn reach(extent: &mut (f32, f32), x: f32, y: f32) {
    extent.0 = extent.0.max(x);
    extent.1 = extent.1.max(y);
}

fn contains(layout_box: &LayoutBox, x: f32, y: f32) -> bool {
    let rect = layout_box.dimensions.padding_box();
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
