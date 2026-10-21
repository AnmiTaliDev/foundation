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

#[derive(Debug, Clone, PartialEq)]
pub enum Math {
    Linear(Linear),
    Add(Box<Math>, Box<Math>),
    Scale(Box<Math>, f32),
    Min(Vec<Math>),
    Max(Vec<Math>),
    Clamp(Box<Math>, Box<Math>, Box<Math>),
}

impl Math {
    pub fn is_px_only(&self) -> bool {
        match self {
            Math::Linear(linear) => linear.is_px_only(),
            Math::Add(left, right) => left.is_px_only() && right.is_px_only(),
            Math::Scale(inner, _) => inner.is_px_only(),
            Math::Min(items) | Math::Max(items) => items.iter().all(Math::is_px_only),
            Math::Clamp(low, value, high) => {
                low.is_px_only() && value.is_px_only() && high.is_px_only()
            }
        }
    }

    pub fn fold_viewport(
        &self,
        root_font_size: f32,
        viewport_width: f32,
        viewport_height: f32,
    ) -> Math {
        let fold =
            |math: &Math| math.fold_viewport(root_font_size, viewport_width, viewport_height);
        match self {
            Math::Linear(linear) => {
                Math::Linear(linear.fold_viewport(root_font_size, viewport_width, viewport_height))
            }
            Math::Add(left, right) => Math::Add(Box::new(fold(left)), Box::new(fold(right))),
            Math::Scale(inner, factor) => Math::Scale(Box::new(fold(inner)), *factor),
            Math::Min(items) => Math::Min(items.iter().map(fold).collect()),
            Math::Max(items) => Math::Max(items.iter().map(fold).collect()),
            Math::Clamp(low, value, high) => Math::Clamp(
                Box::new(fold(low)),
                Box::new(fold(value)),
                Box::new(fold(high)),
            ),
        }
    }

    pub fn resolve(&self, em_size: f32, percent_base: Option<f32>) -> Option<f32> {
        let resolve = |math: &Math| math.resolve(em_size, percent_base);
        match self {
            Math::Linear(linear) => linear.resolve(em_size, percent_base),
            Math::Add(left, right) => Some(resolve(left)? + resolve(right)?),
            Math::Scale(inner, factor) => Some(resolve(inner)? * factor),
            Math::Min(items) => items
                .iter()
                .map(resolve)
                .try_fold(f32::INFINITY, |least, item| Some(least.min(item?))),
            Math::Max(items) => items
                .iter()
                .map(resolve)
                .try_fold(f32::NEG_INFINITY, |most, item| Some(most.max(item?))),
            Math::Clamp(low, value, high) => {
                Some(resolve(low)?.max(resolve(value)?.min(resolve(high)?)))
            }
        }
    }
}

#[derive(Clone)]
enum Operand {
    Scalar(f32),
    Length(Linear),
    Math(Math),
}

impl Operand {
    fn into_math(self) -> Option<Math> {
        match self {
            Operand::Scalar(_) => None,
            Operand::Length(linear) => Some(Math::Linear(linear)),
            Operand::Math(math) => Some(math),
        }
    }
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

pub fn parse_math(text: &str) -> Option<Math> {
    let mut parser = Parser {
        text: text.as_bytes(),
        pos: 0,
    };
    let result = parser.parse_factor()?;
    parser.skip_whitespace();
    if parser.pos != parser.text.len() {
        return None;
    }
    match result {
        Operand::Length(linear) if linear.is_finite() => Some(Math::Linear(linear)),
        Operand::Math(math) => Some(math),
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
        (left, right) => Some(Operand::Math(Math::Add(
            Box::new(left.into_math()?),
            Box::new(Math::Scale(Box::new(right.into_math()?), sign)),
        ))),
    }
}

fn multiply(left: Operand, right: Operand) -> Option<Operand> {
    match (left, right) {
        (Operand::Scalar(a), Operand::Scalar(b)) => Some(Operand::Scalar(a * b)),
        (Operand::Scalar(factor), Operand::Length(length))
        | (Operand::Length(length), Operand::Scalar(factor)) => {
            Some(Operand::Length(length.scaled(factor)))
        }
        (Operand::Scalar(factor), Operand::Math(math))
        | (Operand::Math(math), Operand::Scalar(factor)) => {
            Some(Operand::Math(Math::Scale(Box::new(math), factor)))
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
        Operand::Math(math) => Some(Operand::Math(Math::Scale(Box::new(math), 1.0 / divisor))),
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

    fn parse_arguments(&mut self) -> Option<Vec<Math>> {
        let mut arguments = Vec::new();
        loop {
            arguments.push(self.parse_sum()?.into_math()?);
            self.skip_whitespace();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b')') => {
                    self.pos += 1;
                    return Some(arguments);
                }
                _ => return None,
            }
        }
    }

    fn parse_factor(&mut self) -> Option<Operand> {
        self.skip_whitespace();
        if self.text[self.pos..].starts_with(b"calc(") {
            self.pos += 4;
        }
        for (name, build) in [
            ("min(", Math::Min as fn(Vec<Math>) -> Math),
            ("max(", Math::Max as fn(Vec<Math>) -> Math),
        ] {
            if self.text[self.pos..].starts_with(name.as_bytes()) {
                self.pos += name.len();
                return Some(Operand::Math(build(self.parse_arguments()?)));
            }
        }
        if self.text[self.pos..].starts_with(b"clamp(") {
            self.pos += 6;
            let mut arguments = self.parse_arguments()?.into_iter();
            let (low, value, high) = (arguments.next()?, arguments.next()?, arguments.next()?);
            if arguments.next().is_some() {
                return None;
            }
            return Some(Operand::Math(Math::Clamp(
                Box::new(low),
                Box::new(value),
                Box::new(high),
            )));
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
