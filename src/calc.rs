// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Linear {
    pub px: f32,
    pub em: f32,
    pub rem: f32,
    pub percent: f32,
    pub vw: f32,
    pub vh: f32,
}

#[derive(Clone, Copy)]
enum Operand {
    Scalar(f32),
    Length(Linear),
}

impl Linear {
    pub fn is_px_only(&self) -> bool {
        self.em == 0.0 && self.rem == 0.0 && self.percent == 0.0 && self.vw == 0.0 && self.vh == 0.0
    }

    pub fn fold_viewport(
        &self,
        root_font_size: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Linear {
        Linear {
            px: self.px
                + self.rem * root_font_size
                + self.vw * viewport_width / 100.0
                + self.vh * viewport_height / 100.0,
            em: self.em,
            rem: 0.0,
            percent: self.percent,
            vw: 0.0,
            vh: 0.0,
        }
    }

    pub fn resolve(&self, em_size: f32, percent_base: Option<f32>) -> Option<f32> {
        let mut total = self.px + self.em * em_size;
        if self.percent != 0.0 {
            total += self.percent / 100.0 * percent_base?;
        }
        Some(total)
    }

    fn scaled(&self, factor: f32) -> Linear {
        Linear {
            px: self.px * factor,
            em: self.em * factor,
            rem: self.rem * factor,
            percent: self.percent * factor,
            vw: self.vw * factor,
            vh: self.vh * factor,
        }
    }

    fn plus(&self, other: &Linear) -> Linear {
        Linear {
            px: self.px + other.px,
            em: self.em + other.em,
            rem: self.rem + other.rem,
            percent: self.percent + other.percent,
            vw: self.vw + other.vw,
            vh: self.vh + other.vh,
        }
    }

    fn is_finite(&self) -> bool {
        [self.px, self.em, self.rem, self.percent, self.vw, self.vh]
            .iter()
            .all(|field| field.is_finite())
    }
}

pub fn parse_calc(text: &str) -> Option<Linear> {
    let inner = text.strip_prefix("calc(")?.strip_suffix(')')?;
    let mut parser = Parser {
        text: inner.as_bytes(),
        pos: 0,
    };
    let result = parser.parse_sum()?;
    parser.skip_whitespace();
    if parser.pos != parser.text.len() {
        return None;
    }
    match result {
        Operand::Length(linear) if linear.is_finite() => Some(linear),
        _ => None,
    }
}

struct Parser<'a> {
    text: &'a [u8],
    pos: usize,
}

fn add(left: Operand, right: Operand, sign: f32) -> Option<Operand> {
    match (left, right) {
        (Operand::Scalar(a), Operand::Scalar(b)) => Some(Operand::Scalar(a + sign * b)),
        (Operand::Length(a), Operand::Length(b)) => Some(Operand::Length(a.plus(&b.scaled(sign)))),
        _ => None,
    }
}

fn multiply(left: Operand, right: Operand) -> Option<Operand> {
    match (left, right) {
        (Operand::Scalar(a), Operand::Scalar(b)) => Some(Operand::Scalar(a * b)),
        (Operand::Scalar(factor), Operand::Length(length))
        | (Operand::Length(length), Operand::Scalar(factor)) => {
            Some(Operand::Length(length.scaled(factor)))
        }
        _ => None,
    }
}

fn divide(left: Operand, right: Operand) -> Option<Operand> {
    let Operand::Scalar(divisor) = right else {
        return None;
    };
    if divisor == 0.0 {
        return None;
    }
    match left {
        Operand::Scalar(a) => Some(Operand::Scalar(a / divisor)),
        Operand::Length(length) => Some(Operand::Length(length.scaled(1.0 / divisor))),
    }
}

impl Parser<'_> {
    fn skip_whitespace(&mut self) {
        while self.pos < self.text.len() && self.text[self.pos].is_ascii_whitespace() {
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.text.get(self.pos).copied()
    }

    fn parse_sum(&mut self) -> Option<Operand> {
        let mut left = self.parse_product()?;
        loop {
            self.skip_whitespace();
            let sign = match self.peek() {
                Some(b'+') => 1.0,
                Some(b'-') => -1.0,
                _ => return Some(left),
            };
            self.pos += 1;
            let right = self.parse_product()?;
            left = add(left, right, sign)?;
        }
    }

    fn parse_product(&mut self) -> Option<Operand> {
        let mut left = self.parse_factor()?;
        loop {
            self.skip_whitespace();
            match self.peek() {
                Some(b'*') => {
                    self.pos += 1;
                    let right = self.parse_factor()?;
                    left = multiply(left, right)?;
                }
                Some(b'/') => {
                    self.pos += 1;
                    let right = self.parse_factor()?;
                    left = divide(left, right)?;
                }
                _ => return Some(left),
            }
        }
    }

    fn parse_factor(&mut self) -> Option<Operand> {
        self.skip_whitespace();
        if self.text[self.pos..].starts_with(b"calc(") {
            self.pos += 4;
        }
        if self.peek() == Some(b'(') {
            self.pos += 1;
            let inner = self.parse_sum()?;
            self.skip_whitespace();
            if self.peek() != Some(b')') {
                return None;
            }
            self.pos += 1;
            return Some(inner);
        }
        self.parse_number()
    }

    fn parse_number(&mut self) -> Option<Operand> {
        let start = self.pos;
        if matches!(self.peek(), Some(b'+') | Some(b'-')) {
            self.pos += 1;
        }
        let digits_start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9') | Some(b'.')) {
            self.pos += 1;
        }
        if self.pos == digits_start {
            return None;
        }
        let number: f32 = std::str::from_utf8(&self.text[start..self.pos])
            .ok()?
            .parse()
            .ok()?;
        let unit_start = self.pos;
        while matches!(
            self.peek(),
            Some(b'a'..=b'z') | Some(b'A'..=b'Z') | Some(b'%')
        ) {
            self.pos += 1;
        }
        let unit = std::str::from_utf8(&self.text[unit_start..self.pos])
            .ok()?
            .to_ascii_lowercase();
        let mut linear = Linear::default();
        match unit.as_str() {
            "" => return Some(Operand::Scalar(number)),
            "px" => linear.px = number,
            "em" => linear.em = number,
            "rem" => linear.rem = number,
            "%" => linear.percent = number,
            "vw" => linear.vw = number,
            "vh" => linear.vh = number,
            _ => return None,
        }
        Some(Operand::Length(linear))
    }
}
