// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::css::Color;
use crate::inline::{Fragment, ImageFragment};
use crate::layout::{BoxKind, LayoutBox, Rect};
use crate::picture::Picture;
use crate::style::{Content, Position};

const WHITE: Color = Color {
    r: 255,
    g: 255,
    b: 255,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SolidRect {
    pub color: Color,
    pub rect: Rect,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    Rect(SolidRect),
    Text(Fragment),
    Image(ImageFragment),
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
                self.pixels[y * self.width + x] = item.color;
            }
        }
    }

    fn draw_text(&mut self, fragment: &Fragment, font: &Font) {
        let mut pen_x = fragment.x;
        let baseline = fragment.baseline.round() as i64;
        for c in fragment.text.chars() {
            let (metrics, bitmap) = font.rasterize(c, fragment.font_size);
            let left = (pen_x + metrics.xmin as f32).round() as i64;
            let top = baseline - metrics.height as i64 - i64::from(metrics.ymin);
            for row in 0..metrics.height {
                for col in 0..metrics.width {
                    let coverage = bitmap[row * metrics.width + col];
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
            pen_x += metrics.advance_width;
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

pub fn paint(root: &LayoutBox, width: usize, height: usize, scroll_y: f32, font: &Font) -> Canvas {
    let mut list = DisplayList::new();
    paint_stacking(root, false, &mut list);
    let mut canvas = Canvas::filled(width, height, canvas_background(root));
    for (command, fixed) in &list {
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
                canvas.draw_text(&shifted, font);
            }
            DisplayCommand::Image(image) => {
                let mut rect = image.rect;
                rect.y -= offset;
                canvas.draw_picture(&rect, &image.picture);
            }
        }
    }
    canvas
}

fn canvas_background(root: &LayoutBox) -> Color {
    if let BoxKind::Block(style) = root.kind {
        if let Some(color) = style.background() {
            return color;
        }
    }
    for child in &root.children {
        if let BoxKind::Block(style) = child.kind {
            if let Content::Element(element) = style.content {
                if element.tag_name == "body" {
                    return style.background().unwrap_or(WHITE);
                }
            }
        }
    }
    WHITE
}

struct Positioned<'a, 'b> {
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
                rect: inline_box.rect,
            }));
        }
        if let Some(color) = inline_box.border_color {
            for rect in border_edges(&inline_box.rect, inline_box.border) {
                commands.push(DisplayCommand::Rect(SolidRect { color, rect }));
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
    out: &mut Vec<Positioned<'a, 'b>>,
) {
    for child in children {
        let BoxKind::Block(style) = child.kind else {
            continue;
        };
        if !style.is_positioned() {
            collect_positioned(&child.children, fixed, out);
            continue;
        }
        let child_fixed = fixed || is_fixed(child);
        let z_index = style.z_index();
        out.push(Positioned {
            z_index: z_index.unwrap_or(0),
            creates_context: z_index.is_some(),
            fixed: child_fixed,
            layout_box: child,
        });
        if z_index.is_none() {
            collect_positioned(&child.children, child_fixed, out);
        }
    }
}

fn paint_flow(children: &[LayoutBox], fixed: bool, list: &mut DisplayList) {
    for child in children {
        if is_positioned(child) {
            continue;
        }
        emit(list, fixed, decoration_commands(child));
        emit(list, fixed, content_commands(child));
        paint_flow(&child.children, fixed, list);
    }
}

fn paint_positioned(item: &Positioned, list: &mut DisplayList) {
    if item.creates_context {
        paint_stacking(item.layout_box, item.fixed, list);
    } else {
        emit(list, item.fixed, decoration_commands(item.layout_box));
        emit(list, item.fixed, content_commands(item.layout_box));
        paint_flow(&item.layout_box.children, item.fixed, list);
    }
}

fn paint_stacking(layout_box: &LayoutBox, fixed: bool, list: &mut DisplayList) {
    emit(list, fixed, decoration_commands(layout_box));
    let mut positioned = Vec::new();
    collect_positioned(&layout_box.children, fixed, &mut positioned);
    positioned.sort_by_key(|item| item.z_index);
    for item in positioned.iter().filter(|item| item.z_index < 0) {
        paint_positioned(item, list);
    }
    emit(list, fixed, content_commands(layout_box));
    paint_flow(&layout_box.children, fixed, list);
    for item in positioned.iter().filter(|item| item.z_index >= 0) {
        paint_positioned(item, list);
    }
}

fn paint_background(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    if let Some(color) = style.background() {
        list.push(DisplayCommand::Rect(SolidRect {
            color,
            rect: layout_box.dimensions.border_box(),
        }));
    }
}

fn border_edges(border_box: &Rect, widths: [f32; 4]) -> [Rect; 4] {
    let [top, right, bottom, left] = widths;
    [
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: top,
        },
        Rect {
            x: border_box.x + border_box.width - right,
            y: border_box.y,
            width: right,
            height: border_box.height,
        },
        Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - bottom,
            width: border_box.width,
            height: bottom,
        },
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: left,
            height: border_box.height,
        },
    ]
}

fn paint_borders(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let Some(color) = style.color("border-color") else {
        return;
    };
    let d = &layout_box.dimensions;
    let widths = [d.border.top, d.border.right, d.border.bottom, d.border.left];
    for rect in border_edges(&d.border_box(), widths) {
        list.push(DisplayCommand::Rect(SolidRect { color, rect }));
    }
}
