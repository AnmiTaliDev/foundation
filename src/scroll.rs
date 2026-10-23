// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::css::Color;
use crate::layout::{BoxKind, LayoutBox, Rect};
use crate::snap::{SnapAlign, SnapPoint, Strictness};
use crate::style::{Content, Position, SCROLLBAR_THUMB, SCROLLBAR_TRACK};
use crate::text::Fonts;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Scrollbars {
    pub offset: (f32, f32),
    pub max: (f32, f32),
    pub show: (bool, bool),
    pub width: f32,
    pub rtl: bool,
    pub thumb: Color,
    pub track: Color,
}

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
            self.width
        } else {
            0.0
        };
        let mut bars = Vec::new();
        if self.show.1 {
            let track = Rect {
                x: if self.rtl {
                    area.x
                } else {
                    area.x + area.width - self.width
                },
                y: area.y,
                width: self.width,
                height: (area.height - corner).max(0.0),
            };
            bars.push(bar(
                track,
                true,
                area.height,
                self.offset.1,
                self.max.1,
                false,
            ));
        }
        if self.show.0 {
            let track = Rect {
                x: if self.rtl { area.x + corner } else { area.x },
                y: area.y + area.height - self.width,
                width: (area.width - corner).max(0.0),
                height: self.width,
            };
            bars.push(bar(
                track,
                false,
                area.width,
                self.offset.0,
                self.max.0,
                self.rtl,
            ));
        }
        bars
    }
}

fn bar(track: Rect, vertical: bool, view: f32, offset: f32, max: f32, reversed: bool) -> Bar {
    let length = if vertical { track.height } else { track.width };
    let size = (length * view / (view + max))
        .max(SCROLLBAR_MIN_THUMB)
        .min(length);
    let travel = if max > 0.0 {
        offset / max * (length - size)
    } else {
        0.0
    };
    let travel = if reversed {
        length - size - travel
    } else {
        travel
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
        let fraction = travel / space;
        let fraction = if !grab.vertical && self.rtl {
            1.0 - fraction
        } else {
            fraction
        };
        Some(fraction * max)
    }
}

pub type Offsets = HashMap<usize, (f32, f32)>;

#[derive(Debug, Clone, PartialEq)]
pub struct Scroller {
    pub offset: (f32, f32),
    pub max: (f32, f32),
    pub axes: (bool, bool),
    pub rtl: bool,
    pub port: (f32, f32),
    pub snap: (Option<Strictness>, Option<Strictness>),
    pub points: (Vec<SnapPoint>, Vec<SnapPoint>),
}

impl LayoutBox<'_> {
    pub fn scroll_size(&self, fonts: &Fonts) -> (f32, f32) {
        let padding = self.dimensions.padding_box();
        let content = self.dimensions.content;
        let mut extent = (
            padding.x + padding.width,
            content.y + content.height,
            padding.x,
        );
        self.extend_overflow(fonts, &mut extent);
        let (right, bottom, left) = extent;
        let rtl = matches!(self.kind, BoxKind::Block(style) if style.rtl());
        (
            if rtl {
                padding.x + padding.width - left
            } else {
                right - padding.x
            },
            (bottom + self.dimensions.padding.bottom - padding.y).max(padding.height),
        )
    }

    pub fn apply_scrolls(&mut self, offsets: &Offsets, fonts: &Fonts) -> Vec<Scroller> {
        let mut scrollers = Vec::new();
        self.scroll_walk(offsets, fonts, &mut scrollers);
        self.stick_horizontally();
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
            let (always, bar_width, colors) = match self.kind {
                BoxKind::Block(style) => (
                    style.scrollbars_always(),
                    style.scrollbar_width(),
                    style.scrollbar_colors(),
                ),
                BoxKind::Anonymous(_) => ((false, false), 8.0, (SCROLLBAR_THUMB, SCROLLBAR_TRACK)),
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
            let snap = match self.kind {
                BoxKind::Block(style) => style.scroll_snap_type(),
                BoxKind::Anonymous(_) => (None, None),
            };
            let mut points = (Vec::new(), Vec::new());
            if snap.0.is_some() || snap.1.is_some() {
                let mut found = (Vec::new(), Vec::new());
                let spacing = match self.kind {
                    BoxKind::Block(style) => style.scroll_padding((padding.width, padding.height)),
                    BoxKind::Anonymous(_) => [0.0; 4],
                };
                let port = Rect {
                    x: padding.x + spacing[3],
                    y: padding.y + spacing[0],
                    width: padding.width - spacing[1] - spacing[3],
                    height: padding.height - spacing[0] - spacing[2],
                };
                self.collect_snap_points(port, max, &mut found);
                points = (
                    if snap.0.is_some() {
                        sorted(found.0)
                    } else {
                        Vec::new()
                    },
                    if snap.1.is_some() {
                        sorted(found.1)
                    } else {
                        Vec::new()
                    },
                );
            }
            let rtl = matches!(self.kind, BoxKind::Block(style) if style.rtl());
            scrollers.push(Scroller {
                offset,
                max,
                axes,
                rtl,
                port: (padding.width, padding.height),
                snap,
                points,
            });
            self.scrollbars = Some(Scrollbars {
                offset,
                max,
                show: (
                    bar_width > 0.0 && axes.0 && (always.0 || max.0 > 0.0),
                    bar_width > 0.0 && axes.1 && (always.1 || max.1 > 0.0),
                ),
                width: bar_width,
                rtl,
                thumb: colors.0,
                track: colors.1,
            });
            offset
        });
        for child in &mut self.children {
            child.scroll_walk(offsets, fonts, scrollers);
        }
        if let Some((x, y)) = container {
            if x != 0.0 || y != 0.0 {
                let rtl = matches!(self.kind, BoxKind::Block(style) if style.rtl());
                self.shift_content(if rtl { x } else { -x }, -y);
            }
        }
    }

    fn collect_snap_points(
        &self,
        port: Rect,
        max: (f32, f32),
        found: &mut (Vec<SnapPoint>, Vec<SnapPoint>),
    ) {
        for child in &self.children {
            let BoxKind::Block(style) = child.kind else {
                child.collect_snap_points(port, max, found);
                continue;
            };
            let (align_x, align_y) = style.scroll_snap_align();
            let margin = style.scroll_margin();
            let border = child.dimensions.border_box();
            let area = Rect {
                x: border.x - margin[3],
                y: border.y - margin[0],
                width: border.width + margin[1] + margin[3],
                height: border.height + margin[0] + margin[2],
            };
            let always = style.scroll_snap_stop_always();
            let x = snap_position(align_x, (area.x, area.width), (port.x, port.width), max.0);
            let y = snap_position(align_y, (area.y, area.height), (port.y, port.height), max.1);
            found
                .0
                .extend(x.map(|position| SnapPoint { position, always }));
            found
                .1
                .extend(y.map(|position| SnapPoint { position, always }));
            if child.scroll_axes().is_none() {
                child.collect_snap_points(port, max, found);
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

    fn extend_overflow(&self, fonts: &Fonts, extent: &mut (f32, f32, f32)) {
        for fragment in &self.fragments {
            let width = fonts.width(fragment.face, &fragment.text, fragment.font_size)
                + fragment.letter_spacing * fragment.text.chars().count() as f32;
            reach(
                extent,
                (fragment.x, fragment.x + width),
                fragment.baseline + fonts.descent(fragment.face, fragment.font_size),
            );
        }
        for image in &self.images {
            reach(
                extent,
                (image.rect.x, image.rect.x + image.rect.width),
                image.rect.y + image.rect.height,
            );
        }
        for inline_box in &self.inline_boxes {
            let rect = inline_box.rect;
            reach(extent, (rect.x, rect.x + rect.width), rect.y + rect.height);
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
            reach(
                extent,
                (border.x, border.x + border.width),
                border.y + border.height,
            );
            if !clipped {
                child.extend_overflow(fonts, extent);
            }
        }
    }
}

fn snap_position(align: SnapAlign, area: (f32, f32), port: (f32, f32), limit: f32) -> Option<f32> {
    let position = match align {
        SnapAlign::None => return None,
        SnapAlign::Start => area.0 - port.0,
        SnapAlign::Center => area.0 + area.1 / 2.0 - (port.0 + port.1 / 2.0),
        SnapAlign::End => area.0 + area.1 - (port.0 + port.1),
    };
    Some(position.clamp(0.0, limit))
}

fn sorted(mut points: Vec<SnapPoint>) -> Vec<SnapPoint> {
    points.sort_by(|a, b| a.position.total_cmp(&b.position));
    points.dedup_by(|later, earlier| {
        let same = later.position == earlier.position;
        if same {
            earlier.always |= later.always;
        }
        same
    });
    points
}

fn reach(extent: &mut (f32, f32, f32), x: (f32, f32), y: f32) {
    extent.0 = extent.0.max(x.1);
    extent.1 = extent.1.max(y);
    extent.2 = extent.2.min(x.0);
}

fn contains(layout_box: &LayoutBox, x: f32, y: f32) -> bool {
    let rect = layout_box.dimensions.padding_box();
    x >= rect.x && x < rect.x + rect.width && y >= rect.y && y < rect.y + rect.height
}
