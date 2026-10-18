// SPDX-License-Identifier: GPL-3.0-only

const REM_PX: f32 = 16.0;

#[derive(Debug, Clone, PartialEq)]
pub struct MediaList {
    queries: Vec<MediaQuery>,
}

#[derive(Debug, Clone, PartialEq)]
struct MediaQuery {
    negated: bool,
    screen: bool,
    valid: bool,
    conditions: Vec<Condition>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Condition {
    Compare(Dimension, Operator, f32),
    Portrait(bool),
    Constant(bool),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Dimension {
    Width,
    Height,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Operator {
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
}

impl Operator {
    fn mirrored(self) -> Operator {
        match self {
            Operator::Less => Operator::Greater,
            Operator::LessEqual => Operator::GreaterEqual,
            Operator::Greater => Operator::Less,
            Operator::GreaterEqual => Operator::LessEqual,
            Operator::Equal => Operator::Equal,
        }
    }

    fn holds(self, left: f32, right: f32) -> bool {
        match self {
            Operator::Less => left < right,
            Operator::LessEqual => left <= right,
            Operator::Greater => left > right,
            Operator::GreaterEqual => left >= right,
            Operator::Equal => left == right,
        }
    }
}

impl MediaList {
    pub fn parse(text: &str) -> MediaList {
        let queries = text
            .to_ascii_lowercase()
            .split(',')
            .map(parse_query)
            .collect();
        MediaList { queries }
    }

    pub fn matches(&self, width: f32, height: f32) -> bool {
        self.queries
            .iter()
            .any(|query| query.matches(width, height))
    }
}

impl MediaQuery {
    fn matches(&self, width: f32, height: f32) -> bool {
        if !self.valid {
            return false;
        }
        let result = self.screen
            && self
                .conditions
                .iter()
                .all(|condition| condition.holds(width, height));
        result != self.negated
    }
}

impl Condition {
    fn holds(self, width: f32, height: f32) -> bool {
        match self {
            Condition::Compare(Dimension::Width, operator, px) => operator.holds(width, px),
            Condition::Compare(Dimension::Height, operator, px) => operator.holds(height, px),
            Condition::Portrait(portrait) => (height >= width) == portrait,
            Condition::Constant(value) => value,
        }
    }
}

fn split_terms(text: &str) -> Vec<&str> {
    let mut terms = Vec::new();
    let mut depth = 0usize;
    let mut start: Option<usize> = None;
    for (index, c) in text.char_indices() {
        if c.is_whitespace() && depth == 0 {
            if let Some(begin) = start.take() {
                terms.push(&text[begin..index]);
            }
            continue;
        }
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            depth = depth.saturating_sub(1);
        }
        if start.is_none() {
            start = Some(index);
        }
    }
    if let Some(begin) = start {
        terms.push(&text[begin..]);
    }
    terms
}

fn parse_query(text: &str) -> MediaQuery {
    let mut query = MediaQuery {
        negated: false,
        screen: true,
        valid: true,
        conditions: Vec::new(),
    };
    for (index, term) in split_terms(text).into_iter().enumerate() {
        match term {
            "not" if index == 0 => query.negated = true,
            "only" | "and" | "all" | "screen" => {}
            _ => match term
                .strip_prefix('(')
                .and_then(|rest| rest.strip_suffix(')'))
            {
                Some(inner) => match parse_feature(inner.trim()) {
                    Some(conditions) => query.conditions.extend(conditions),
                    None => query.valid = false,
                },
                None if term.chars().all(|c| c.is_ascii_alphabetic()) => query.screen = false,
                None => query.valid = false,
            },
        }
    }
    query
}

fn parse_length(text: &str) -> Option<f32> {
    let text = text.trim();
    let (number, unit) = match text.find(|c: char| c.is_ascii_alphabetic()) {
        Some(split) => text.split_at(split),
        None => (text, ""),
    };
    let number: f32 = number
        .parse()
        .ok()
        .filter(|number: &f32| number.is_finite())?;
    match unit {
        "px" => Some(number),
        "em" | "rem" => Some(number * REM_PX),
        "" if number == 0.0 => Some(0.0),
        _ => None,
    }
}

fn dimension_of(name: &str) -> Option<Dimension> {
    match name.trim() {
        "width" => Some(Dimension::Width),
        "height" => Some(Dimension::Height),
        _ => None,
    }
}

fn parse_feature(inner: &str) -> Option<Vec<Condition>> {
    if let Some((name, value)) = inner.split_once(':') {
        let (name, value) = (name.trim(), value.trim());
        let (dimension, operator) = match name {
            "width" => (Dimension::Width, Operator::Equal),
            "height" => (Dimension::Height, Operator::Equal),
            "min-width" => (Dimension::Width, Operator::GreaterEqual),
            "max-width" => (Dimension::Width, Operator::LessEqual),
            "min-height" => (Dimension::Height, Operator::GreaterEqual),
            "max-height" => (Dimension::Height, Operator::LessEqual),
            "orientation" => {
                return match value {
                    "portrait" => Some(vec![Condition::Portrait(true)]),
                    "landscape" => Some(vec![Condition::Portrait(false)]),
                    _ => None,
                };
            }
            "prefers-color-scheme" => {
                return match value {
                    "light" => Some(vec![Condition::Constant(true)]),
                    "dark" => Some(vec![Condition::Constant(false)]),
                    _ => None,
                };
            }
            _ => return None,
        };
        return Some(vec![Condition::Compare(
            dimension,
            operator,
            parse_length(value)?,
        )]);
    }
    parse_range(inner)
}

fn parse_range(inner: &str) -> Option<Vec<Condition>> {
    let mut parts = Vec::new();
    let mut operators = Vec::new();
    let mut start = 0;
    let mut chars = inner.char_indices().peekable();
    while let Some((index, c)) = chars.next() {
        if !matches!(c, '<' | '>' | '=') {
            continue;
        }
        let mut end = index + 1;
        let operator = match (c, chars.peek()) {
            ('<', Some((_, '='))) => Operator::LessEqual,
            ('>', Some((_, '='))) => Operator::GreaterEqual,
            ('<', _) => Operator::Less,
            ('>', _) => Operator::Greater,
            _ => Operator::Equal,
        };
        if c != '=' && chars.peek().map(|(_, next)| *next) == Some('=') {
            chars.next();
            end += 1;
        }
        parts.push(&inner[start..index]);
        operators.push(operator);
        start = end;
    }
    parts.push(&inner[start..]);
    match (parts.as_slice(), operators.as_slice()) {
        ([left, right], [operator]) => {
            if let Some(dimension) = dimension_of(left) {
                Some(vec![Condition::Compare(
                    dimension,
                    *operator,
                    parse_length(right)?,
                )])
            } else {
                Some(vec![Condition::Compare(
                    dimension_of(right)?,
                    operator.mirrored(),
                    parse_length(left)?,
                )])
            }
        }
        ([low, name, high], [first, second]) => {
            let dimension = dimension_of(name)?;
            Some(vec![
                Condition::Compare(dimension, first.mirrored(), parse_length(low)?),
                Condition::Compare(dimension, *second, parse_length(high)?),
            ])
        }
        _ => None,
    }
}
