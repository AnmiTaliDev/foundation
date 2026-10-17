// SPDX-License-Identifier: GPL-3.0-only

use std::process::ExitCode;
use std::{env, fs};

use minifb::{Key, Window, WindowOptions};

const DEFAULT_WIDTH: usize = 800;
const DEFAULT_HEIGHT: usize = 600;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: foundation-window <input.html>");
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

fn run(path: &str) -> Result<(), String> {
    let source =
        fs::read_to_string(path).map_err(|error| format!("cannot read {}: {}", path, error))?;
    let options = WindowOptions {
        resize: true,
        ..WindowOptions::default()
    };
    let mut window = Window::new("Foundation", DEFAULT_WIDTH, DEFAULT_HEIGHT, options)
        .map_err(|error| format!("cannot create window: {}", error))?;
    window.set_target_fps(60);

    let mut size = (0, 0);
    let mut buffer = Vec::new();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        let (width, height) = window.get_size();
        if (width, height) != size && width > 0 && height > 0 {
            size = (width, height);
            buffer = foundation::render(&source, width, height).to_argb();
        }
        window
            .update_with_buffer(&buffer, size.0, size.1)
            .map_err(|error| format!("cannot update window: {}", error))?;
    }
    Ok(())
}
