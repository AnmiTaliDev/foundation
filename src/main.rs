// SPDX-License-Identifier: GPL-3.0-only

use std::process::ExitCode;
use std::{env, fs};

use foundation::Page;

const DEFAULT_WIDTH: usize = 800;
const DEFAULT_HEIGHT: usize = 600;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 && args.len() != 5 {
        eprintln!("usage: foundation <input.html|url> <output.ppm> [width height]");
        return ExitCode::from(2);
    }
    match run(&args) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("foundation: {}", message);
            ExitCode::FAILURE
        }
    }
}

fn run(args: &[String]) -> Result<(), String> {
    let page = Page::open(&args[1])?;
    for warning in &page.warnings {
        eprintln!("foundation: warning: {}", warning);
    }
    let (width, height) = if args.len() == 5 {
        (parse_dimension(&args[3])?, parse_dimension(&args[4])?)
    } else {
        (DEFAULT_WIDTH, DEFAULT_HEIGHT)
    };
    let canvas = page.render(width, height, 0.0).canvas;
    fs::write(&args[2], canvas.to_ppm())
        .map_err(|error| format!("cannot write {}: {}", args[2], error))
}

fn parse_dimension(text: &str) -> Result<usize, String> {
    match text.parse::<usize>() {
        Ok(value) if value > 0 => Ok(value),
        _ => Err(format!("invalid dimension: {}", text)),
    }
}
