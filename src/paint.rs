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
use crate::style::{Content, Position};

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
    BeginGroup(Group),
    EndGroup,
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
        }
    }

    pub fn pixel(&self, x: usize, y: usize) -> Color {
        self.pixels[y * self.width + x]
    }

    fn fill(&mut self, item: &SolidRect) {
        let x0 = item.rect.x.clamp(0.0, self.width as f32).round() as usize;
        let y0 = item.rect.y.clamp(0.0, self.height as f32).round() as usize;
        let x1 = (item.rect.x + item.rect.width)
            .clamp(0.0, self.width as f32)
            .round() as usize;
        let y1 = (item.rect.y + item.rect.height)
            .clamp(0.0, self.height as f32)
            .round() as usize;
        for y in y0..y1 {
            for x in x0..x1 {
                let index = y * self.width + x;
                self.pixels[index] = if item.alpha == 255 {
                    item.color
                } else {
                    blend(self.pixels[index], item.color, item.alpha)
                };
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
                        || x < 0
                        || y < 0
                        || x >= self.width as i64
                        || y >= self.height as i64
                    {
                        continue;
                    }
                    let index = y as usize * self.width + x as usize;
                    self.pixels[index] = blend(self.pixels[index], fragment.color, coverage);
                }
            }
            pen_x += fonts.advance(fragment.face, c, fragment.font_size);
        }
    }

    fn draw_picture(&mut self, rect: &Rect, picture: &Picture) {
        if rect.width <= 0.0 || rect.height <= 0.0 || picture.width == 0 || picture.height == 0 {
            return;
        }
        let x_start = rect.x.round().clamp(0.0, self.width as f32) as usize;
        let y_start = rect.y.round().clamp(0.0, self.height as f32) as usize;
        let x_end = (rect.x + rect.width).round().clamp(0.0, self.width as f32) as usize;
        let y_end = (rect.y + rect.height)
            .round()
            .clamp(0.0, self.height as f32) as usize;
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
                let index = y * self.width + x;
                self.pixels[index] = blend(self.pixels[index], color, alpha);
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
        let x_start = tiles.clip.x.round().clamp(0.0, self.width as f32) as usize;
        let y_start = tiles.clip.y.round().clamp(0.0, self.height as f32) as usize;
        let x_end = (tiles.clip.x + tiles.clip.width)
            .round()
            .clamp(0.0, self.width as f32) as usize;
        let y_end = (tiles.clip.y + tiles.clip.height)
            .round()
            .clamp(0.0, self.height as f32) as usize;
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
                let index = y * self.width + x;
                self.pixels[index] = blend(self.pixels[index], color, alpha);
            }
        }
    }

    fn draw_tiles(&mut self, tiles: &Tiles) {
        let tile = tiles.tile;
        let picture = &tiles.picture;
        if tile.width <= 0.0 || tile.height <= 0.0 || picture.width == 0 || picture.height == 0 {
            return;
        }
        let x_start = tiles.clip.x.round().clamp(0.0, self.width as f32) as usize;
        let y_start = tiles.clip.y.round().clamp(0.0, self.height as f32) as usize;
        let x_end = (tiles.clip.x + tiles.clip.width)
            .round()
            .clamp(0.0, self.width as f32) as usize;
        let y_end = (tiles.clip.y + tiles.clip.height)
            .round()
            .clamp(0.0, self.height as f32) as usize;
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
                let index = y * self.width + x;
                self.pixels[index] = blend(self.pixels[index], color, alpha);
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
            DisplayCommand::BeginGroup(_) | DisplayCommand::BeginShift(_) => depth += 1,
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
    paint_background(layout_box, &mut commands);
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
    for fragment in &layout_box.fragments {
        commands.push(DisplayCommand::Text(fragment.clone()));
    }
    for image in &layout_box.images {
        commands.push(DisplayCommand::Image(image.clone()));
    }
    commands
}

fn collect_positioned<'a, 'b>(
    children: &'b [LayoutBox<'a>],
    fixed: bool,
    container: Rect,
    out: &mut Vec<Positioned<'a, 'b>>,
) {
    for child in children {
        let BoxKind::Block(style) = child.kind else {
            collect_positioned(&child.children, fixed, container, out);
            continue;
        };
        if !style.is_positioned() {
            if !style.has_effect() {
                collect_positioned(&child.children, fixed, child.dimensions.content, out);
            }
            continue;
        }
        let child_fixed = fixed || is_fixed(child);
        let z_index = style.z_index();
        let creates_context =
            z_index.is_some() || style.has_effect() || style.position() == Position::Sticky;
        out.push(Positioned {
            container,
            z_index: z_index.unwrap_or(0),
            creates_context,
            fixed: child_fixed,
            layout_box: child,
        });
        if !creates_context {
            collect_positioned(&child.children, child_fixed, child.dimensions.content, out);
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
        emit(list, fixed, content_commands(child));
        paint_flow(&child.children, fixed, child.dimensions.content, view, list);
    }
}

fn paint_positioned(item: &Positioned, view: View, list: &mut DisplayList) {
    if item.creates_context {
        paint_stacking(item.layout_box, item.fixed, item.container, view, list);
    } else {
        emit(list, item.fixed, decoration_commands(item.layout_box));
        emit(list, item.fixed, content_commands(item.layout_box));
        paint_flow(
            &item.layout_box.children,
            item.fixed,
            item.layout_box.dimensions.content,
            view,
            list,
        );
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
    let content = layout_box.dimensions.content;
    let mut positioned = Vec::new();
    collect_positioned(&layout_box.children, fixed, content, &mut positioned);
    positioned.sort_by_key(|item| item.z_index);
    for item in positioned.iter().filter(|item| item.z_index < 0) {
        paint_positioned(item, view, list);
    }
    emit(list, fixed, content_commands(layout_box));
    paint_flow(&layout_box.children, fixed, content, view, list);
    for item in positioned.iter().filter(|item| item.z_index >= 0) {
        paint_positioned(item, view, list);
    }
}

fn paint_background(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    if let Some(Rgba { color, alpha }) = style.rgba("background-color") {
        if alpha > 0 {
            list.push(DisplayCommand::Rect(SolidRect {
                color,
                alpha,
                rect: layout_box.dimensions.border_box(),
            }));
        }
    }
    if let Some(gradient) = style.background_gradient() {
        let origin = layout_box.dimensions.padding_box();
        let area = (origin.width, origin.height);
        let (width, height) = style.background_size(area, area);
        let (repeat_x, repeat_y) = style.background_repeat();
        list.push(DisplayCommand::GradientTiles(GradientTiles {
            clip: layout_box.dimensions.border_box(),
            tile: Rect {
                x: origin.x
                    + style.background_offset("background-position-x", origin.width - width),
                y: origin.y
                    + style.background_offset("background-position-y", origin.height - height),
                width,
                height,
            },
            repeat_x,
            repeat_y,
            gradient: Rc::new(gradient.clone()),
            font_size: style.font_size(),
        }));
    }
    let Some(picture) = &layout_box.background_picture else {
        return;
    };
    let origin = layout_box.dimensions.padding_box();
    let (width, height) = style.background_size(
        (origin.width, origin.height),
        (picture.width as f32, picture.height as f32),
    );
    let (repeat_x, repeat_y) = style.background_repeat();
    let x = origin.x + style.background_offset("background-position-x", origin.width - width);
    let y = origin.y + style.background_offset("background-position-y", origin.height - height);
    list.push(DisplayCommand::Tiles(Tiles {
        clip: layout_box.dimensions.border_box(),
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

fn paint_borders(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let Some(Rgba { color, alpha }) = style.rgba("border-color") else {
        return;
    };
    let d = &layout_box.dimensions;
    let widths = [d.border.top, d.border.right, d.border.bottom, d.border.left];
    for rect in border_edges(&d.border_box(), widths) {
        list.push(DisplayCommand::Rect(SolidRect { color, alpha, rect }));
    }
}
