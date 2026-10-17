// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::css::Color;
use crate::inline::{Fragment, ImageFragment};
use crate::layout::{BoxKind, LayoutBox, Rect};
use crate::picture::Picture;
use crate::style::Content;

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

pub fn paint(root: &LayoutBox, width: usize, height: usize, scroll_y: f32, font: &Font) -> Canvas {
    let mut list = Vec::new();
    build_display_list(root, &mut list);
    let mut canvas = Canvas::filled(width, height, canvas_background(root));
    for command in &list {
        match command {
            DisplayCommand::Rect(item) => {
                let mut shifted = *item;
                shifted.rect.y -= scroll_y;
                canvas.fill(&shifted);
            }
            DisplayCommand::Text(fragment) => {
                let mut shifted = fragment.clone();
                shifted.baseline -= scroll_y;
                canvas.draw_text(&shifted, font);
            }
            DisplayCommand::Image(image) => {
                let mut rect = image.rect;
                rect.y -= scroll_y;
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

fn build_display_list(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    paint_background(layout_box, list);
    paint_borders(layout_box, list);
    for fragment in &layout_box.fragments {
        if let Some((color, rect)) = fragment.background {
            list.push(DisplayCommand::Rect(SolidRect { color, rect }));
        }
        list.push(DisplayCommand::Text(fragment.clone()));
    }
    for image in &layout_box.images {
        list.push(DisplayCommand::Image(image.clone()));
    }
    for child in &layout_box.children {
        build_display_list(child, list);
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

fn paint_borders(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let Some(color) = style.color("border-color") else {
        return;
    };
    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    let edges = [
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: d.border.left,
            height: border_box.height,
        },
        Rect {
            x: border_box.x + border_box.width - d.border.right,
            y: border_box.y,
            width: d.border.right,
            height: border_box.height,
        },
        Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: d.border.top,
        },
        Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - d.border.bottom,
            width: border_box.width,
            height: d.border.bottom,
        },
    ];
    for rect in edges {
        list.push(DisplayCommand::Rect(SolidRect { color, rect }));
    }
}
