// SPDX-License-Identifier: GPL-3.0-only

use std::env;
use std::process::ExitCode;

use foundation::Page;
use minifb::{Key, KeyRepeat, MouseMode, Window, WindowOptions};

const DEFAULT_WIDTH: usize = 800;
const DEFAULT_HEIGHT: usize = 600;
const LINE_SCROLL: f32 = 40.0;
const WHEEL_SCROLL: f32 = 40.0;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: foundation-window <input.html|url>");
        return ExitCode::from(2);
    }
    match run(&args[1]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("foundation-window: {}", message);
            ExitCode::FAILURE
        }
    }
}

fn scroll_delta(window: &Window, height: usize) -> f32 {
    let page = height as f32 - LINE_SCROLL;
    let mut delta = 0.0;
    if window.is_key_pressed(Key::Down, KeyRepeat::Yes) {
        delta += LINE_SCROLL;
    }
    if window.is_key_pressed(Key::Up, KeyRepeat::Yes) {
        delta -= LINE_SCROLL;
    }
    if window.is_key_pressed(Key::PageDown, KeyRepeat::Yes) {
        delta += page;
    }
    if window.is_key_pressed(Key::PageUp, KeyRepeat::Yes) {
        delta -= page;
    }
    delta
}

fn run(path: &str) -> Result<(), String> {
    let mut page = Page::open(path)?;
    for warning in &page.warnings {
        eprintln!("foundation-window: warning: {}", warning);
    }
    let options = WindowOptions {
        resize: true,
        ..WindowOptions::default()
    };
    let mut window = Window::new("Foundation", DEFAULT_WIDTH, DEFAULT_HEIGHT, options)
        .map_err(|error| format!("cannot create window: {}", error))?;
    window.set_target_fps(60);

    let mut size = (0, 0);
    let mut scroll_y = 0.0;
    let mut content_height = 0.0;
    let mut buffer = Vec::new();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (width, height) = window.get_size();
        if width == 0 || height == 0 {
            window.update();
            continue;
        }
        let mut wheel = 0.0;
        let mut scrolled = false;
        if let Some((wheel_x, wheel_y)) = window.get_scroll_wheel() {
            let delta = (-wheel_x * WHEEL_SCROLL, -wheel_y * WHEEL_SCROLL);
            wheel = delta.1;
            if let Some(point) = window.get_mouse_pos(MouseMode::Discard) {
                let remaining = page.scroll_by((width, height), scroll_y, point, delta);
                scrolled = remaining != delta;
                wheel = remaining.1;
            }
        }
        let mut target = scroll_y + wheel + scroll_delta(&window, height);
        if window.is_key_pressed(Key::Home, KeyRepeat::No) {
            target = 0.0;
        }
        if window.is_key_pressed(Key::End, KeyRepeat::No) {
            target = content_height;
        }
        if (width, height) != size || target != scroll_y || scrolled {
            let frame = page.render(width, height, target);
            size = (width, height);
            scroll_y = frame.scroll_y;
            content_height = frame.content_height;
            buffer = frame.canvas.to_argb();
        }
        window
            .update_with_buffer(&buffer, size.0, size.1)
            .map_err(|error| format!("cannot update window: {}", error))?;
    }
    Ok(())
}
