// SPDX-License-Identifier: GPL-3.0-only

use std::rc::Rc;

use crate::color::Rgba;
use crate::composite;
use crate::gradient::Gradient;
use crate::text::Fonts;
use crate::transform::Matrix;

use crate::css::Color;
use crate::inline::{Fragment, ImageFragment};
use crate::layout::{BoxKind, LayoutBox, Rect};
use crate::picture::Picture;
use crate::radius::{Radii, Round};
use crate::shadow;
use crate::style::{BackgroundBox, Content, Position, StrokeStyle, StyledNode};

const BLACK: Color = Color { r: 0, g: 0, b: 0 };

const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidRect {
    pub color: Color,
    pub alpha: u8,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    Rect(SolidRect),
    Text(Fragment),
    Image(ImageFragment),
    Tiles(Tiles),
    GradientTiles(GradientTiles),
    BeginShift(f32),
    BeginClip(Clip),
    BeginRound(Round),
    Shadow(ShadowPaint),
    BeginGroup(Group),
    EndGroup,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowPaint {
    pub shape: Round,
    pub container: Round,
    pub bounds: Rect,
    pub blur: f32,
    pub color: Color,
    pub alpha: u8,
    pub inset: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clip {
    pub rect: Rect,
    pub radii: Radii,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Group {
    pub opacity: f32,
    pub transform: Option<Matrix>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientTiles {
    pub clip: Rect,
    pub tile: Rect,
    pub repeat_x: bool,
    pub repeat_y: bool,
    pub gradient: Rc<Gradient>,
    pub font_size: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tiles {
    pub clip: Rect,
    pub tile: Rect,
    pub repeat_x: bool,
    pub repeat_y: bool,
    pub picture: Rc<Picture>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color>,
    pub(crate) clip: (usize, usize, usize, usize),
    rounds: Vec<Round>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Canvas {
        Canvas::filled(width, height, WHITE)
    }

    pub fn filled(width: usize, height: usize, color: Color) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![color; width * height],
            clip: (0, 0, width, height),
            rounds: Vec::new(),
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> Color {
        self.pixels[y * self.width + x]
    }

    pub(crate) fn coverage(&self, x: usize, y: usize) -> f32 {
        let mut coverage = 1.0;
        for round in &self.rounds {
            coverage *= round.coverage(x as f32 + 0.5, y as f32 + 0.5);
        }
        coverage
    }

    fn put(&mut self, x: usize, y: usize, color: Color, alpha: u8) {
        let alpha = if self.rounds.is_empty() {
            alpha
        } else {
            (f32::from(alpha) * self.coverage(x, y)).round() as u8
        };
        if alpha == 0 {
            return;
        }
        let index = y * self.width + x;
        self.pixels[index] = if alpha == 255 {
            color
        } else {
            blend(self.pixels[index], color, alpha)
        };
    }

    fn draw_shadow(&mut self, paint: &ShadowPaint) {
        let bounds = paint.bounds;
        let x_start = bounds
            .x
            .floor()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let x_end = (bounds.x + bounds.width)
            .ceil()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_start = bounds
            .y
            .floor()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        let y_end = (bounds.y + bounds.height)
            .ceil()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        for y in y_start..y_end {
            for x in x_start..x_end {
                let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
                let shape = shadow::blurred_coverage(paint.shape.distance(px, py), paint.blur);
                let container = paint.container.coverage(px, py);
                let amount = if paint.inset {
                    (1.0 - shape) * container
                } else {
                    shape * (1.0 - container)
                };
                let alpha = (f32::from(paint.alpha) * amount).round() as u8;
                if alpha > 0 {
                    self.put(x, y, paint.color, alpha);
                }
            }
        }
    }

    fn fill(&mut self, item: &SolidRect) {
        let x0 = item
            .rect
            .x
            .clamp(self.clip.0 as f32, self.clip.2 as f32)
            .round() as usize;
        let y0 = item
            .rect
            .y
            .clamp(self.clip.1 as f32, self.clip.3 as f32)
            .round() as usize;
        let x1 = (item.rect.x + item.rect.width)
            .clamp(self.clip.0 as f32, self.clip.2 as f32)
            .round() as usize;
        let y1 = (item.rect.y + item.rect.height)
            .clamp(self.clip.1 as f32, self.clip.3 as f32)
            .round() as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                self.put(x, y, item.color, item.alpha);
            }
        }
    }

    fn draw_text(&mut self, fragment: &Fragment, fonts: &Fonts) {
        let mut pen_x = fragment.x;
        let baseline = fragment.baseline.round() as i64;
        for c in fragment.text.chars() {
            let (metrics, bitmap) = fonts.font(fragment.face).rasterize(c, fragment.font_size);
            let left = (pen_x + metrics.xmin as f32).round() as i64;
            let top = baseline - metrics.height as i64 - i64::from(metrics.ymin);
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let coverage = (u32::from(bitmap[row * metrics.width + col])
                        * u32::from(fragment.alpha)
                        / 255) as u8;
                    let x = left + col as i64;
                    let y = top + row as i64;
                    if coverage == 0
                        || x < self.clip.0 as i64
                        || y < self.clip.1 as i64
                        || x >= self.clip.2 as i64
                        || y >= self.clip.3 as i64
                    {
                        continue;
                    }
                    self.put(x as usize, y as usize, fragment.color, coverage);
                }
            }
            pen_x += fonts.advance(fragment.face, c, fragment.font_size)
                + fragment.letter_spacing
                + if c == ' ' { fragment.word_spacing } else { 0.0 };
        }
    }

    fn draw_picture(&mut self, rect: &Rect, picture: &Picture) {
        if rect.width <= 0.0 || rect.height <= 0.0 || picture.width == 0 || picture.height == 0 {
            return;
        }
        let x_start = rect.x.round().clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_start = rect.y.round().clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        let x_end = (rect.x + rect.width)
            .round()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_end = (rect.y + rect.height)
            .round()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        let last_column = picture.width as usize - 1;
        let last_row = picture.height as usize - 1;
        for y in y_start..y_end {
            let source_y = (((y as f32 + 0.5 - rect.y) / rect.height) * picture.height as f32)
                .max(0.0) as usize;
            let source_y = source_y.min(last_row);
            for x in x_start..x_end {
                let source_x = (((x as f32 + 0.5 - rect.x) / rect.width) * picture.width as f32)
                    .max(0.0) as usize;
                let source_x = source_x.min(last_column);
                let offset = (source_y * picture.width as usize + source_x) * 4;
                let alpha = picture.rgba[offset + 3];
                if alpha == 0 {
                    continue;
                }
                let color = Color {
                    r: picture.rgba[offset],
                    g: picture.rgba[offset + 1],
                    b: picture.rgba[offset + 2],
                };
                self.put(x, y, color, alpha);
            }
        }
    }

    fn draw_gradient(&mut self, tiles: &GradientTiles) {
        let tile = tiles.tile;
        if tile.width <= 0.0 || tile.height <= 0.0 {
            return;
        }
        let prepared = tiles
            .gradient
            .prepare(tile.width, tile.height, tiles.font_size);
        let x_start = tiles
            .clip
            .x
            .round()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_start = tiles
            .clip
            .y
            .round()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        let x_end = (tiles.clip.x + tiles.clip.width)
            .round()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_end = (tiles.clip.y + tiles.clip.height)
            .round()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        for y in y_start..y_end {
            let Some(dy) = tile_offset(y as f32 + 0.5 - tile.y, tile.height, tiles.repeat_y) else {
                continue;
            };
            for x in x_start..x_end {
                let Some(dx) = tile_offset(x as f32 + 0.5 - tile.x, tile.width, tiles.repeat_x)
                else {
                    continue;
                };
                let (color, alpha) = prepared.color_at(dx, dy);
                if alpha == 0 {
                    continue;
                }
                self.put(x, y, color, alpha);
            }
        }
    }

    fn draw_tiles(&mut self, tiles: &Tiles) {
        let tile = tiles.tile;
        let picture = &tiles.picture;
        if tile.width <= 0.0 || tile.height <= 0.0 || picture.width == 0 || picture.height == 0 {
            return;
        }
        let x_start = tiles
            .clip
            .x
            .round()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_start = tiles
            .clip
            .y
            .round()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        let x_end = (tiles.clip.x + tiles.clip.width)
            .round()
            .clamp(self.clip.0 as f32, self.clip.2 as f32) as usize;
        let y_end = (tiles.clip.y + tiles.clip.height)
            .round()
            .clamp(self.clip.1 as f32, self.clip.3 as f32) as usize;
        let last_column = picture.width as usize - 1;
        let last_row = picture.height as usize - 1;
        for y in y_start..y_end {
            let mut dy = y as f32 + 0.5 - tile.y;
            if tiles.repeat_y {
                dy = dy.rem_euclid(tile.height);
            } else if dy < 0.0 || dy >= tile.height {
                continue;
            }
            let source_y = ((dy / tile.height * picture.height as f32) as usize).min(last_row);
            for x in x_start..x_end {
                let mut dx = x as f32 + 0.5 - tile.x;
                if tiles.repeat_x {
                    dx = dx.rem_euclid(tile.width);
                } else if dx < 0.0 || dx >= tile.width {
                    continue;
                }
                let source_x = ((dx / tile.width * picture.width as f32) as usize).min(last_column);
                let offset = (source_y * picture.width as usize + source_x) * 4;
                let alpha = picture.rgba[offset + 3];
                if alpha == 0 {
                    continue;
                }
                let color = Color {
                    r: picture.rgba[offset],
                    g: picture.rgba[offset + 1],
                    b: picture.rgba[offset + 2],
                };
                self.put(x, y, color, alpha);
            }
        }
    }

    pub fn to_argb(&self) -> Vec<u32> {
        let mut out = Vec::with_capacity(self.pixels.len());
        for pixel in &self.pixels {
            out.push((u32::from(pixel.r) << 16) | (u32::from(pixel.g) << 8) | u32::from(pixel.b));
        }
        out
    }

    pub fn to_ppm(&self) -> Vec<u8> {
        let mut out = format!("P6\n{} {}\n255\n", self.width, self.height).into_bytes();
        out.reserve(self.pixels.len() * 3);
        for pixel in &self.pixels {
            out.push(pixel.r);
            out.push(pixel.g);
            out.push(pixel.b);
        }
        out
    }
}

fn tile_offset(offset: f32, size: f32, repeat: bool) -> Option<f32> {
    if repeat {
        Some(offset.rem_euclid(size))
    } else if offset < 0.0 || offset >= size {
        None
    } else {
        Some(offset)
    }
}

fn blend(background: Color, foreground: Color, coverage: u8) -> Color {
    Color {
        r: blend_channel(background.r, foreground.r, coverage),
        g: blend_channel(background.g, foreground.g, coverage),
        b: blend_channel(background.b, foreground.b, coverage),
    }
}

fn blend_channel(background: u8, foreground: u8, coverage: u8) -> u8 {
    let coverage = u32::from(coverage);
    ((u32::from(background) * (255 - coverage) + u32::from(foreground) * coverage + 127) / 255)
        as u8
}

type DisplayList = Vec<(DisplayCommand, bool)>;

pub fn paint(
    root: &LayoutBox,
    width: usize,
    height: usize,
    scroll_y: f32,
    fonts: &Fonts,
) -> Canvas {
    let mut list = DisplayList::new();
    let view = View {
        scroll_y,
        height: height as f32,
    };
    paint_stacking(root, false, Rect::default(), view, &mut list);
    let mut canvas = Canvas::filled(width, height, canvas_background(root));
    run(&list, &mut canvas, scroll_y, fonts);
    canvas
}

fn group_end(list: &[(DisplayCommand, bool)]) -> usize {
    let mut depth = 0usize;
    for (index, (command, _)) in list.iter().enumerate() {
        match command {
            DisplayCommand::BeginGroup(_)
            | DisplayCommand::BeginShift(_)
            | DisplayCommand::BeginClip(_)
            | DisplayCommand::BeginRound(_) => depth += 1,
            DisplayCommand::EndGroup => {
                if depth == 0 {
                    return index;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    list.len()
}

fn run(list: &[(DisplayCommand, bool)], canvas: &mut Canvas, scroll_y: f32, fonts: &Fonts) {
    let mut index = 0;
    while index < list.len() {
        let (command, fixed) = &list[index];
        index += 1;
        let offset = if *fixed { 0.0 } else { scroll_y };
        match command {
            DisplayCommand::Rect(item) => {
                let mut shifted = *item;
                shifted.rect.y -= offset;
                canvas.fill(&shifted);
            }
            DisplayCommand::Text(fragment) => {
                let mut shifted = fragment.clone();
                shifted.baseline -= offset;
                canvas.draw_text(&shifted, fonts);
            }
            DisplayCommand::Image(image) => {
                let mut rect = image.rect;
                rect.y -= offset;
                canvas.draw_picture(&rect, &image.picture);
            }
            DisplayCommand::Tiles(tiles) => {
                let mut shifted = tiles.clone();
                shifted.clip.y -= offset;
                shifted.tile.y -= offset;
                canvas.draw_tiles(&shifted);
            }
            DisplayCommand::GradientTiles(tiles) => {
                let mut shifted = tiles.clone();
                shifted.clip.y -= offset;
                shifted.tile.y -= offset;
                canvas.draw_gradient(&shifted);
            }
            DisplayCommand::BeginGroup(group) => {
                let end = index + group_end(&list[index..]);
                let inner = &list[index..end];
                index = (end + 1).min(list.len());
                match group.transform {
                    None => {
                        let backdrop = canvas.pixels.clone();
                        run(inner, canvas, scroll_y, fonts);
                        composite::fade(canvas, &backdrop, group.opacity);
                    }
                    Some(matrix) => {
                        let mut on_black = Canvas::filled(canvas.width, canvas.height, BLACK);
                        let mut on_white = Canvas::filled(canvas.width, canvas.height, WHITE);
                        run(inner, &mut on_black, scroll_y, fonts);
                        run(inner, &mut on_white, scroll_y, fonts);
                        let [a, b, c, d, e, f] = matrix;
                        let shifted = [a, b, c, d, e + c * offset, f + d * offset - offset];
                        composite::draw_layer(
                            canvas,
                            &on_black,
                            &on_white,
                            &shifted,
                            group.opacity,
                        );
                    }
                }
            }
            DisplayCommand::BeginShift(shift) => {
                let end = index + group_end(&list[index..]);
                run(&list[index..end], canvas, scroll_y - shift, fonts);
                index = (end + 1).min(list.len());
            }
            DisplayCommand::Shadow(paint) => {
                let mut shifted = *paint;
                shifted.shape.rect.y -= offset;
                shifted.container.rect.y -= offset;
                shifted.bounds.y -= offset;
                canvas.draw_shadow(&shifted);
            }
            DisplayCommand::BeginRound(round) => {
                let end = index + group_end(&list[index..]);
                let mut shifted = *round;
                shifted.rect.y -= offset;
                canvas.rounds.push(shifted);
                run(&list[index..end], canvas, scroll_y, fonts);
                canvas.rounds.pop();
                index = (end + 1).min(list.len());
            }
            DisplayCommand::BeginClip(Clip { rect, radii }) => {
                let end = index + group_end(&list[index..]);
                let saved = canvas.clip;
                let x0 = (rect.x.round().max(0.0) as usize).clamp(saved.0, saved.2);
                let y0 = ((rect.y - offset).round().max(0.0) as usize).clamp(saved.1, saved.3);
                let x1 = ((rect.x + rect.width).round().max(0.0) as usize).clamp(x0, saved.2);
                let y1 = (((rect.y + rect.height) - offset).round().max(0.0) as usize)
                    .clamp(y0, saved.3);
                canvas.clip = (x0, y0, x1, y1);
                let rounded = !radii.is_zero();
                if rounded {
                    let mut shifted = *rect;
                    shifted.y -= offset;
                    canvas.rounds.push(Round {
                        rect: shifted,
                        radii: *radii,
                        inverted: false,
                    });
                }
                run(&list[index..end], canvas, scroll_y, fonts);
                if rounded {
                    canvas.rounds.pop();
                }
                canvas.clip = saved;
                index = (end + 1).min(list.len());
            }
            DisplayCommand::EndGroup => {}
        }
    }
}

fn canvas_background(root: &LayoutBox) -> Color {
    let over_white = |style: &crate::style::StyledNode| {
        style
            .rgba("background-color")
            .filter(|rgba| rgba.alpha > 0)
            .map(|rgba| blend(WHITE, rgba.color, rgba.alpha))
    };
    if let BoxKind::Block(style) = root.kind {
        if let Some(color) = over_white(style) {
            return color;
        }
    }
    for child in &root.children {
        if let BoxKind::Block(style) = child.kind {
            if let Content::Element(element) = style.content {
                if element.tag_name == "body" {
                    return over_white(style).unwrap_or(WHITE);
                }
            }
        }
    }
    WHITE
}

#[derive(Clone, Copy)]
struct View {
    scroll_y: f32,
    height: f32,
}

struct Positioned<'a, 'b> {
    container: Rect,
    z_index: i32,
    creates_context: bool,
    fixed: bool,
    layout_box: &'b LayoutBox<'a>,
    clips: Vec<Clip>,
}

fn is_positioned(layout_box: &LayoutBox) -> bool {
    match layout_box.kind {
        BoxKind::Block(style) => style.is_positioned(),
        BoxKind::Anonymous(_) => false,
    }
}

fn is_fixed(layout_box: &LayoutBox) -> bool {
    match layout_box.kind {
        BoxKind::Block(style) => style.position() == Position::Fixed,
        BoxKind::Anonymous(_) => false,
    }
}

fn emit(list: &mut DisplayList, fixed: bool, commands: Vec<DisplayCommand>) {
    for command in commands {
        list.push((command, fixed));
    }
}

fn decoration_commands(layout_box: &LayoutBox) -> Vec<DisplayCommand> {
    let mut commands = Vec::new();
    paint_shadows(layout_box, false, &mut commands);
    paint_background(layout_box, &mut commands);
    paint_shadows(layout_box, true, &mut commands);
    paint_borders(layout_box, &mut commands);
    commands
}

fn content_commands(layout_box: &LayoutBox) -> Vec<DisplayCommand> {
    let mut commands = Vec::new();
    for inline_box in &layout_box.inline_boxes {
        if let Some(color) = inline_box.background {
            commands.push(DisplayCommand::Rect(SolidRect {
                color,
                alpha: 255,
                rect: inline_box.rect,
            }));
        }
        if let Some(color) = inline_box.border_color {
            for rect in border_edges(&inline_box.rect, inline_box.border) {
                commands.push(DisplayCommand::Rect(SolidRect {
                    color,
                    alpha: 255,
                    rect,
                }));
            }
        }
    }
    let rule_commands = |over_text: bool| {
        layout_box
            .rules
            .iter()
            .filter(move |rule| rule.over_text == over_text)
            .map(|rule| {
                DisplayCommand::Rect(SolidRect {
                    color: rule.color,
                    alpha: rule.alpha,
                    rect: rule.rect,
                })
            })
    };
    commands.extend(rule_commands(false));
    for fragment in &layout_box.fragments {
        commands.push(DisplayCommand::Text(fragment.clone()));
    }
    commands.extend(rule_commands(true));
    for image in &layout_box.images {
        if image.drawn == image.rect {
            commands.push(DisplayCommand::Image(image.clone()));
            continue;
        }
        commands.push(DisplayCommand::BeginClip(Clip {
            rect: image.rect,
            radii: Radii::default(),
        }));
        commands.push(DisplayCommand::Image(ImageFragment {
            rect: image.drawn,
            ..image.clone()
        }));
        commands.push(DisplayCommand::EndGroup);
    }
    commands
}

fn collect_positioned<'a, 'b>(
    children: &'b [LayoutBox<'a>],
    fixed: bool,
    container: Rect,
    clips: &[(Clip, bool)],
    out: &mut Vec<Positioned<'a, 'b>>,
) {
    for child in children {
        let BoxKind::Block(style) = child.kind else {
            collect_positioned(&child.children, fixed, container, clips, out);
            continue;
        };
        if !style.is_positioned() {
            if !style.has_effect() {
                let mut inner = clips.to_vec();
                if let Some(clip) = clip_of(child) {
                    inner.push((clip, false));
                }
                collect_positioned(
                    &child.children,
                    fixed,
                    child.dimensions.content,
                    &inner,
                    out,
                );
            }
            continue;
        }
        let child_fixed = fixed || is_fixed(child);
        let z_index = style.z_index();
        let creates_context =
            z_index.is_some() || style.has_effect() || style.position() == Position::Sticky;
        let applying: Vec<Clip> = match style.position() {
            Position::Fixed => Vec::new(),
            Position::Absolute => clips
                .iter()
                .filter(|(_, absolute)| *absolute)
                .map(|(clip, _)| *clip)
                .collect(),
            _ => clips.iter().map(|(clip, _)| *clip).collect(),
        };
        out.push(Positioned {
            container,
            z_index: z_index.unwrap_or(0),
            creates_context,
            fixed: child_fixed,
            layout_box: child,
            clips: applying,
        });
        if !creates_context {
            let mut inner: Vec<(Clip, bool)> =
                clips.iter().map(|(clip, _)| (*clip, true)).collect();
            if let Some(clip) = clip_of(child) {
                inner.push((clip, true));
            }
            collect_positioned(
                &child.children,
                child_fixed,
                child.dimensions.content,
                &inner,
                out,
            );
        }
    }
}

fn paint_flow(
    children: &[LayoutBox],
    fixed: bool,
    container: Rect,
    view: View,
    list: &mut DisplayList,
) {
    for child in children {
        if is_positioned(child) {
            continue;
        }
        if group_of(child).is_some() {
            paint_stacking(child, fixed, container, view, list);
            continue;
        }
        emit(list, fixed, decoration_commands(child));
        let clip = clip_of(child);
        begin_clip(list, fixed, clip);
        emit(list, fixed, content_commands(child));
        paint_flow(&child.children, fixed, child.dimensions.content, view, list);
        end_clip(list, fixed, clip);
        emit(list, fixed, outline_commands(child));
    }
}

fn paint_positioned(item: &Positioned, view: View, list: &mut DisplayList) {
    for clip in &item.clips {
        begin_clip(list, item.fixed, Some(*clip));
    }
    if item.creates_context {
        paint_stacking(item.layout_box, item.fixed, item.container, view, list);
    } else {
        emit(list, item.fixed, decoration_commands(item.layout_box));
        let clip = clip_of(item.layout_box);
        begin_clip(list, item.fixed, clip);
        emit(list, item.fixed, content_commands(item.layout_box));
        paint_flow(
            &item.layout_box.children,
            item.fixed,
            item.layout_box.dimensions.content,
            view,
            list,
        );
        end_clip(list, item.fixed, clip);
        emit(list, item.fixed, outline_commands(item.layout_box));
    }
    for clip in &item.clips {
        end_clip(list, item.fixed, Some(*clip));
    }
}

fn clip_of(layout_box: &LayoutBox) -> Option<Clip> {
    let BoxKind::Block(style) = layout_box.kind else {
        return None;
    };
    let is_root = matches!(
        style.content,
        Content::Element(element) if element.tag_name == "html" || element.tag_name == "body"
    );
    if is_root || !style.establishes_clip() {
        return None;
    }
    let d = &layout_box.dimensions;
    let border_box = d.border_box();
    let radii = style
        .border_radii(border_box.width, border_box.height)
        .shrink([d.border.top, d.border.right, d.border.bottom, d.border.left]);
    Some(Clip {
        rect: d.padding_box(),
        radii,
    })
}

fn begin_clip(list: &mut DisplayList, fixed: bool, clip: Option<Clip>) {
    if let Some(rect) = clip {
        list.push((DisplayCommand::BeginClip(rect), fixed));
    }
}

fn end_clip(list: &mut DisplayList, fixed: bool, clip: Option<Clip>) {
    if clip.is_some() {
        list.push((DisplayCommand::EndGroup, fixed));
    }
}

fn group_of(layout_box: &LayoutBox) -> Option<Group> {
    let BoxKind::Block(style) = layout_box.kind else {
        return None;
    };
    if !style.has_effect() {
        return None;
    }
    let border_box = layout_box.dimensions.border_box();
    Some(Group {
        opacity: style.opacity(),
        transform: style.transform_matrix(
            border_box.x,
            border_box.y,
            border_box.width,
            border_box.height,
        ),
    })
}

fn sticky_shift(layout_box: &LayoutBox, container: Rect, view: View) -> f32 {
    let BoxKind::Block(style) = layout_box.kind else {
        return 0.0;
    };
    if style.position() != Position::Sticky {
        return 0.0;
    }
    let border = layout_box.dimensions.border_box();
    let margin = layout_box.dimensions.margin_box();
    let top = style.length_or_auto("top", None, Some(view.height));
    let bottom = style.length_or_auto("bottom", None, Some(view.height));
    let from_top = top.map(|top| (view.scroll_y + top - border.y).max(0.0));
    let from_bottom = bottom
        .map(|bottom| (view.scroll_y + view.height - bottom - (border.y + border.height)).min(0.0));
    let shift = match (from_top, from_bottom) {
        (Some(shift), _) if shift > 0.0 => shift,
        (_, Some(shift)) => shift,
        _ => 0.0,
    };
    let lowest = container.y + container.height - (margin.y + margin.height);
    let highest = container.y - margin.y;
    shift.min(lowest).max(highest)
}

fn paint_stacking(
    layout_box: &LayoutBox,
    fixed: bool,
    container: Rect,
    view: View,
    list: &mut DisplayList,
) {
    let shift = if fixed {
        0.0
    } else {
        sticky_shift(layout_box, container, view)
    };
    if shift != 0.0 {
        list.push((DisplayCommand::BeginShift(shift), fixed));
    }
    let group = group_of(layout_box);
    if let Some(group) = group {
        list.push((DisplayCommand::BeginGroup(group), fixed));
    }
    paint_stacking_contents(layout_box, fixed, view, list);
    if group.is_some() {
        list.push((DisplayCommand::EndGroup, fixed));
    }
    if shift != 0.0 {
        list.push((DisplayCommand::EndGroup, fixed));
    }
}

fn paint_stacking_contents(
    layout_box: &LayoutBox,
    fixed: bool,
    view: View,
    list: &mut DisplayList,
) {
    emit(list, fixed, decoration_commands(layout_box));
    let clip = clip_of(layout_box);
    begin_clip(list, fixed, clip);
    let content = layout_box.dimensions.content;
    let mut positioned = Vec::new();
    collect_positioned(&layout_box.children, fixed, content, &[], &mut positioned);
    positioned.sort_by_key(|item| item.z_index);
    for item in positioned.iter().filter(|item| item.z_index < 0) {
        paint_positioned(item, view, list);
    }
    emit(list, fixed, content_commands(layout_box));
    paint_flow(&layout_box.children, fixed, content, view, list);
    for item in positioned.iter().filter(|item| item.z_index >= 0) {
        paint_positioned(item, view, list);
    }
    end_clip(list, fixed, clip);
    emit(list, fixed, outline_commands(layout_box));
}

fn paint_shadows(layout_box: &LayoutBox, inset: bool, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let d = &layout_box.dimensions;
    let border_box = d.border_box();
    let radii = box_radii(layout_box, style);
    let widths = [d.border.top, d.border.right, d.border.bottom, d.border.left];
    let font_size = style.font_size();
    for spec in style.box_shadows().iter().rev() {
        if spec.inset != inset {
            continue;
        }
        let x = spec.offset_x.resolve(font_size);
        let y = spec.offset_y.resolve(font_size);
        let blur = spec.blur.resolve(font_size);
        let spread = spec.spread.resolve(font_size);
        let Rgba { color, alpha } = spec.color.unwrap_or_else(|| Rgba {
            color: style.text_color(),
            alpha: 255,
        });
        if alpha == 0 {
            continue;
        }
        let grow = |rect: Rect, amount: f32| Rect {
            x: rect.x - amount,
            y: rect.y - amount,
            width: (rect.width + 2.0 * amount).max(0.0),
            height: (rect.height + 2.0 * amount).max(0.0),
        };
        let (shape, container, bounds) = if inset {
            let padding = d.padding_box();
            let inner = radii.shrink(widths);
            let mut hole = grow(padding, -spread);
            hole.x += x;
            hole.y += y;
            let hole_radii = Radii {
                corners: inner.corners.map(|(rx, ry)| {
                    if rx > 0.0 && ry > 0.0 {
                        ((rx - spread).max(0.0), (ry - spread).max(0.0))
                    } else {
                        (0.0, 0.0)
                    }
                }),
            };
            (
                Round {
                    rect: hole,
                    radii: hole_radii,
                    inverted: false,
                },
                Round {
                    rect: padding,
                    radii: inner,
                    inverted: false,
                },
                padding,
            )
        } else {
            let mut shape_rect = grow(border_box, spread);
            shape_rect.x += x;
            shape_rect.y += y;
            let shape_radii = Radii {
                corners: radii.corners.map(|(rx, ry)| {
                    if rx > 0.0 && ry > 0.0 {
                        ((rx + spread).max(0.0), (ry + spread).max(0.0))
                    } else {
                        (0.0, 0.0)
                    }
                }),
            };
            (
                Round {
                    rect: shape_rect,
                    radii: shape_radii,
                    inverted: false,
                },
                Round {
                    rect: border_box,
                    radii,
                    inverted: false,
                },
                grow(shape_rect, blur + 1.0),
            )
        };
        list.push(DisplayCommand::Shadow(ShadowPaint {
            shape,
            container,
            bounds,
            blur,
            color,
            alpha,
            inset,
        }));
    }
}

fn box_radii(layout_box: &LayoutBox, style: &StyledNode) -> Radii {
    let border_box = layout_box.dimensions.border_box();
    style.border_radii(border_box.width, border_box.height)
}

fn paint_background(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let layers = style.background_layers();
    if let Some(Rgba { color, alpha }) = style.rgba("background-color") {
        if alpha > 0 {
            with_clip(layout_box, style, layers - 1, list, |clip, list| {
                list.push(DisplayCommand::Rect(SolidRect {
                    color,
                    alpha,
                    rect: clip,
                }));
            });
        }
    }
    for index in (0..layers).rev() {
        let picture = layout_box
            .background_pictures
            .get(index)
            .and_then(Option::as_ref);
        if style.background_gradient(index).is_none() && picture.is_none() {
            continue;
        }
        with_clip(layout_box, style, index, list, |clip, list| {
            push_layer(layout_box, style, index, clip, list);
        });
    }
}

fn with_clip(
    layout_box: &LayoutBox,
    style: &StyledNode,
    index: usize,
    list: &mut Vec<DisplayCommand>,
    paint: impl FnOnce(Rect, &mut Vec<DisplayCommand>),
) {
    let d = &layout_box.dimensions;
    let border = [d.border.top, d.border.right, d.border.bottom, d.border.left];
    let (clip, inset) = match style.background_clip(index) {
        BackgroundBox::Border => (d.border_box(), [0.0; 4]),
        BackgroundBox::Padding => (d.padding_box(), border),
        BackgroundBox::Content => (
            d.content,
            [
                border[0] + d.padding.top,
                border[1] + d.padding.right,
                border[2] + d.padding.bottom,
                border[3] + d.padding.left,
            ],
        ),
    };
    let radii = box_radii(layout_box, style).shrink(inset);
    if radii.is_zero() {
        paint(clip, list);
        return;
    }
    list.push(DisplayCommand::BeginRound(Round {
        rect: clip,
        radii,
        inverted: false,
    }));
    paint(clip, list);
    list.push(DisplayCommand::EndGroup);
}

fn background_origin(layout_box: &LayoutBox, style: &StyledNode, index: usize) -> Rect {
    match style.background_origin(index) {
        BackgroundBox::Border => layout_box.dimensions.border_box(),
        BackgroundBox::Padding => layout_box.dimensions.padding_box(),
        BackgroundBox::Content => layout_box.dimensions.content,
    }
}

fn push_layer(
    layout_box: &LayoutBox,
    style: &StyledNode,
    index: usize,
    clip: Rect,
    list: &mut Vec<DisplayCommand>,
) {
    if let Some(gradient) = style.background_gradient(index) {
        let origin = background_origin(layout_box, style, index);
        let area = (origin.width, origin.height);
        let (width, height) = style.background_size(index, area, area);
        let (repeat_x, repeat_y) = style.background_repeat(index);
        list.push(DisplayCommand::GradientTiles(GradientTiles {
            clip,
            tile: Rect {
                x: origin.x
                    + style.background_offset("background-position-x", index, origin.width - width),
                y: origin.y
                    + style.background_offset(
                        "background-position-y",
                        index,
                        origin.height - height,
                    ),
                width,
                height,
            },
            repeat_x,
            repeat_y,
            gradient: Rc::new(gradient.clone()),
            font_size: style.font_size(),
        }));
    }
    let Some(Some(picture)) = layout_box.background_pictures.get(index) else {
        return;
    };
    let origin = background_origin(layout_box, style, index);
    let (width, height) = style.background_size(
        index,
        (origin.width, origin.height),
        (picture.width as f32, picture.height as f32),
    );
    let (repeat_x, repeat_y) = style.background_repeat(index);
    let x =
        origin.x + style.background_offset("background-position-x", index, origin.width - width);
    let y =
        origin.y + style.background_offset("background-position-y", index, origin.height - height);
    list.push(DisplayCommand::Tiles(Tiles {
        clip,
        tile: Rect {
            x,
            y,
            width,
            height,
        },
        repeat_x,
        repeat_y,
        picture: picture.clone(),
    }));
}

fn border_edges(border_box: &Rect, widths: [f32; 4]) -> [Rect; 4] {
    let [top, right, bottom, left] = widths;
    let side_top = border_box.y + top;
    let side_height = (border_box.height - top - bottom).max(0.0);
    [
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: top,
        },
        Rect {
            x: border_box.x + border_box.width - right,
            y: side_top,
            width: right,
            height: side_height,
        },
        Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - bottom,
            width: border_box.width,
            height: bottom,
        },
        Rect {
            x: border_box.x,
            y: side_top,
            width: left,
            height: side_height,
        },
    ]
}

fn outline_commands(layout_box: &LayoutBox) -> Vec<DisplayCommand> {
    let mut list = Vec::new();
    let BoxKind::Block(style) = layout_box.kind else {
        return list;
    };
    let Some(outline) = style.outline() else {
        return list;
    };
    let Rgba { color, alpha } = outline.color;
    let border_box = layout_box.dimensions.border_box();
    let inflate = |distance: f32| Rect {
        x: border_box.x - distance,
        y: border_box.y - distance,
        width: (border_box.width + distance * 2.0).max(0.0),
        height: (border_box.height + distance * 2.0).max(0.0),
    };
    let inner = inflate(outline.offset);
    let outer = inflate(outline.offset + outline.width);
    let radii = box_radii(layout_box, style);
    if radii.is_zero() {
        for (side, rect) in border_edges(&outer, [outline.width; 4])
            .into_iter()
            .enumerate()
        {
            push_edge(&mut list, rect, side % 2 == 0, outline.style, color, alpha);
        }
        return list;
    }
    list.push(DisplayCommand::BeginRound(Round {
        rect: outer,
        radii: radii.grow(outline.offset + outline.width),
        inverted: false,
    }));
    list.push(DisplayCommand::BeginRound(Round {
        rect: inner,
        radii: radii.grow(outline.offset),
        inverted: true,
    }));
    list.push(DisplayCommand::Rect(SolidRect {
        color,
        alpha,
        rect: outer,
    }));
    list.push(DisplayCommand::EndGroup);
    list.push(DisplayCommand::EndGroup);
    list
}

fn paint_borders(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let d = &layout_box.dimensions;
    let widths = [d.border.top, d.border.right, d.border.bottom, d.border.left];
    let radii = box_radii(layout_box, style);
    if !radii.is_zero() {
        let Some(Rgba { color, alpha }) = (0..4)
            .filter(|side| widths[*side] > 0.0)
            .find_map(|side| style.border_color(side))
        else {
            return;
        };
        list.push(DisplayCommand::BeginRound(Round {
            rect: d.border_box(),
            radii,
            inverted: false,
        }));
        list.push(DisplayCommand::BeginRound(Round {
            rect: d.padding_box(),
            radii: radii.shrink(widths),
            inverted: true,
        }));
        list.push(DisplayCommand::Rect(SolidRect {
            color,
            alpha,
            rect: d.border_box(),
        }));
        list.push(DisplayCommand::EndGroup);
        list.push(DisplayCommand::EndGroup);
        return;
    }
    for (side, rect) in border_edges(&d.border_box(), widths)
        .into_iter()
        .enumerate()
    {
        let Some(Rgba { color, alpha }) = style.border_color(side) else {
            continue;
        };
        push_edge(
            list,
            rect,
            side % 2 == 0,
            style.border_style(side),
            color,
            alpha,
        );
    }
}

fn push_edge(
    list: &mut Vec<DisplayCommand>,
    rect: Rect,
    horizontal: bool,
    style: StrokeStyle,
    color: Color,
    alpha: u8,
) {
    let thickness = if horizontal { rect.height } else { rect.width };
    let length = if horizontal { rect.width } else { rect.height };
    let mut piece = |offset: f32, size: f32, start: f32, depth: f32| {
        let (x, y, width, height) = if horizontal {
            (rect.x + offset, rect.y + start, size, depth)
        } else {
            (rect.x + start, rect.y + offset, depth, size)
        };
        list.push(DisplayCommand::Rect(SolidRect {
            color,
            alpha,
            rect: Rect {
                x,
                y,
                width,
                height,
            },
        }));
    };
    match style {
        StrokeStyle::Dashed | StrokeStyle::Dotted if thickness > 0.0 => {
            let dash = if style == StrokeStyle::Dashed {
                thickness * 3.0
            } else {
                thickness
            };
            let step = dash * 2.0;
            let mut offset = 0.0;
            while offset < length {
                piece(offset, dash.min(length - offset), 0.0, thickness);
                offset += step;
            }
        }
        StrokeStyle::Double if thickness >= 3.0 => {
            let band = (thickness / 3.0).round();
            piece(0.0, length, 0.0, band);
            piece(0.0, length, thickness - band, band);
        }
        _ => piece(0.0, length, 0.0, thickness),
    }
}
