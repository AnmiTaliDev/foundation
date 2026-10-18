// SPDX-License-Identifier: GPL-3.0-only

pub type Matrix = [f32; 6];

pub const IDENTITY: Matrix = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Amount {
    Px(f32),
    Em(f32),
    Percent(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Function {
    Translate(Amount, Amount),
    Scale(f32, f32),
    Rotate(f32),
    Skew(f32, f32),
    Matrix(Matrix),
}

pub fn multiply(left: &Matrix, right: &Matrix) -> Matrix {
    let [a1, b1, c1, d1, e1, f1] = *left;
    let [a2, b2, c2, d2, e2, f2] = *right;
    [
        a1 * a2 + c1 * b2,
        b1 * a2 + d1 * b2,
        a1 * c2 + c1 * d2,
        b1 * c2 + d1 * d2,
        a1 * e2 + c1 * f2 + e1,
        b1 * e2 + d1 * f2 + f1,
    ]
}

pub fn invert(matrix: &Matrix) -> Option<Matrix> {
    let [a, b, c, d, e, f] = *matrix;
    let determinant = a * d - b * c;
    if determinant.abs() < 1e-6 {
        return None;
    }
    Some([
        d / determinant,
        -b / determinant,
        -c / determinant,
        a / determinant,
        (c * f - d * e) / determinant,
        (b * e - a * f) / determinant,
    ])
}

pub fn apply(matrix: &Matrix, x: f32, y: f32) -> (f32, f32) {
    (
        matrix[0] * x + matrix[2] * y + matrix[4],
        matrix[1] * x + matrix[3] * y + matrix[5],
    )
}

pub fn compose(
    functions: &[Function],
    origin: (f32, f32),
    width: f32,
    height: f32,
    font_size: f32,
) -> Matrix {
    let amount = |amount: Amount, base: f32| match amount {
        Amount::Px(px) => px,
        Amount::Em(em) => em * font_size,
        Amount::Percent(percent) => percent / 100.0 * base,
    };
    let mut total = IDENTITY;
    for function in functions {
        let next = match function {
            Function::Translate(x, y) => {
                [1.0, 0.0, 0.0, 1.0, amount(*x, width), amount(*y, height)]
            }
            Function::Scale(x, y) => [*x, 0.0, 0.0, *y, 0.0, 0.0],
            Function::Rotate(angle) => [
                angle.cos(),
                angle.sin(),
                -angle.sin(),
                angle.cos(),
                0.0,
                0.0,
            ],
            Function::Skew(x, y) => [1.0, y.tan(), x.tan(), 1.0, 0.0, 0.0],
            Function::Matrix(matrix) => *matrix,
        };
        total = multiply(&total, &next);
    }
    let to_origin = [1.0, 0.0, 0.0, 1.0, origin.0, origin.1];
    let from_origin = [1.0, 0.0, 0.0, 1.0, -origin.0, -origin.1];
    multiply(&multiply(&to_origin, &total), &from_origin)
}

pub fn parse(text: &str) -> Option<Vec<Function>> {
    let mut functions = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let open = rest.find('(')?;
        let close = rest.find(')')?;
        if close < open {
            return None;
        }
        let name = rest[..open].trim().to_ascii_lowercase();
        let arguments: Vec<&str> = rest[open + 1..close].split(',').map(str::trim).collect();
        functions.push(parse_function(&name, &arguments)?);
        rest = rest[close + 1..].trim_start();
    }
    if functions.is_empty() {
        None
    } else {
        Some(functions)
    }
}

fn parse_amount(text: &str) -> Option<Amount> {
    if let Some(number) = text.strip_suffix("px") {
        return number.trim().parse().ok().map(Amount::Px);
    }
    if let Some(number) = text.strip_suffix("em") {
        return number.trim().parse().ok().map(Amount::Em);
    }
    if let Some(number) = text.strip_suffix('%') {
        return number.trim().parse().ok().map(Amount::Percent);
    }
    match text.parse::<f32>() {
        Ok(0.0) => Some(Amount::Px(0.0)),
        _ => None,
    }
}

fn parse_number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|number| number.is_finite())
}

fn parse_angle(text: &str) -> Option<f32> {
    let (number, factor) = if let Some(number) = text.strip_suffix("deg") {
        (number, std::f32::consts::PI / 180.0)
    } else if let Some(number) = text.strip_suffix("grad") {
        (number, std::f32::consts::PI / 200.0)
    } else if let Some(number) = text.strip_suffix("rad") {
        (number, 1.0)
    } else if let Some(number) = text.strip_suffix("turn") {
        (number, 2.0 * std::f32::consts::PI)
    } else {
        return match text.parse::<f32>() {
            Ok(0.0) => Some(0.0),
            _ => None,
        };
    };
    parse_number(number.trim()).map(|value| value * factor)
}

fn parse_function(name: &str, arguments: &[&str]) -> Option<Function> {
    let zero = Amount::Px(0.0);
    match (name, arguments) {
        ("translate", [x]) => Some(Function::Translate(parse_amount(x)?, zero)),
        ("translate", [x, y]) => Some(Function::Translate(parse_amount(x)?, parse_amount(y)?)),
        ("translatex", [x]) => Some(Function::Translate(parse_amount(x)?, zero)),
        ("translatey", [y]) => Some(Function::Translate(zero, parse_amount(y)?)),
        ("scale", [s]) => {
            let s = parse_number(s)?;
            Some(Function::Scale(s, s))
        }
        ("scale", [x, y]) => Some(Function::Scale(parse_number(x)?, parse_number(y)?)),
        ("scalex", [x]) => Some(Function::Scale(parse_number(x)?, 1.0)),
        ("scaley", [y]) => Some(Function::Scale(1.0, parse_number(y)?)),
        ("rotate", [angle]) => Some(Function::Rotate(parse_angle(angle)?)),
        ("skew", [x]) => Some(Function::Skew(parse_angle(x)?, 0.0)),
        ("skew", [x, y]) => Some(Function::Skew(parse_angle(x)?, parse_angle(y)?)),
        ("skewx", [x]) => Some(Function::Skew(parse_angle(x)?, 0.0)),
        ("skewy", [y]) => Some(Function::Skew(0.0, parse_angle(y)?)),
        ("matrix", values) if values.len() == 6 => {
            let mut matrix = IDENTITY;
            for (slot, value) in matrix.iter_mut().zip(values) {
                *slot = parse_number(value)?;
            }
            Some(Function::Matrix(matrix))
        }
        _ => None,
    }
}
