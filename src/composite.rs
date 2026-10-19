// SPDX-License-Identifier: GPL-3.0-only

use crate::css::Color;
use crate::paint::Canvas;
use crate::transform::{self, Matrix};

pub(crate) fn fade(canvas: &mut Canvas, backdrop: &[Color], opacity: f32) {
    for (pixel, before) in canvas.pixels.iter_mut().zip(backdrop) {
        pixel.r = mix(before.r as f32, pixel.r as f32, opacity);
        pixel.g = mix(before.g as f32, pixel.g as f32, opacity);
        pixel.b = mix(before.b as f32, pixel.b as f32, opacity);
    }
}

fn mix(from: f32, to: f32, amount: f32) -> u8 {
    (from + (to - from) * amount).round().clamp(0.0, 255.0) as u8
}

fn sample(canvas: &Canvas, x: i64, y: i64, outside: f32) -> [f32; 3] {
    if x < 0 || y < 0 || x >= canvas.width as i64 || y >= canvas.height as i64 {
        return [outside; 3];
    }
    let pixel = canvas.pixel(x as usize, y as usize);
    [pixel.r as f32, pixel.g as f32, pixel.b as f32]
}

fn bilinear(canvas: &Canvas, x: f32, y: f32, outside: f32) -> [f32; 3] {
    let x0 = (x - 0.5).floor();
    let y0 = (y - 0.5).floor();
    let fx = x - 0.5 - x0;
    let fy = y - 0.5 - y0;
    let corners = [
        (
            sample(canvas, x0 as i64, y0 as i64, outside),
            (1.0 - fx) * (1.0 - fy),
        ),
        (
            sample(canvas, x0 as i64 + 1, y0 as i64, outside),
            fx * (1.0 - fy),
        ),
        (
            sample(canvas, x0 as i64, y0 as i64 + 1, outside),
            (1.0 - fx) * fy,
        ),
        (
            sample(canvas, x0 as i64 + 1, y0 as i64 + 1, outside),
            fx * fy,
        ),
    ];
    let mut result = [0.0; 3];
    for (color, weight) in corners {
        for channel in 0..3 {
            result[channel] += color[channel] * weight;
        }
    }
    result
}

pub(crate) fn draw_layer(
    destination: &mut Canvas,
    on_black: &Canvas,
    on_white: &Canvas,
    matrix: &Matrix,
    opacity: f32,
) {
    let Some(inverse) = transform::invert(matrix) else {
        return;
    };
    let width = on_black.width as f32;
    let height = on_black.height as f32;
    let corners = [(0.0, 0.0), (width, 0.0), (0.0, height), (width, height)]
        .map(|(x, y)| transform::apply(matrix, x, y));
    let min_x = corners.iter().map(|c| c.0).fold(f32::MAX, f32::min);
    let max_x = corners.iter().map(|c| c.0).fold(f32::MIN, f32::max);
    let min_y = corners.iter().map(|c| c.1).fold(f32::MAX, f32::min);
    let max_y = corners.iter().map(|c| c.1).fold(f32::MIN, f32::max);
    let clip = destination.clip;
    let x_start = min_x.floor().clamp(clip.0 as f32, clip.2 as f32) as usize;
    let x_end = max_x.ceil().clamp(clip.0 as f32, clip.2 as f32) as usize;
    let y_start = min_y.floor().clamp(clip.1 as f32, clip.3 as f32) as usize;
    let y_end = max_y.ceil().clamp(clip.1 as f32, clip.3 as f32) as usize;
    for y in y_start..y_end {
        for x in x_start..x_end {
            let (source_x, source_y) = transform::apply(&inverse, x as f32 + 0.5, y as f32 + 0.5);
            if source_x < 0.0 || source_y < 0.0 || source_x >= width || source_y >= height {
                continue;
            }
            let black = bilinear(on_black, source_x, source_y, 0.0);
            let white = bilinear(on_white, source_x, source_y, 255.0);
            let covered: f32 = (0..3).map(|channel| white[channel] - black[channel]).sum();
            let alpha = 1.0 - covered / (3.0 * 255.0);
            if alpha < 0.004 {
                continue;
            }
            let weight = alpha.min(1.0) * opacity * destination.coverage(x, y);
            let index = y * destination.width + x;
            let before = destination.pixels[index];
            let channel =
                |from: u8, value: f32| mix(from as f32, (value / alpha).min(255.0), weight);
            destination.pixels[index] = Color {
                r: channel(before.r, black[0]),
                g: channel(before.g, black[1]),
                b: channel(before.b, black[2]),
            };
        }
    }
}
