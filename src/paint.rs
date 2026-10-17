// SPDX-License-Identifier: GPL-3.0-only

use crate::css::Color;
use crate::layout::{LayoutBox, Rect};

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

pub fn paint(root: &LayoutBox, width: usize, height: usize) -> Canvas {
    let mut list = Vec::new();
    build_display_list(root, &mut list);
    let mut canvas = Canvas::new(width, height);
    for item in &list {
        canvas.fill(item);
    }
    canvas
}

fn build_display_list(layout_box: &LayoutBox, list: &mut Vec<SolidRect>) {
    paint_background(layout_box, list);
    paint_borders(layout_box, list);
    for child in &layout_box.children {
        build_display_list(child, list);
    }
}

fn paint_background(layout_box: &LayoutBox, list: &mut Vec<SolidRect>) {
    let style = layout_box.style;
    let color = style
        .color("background-color")
        .or(style.color("background"));
    if let Some(color) = color {
        list.push(SolidRect {
            color,
            rect: layout_box.dimensions.border_box(),
        });
    }
}

fn paint_borders(layout_box: &LayoutBox, list: &mut Vec<SolidRect>) {
    let Some(color) = layout_box.style.color("border-color") else {
        return;
    };
    let d = &layout_box.dimensions;
    let border_box = d.border_box();

    list.push(SolidRect {
        color,
        rect: Rect {
            x: border_box.x,
            y: border_box.y,
            width: d.border.left,
            height: border_box.height,
        },
    });
    list.push(SolidRect {
        color,
        rect: Rect {
            x: border_box.x + border_box.width - d.border.right,
            y: border_box.y,
            width: d.border.right,
            height: border_box.height,
        },
    });
    list.push(SolidRect {
        color,
        rect: Rect {
            x: border_box.x,
            y: border_box.y,
            width: border_box.width,
            height: d.border.top,
        },
    });
    list.push(SolidRect {
        color,
        rect: Rect {
            x: border_box.x,
            y: border_box.y + border_box.height - d.border.bottom,
            width: border_box.width,
            height: d.border.bottom,
        },
    });
}
