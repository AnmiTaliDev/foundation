// SPDX-License-Identifier: GPL-3.0-only

use std::process::ExitCode;
use std::{env, fs};

const DEFAULT_WIDTH: usize = 800;
const DEFAULT_HEIGHT: usize = 600;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 && args.len() != 5 {
        eprintln!("usage: foundation <input.html> <output.ppm> [width height]");
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
    let source = fs::read_to_string(&args[1])
        .map_err(|error| format!("cannot read {}: {}", args[1], error))?;
    let (width, height) = if args.len() == 5 {
        (parse_dimension(&args[3])?, parse_dimension(&args[4])?)
    } else {
        (DEFAULT_WIDTH, DEFAULT_HEIGHT)
    };
    let canvas = foundation::render(&source, width, height);
    fs::write(&args[2], canvas.to_ppm())
        .map_err(|error| format!("cannot write {}: {}", args[2], error))
}

fn parse_dimension(text: &str) -> Result<usize, String> {
    match text.parse::<usize>() {
        Ok(value) if value > 0 => Ok(value),
        _ => Err(format!("invalid dimension: {}", text)),
    }
}
