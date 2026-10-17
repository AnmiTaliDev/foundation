// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::css::Color;
use crate::inline::Fragment;
use crate::layout::{BoxKind, LayoutBox, Rect};

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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Canvas {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<Color>,
}

impl Canvas {
    pub fn new(width: usize, height: usize) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![WHITE; width * height],
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

pub fn paint(root: &LayoutBox, width: usize, height: usize, font: &Font) -> Canvas {
    let mut list = Vec::new();
    build_display_list(root, &mut list);
    let mut canvas = Canvas::new(width, height);
    for command in &list {
        match command {
            DisplayCommand::Rect(item) => canvas.fill(item),
            DisplayCommand::Text(fragment) => canvas.draw_text(fragment, font),
        }
    }
    canvas
}

fn build_display_list(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    paint_background(layout_box, list);
    paint_borders(layout_box, list);
    for fragment in &layout_box.fragments {
        list.push(DisplayCommand::Text(fragment.clone()));
    }
    for child in &layout_box.children {
        build_display_list(child, list);
    }
}

fn paint_background(layout_box: &LayoutBox, list: &mut Vec<DisplayCommand>) {
    let BoxKind::Block(style) = layout_box.kind else {
        return;
    };
    let color = style
        .color("background-color")
        .or(style.color("background"));
    if let Some(color) = color {
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
