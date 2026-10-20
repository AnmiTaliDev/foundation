// SPDX-License-Identifier: GPL-3.0-only

use crate::counters::Counters;
use crate::dom::ElementData;
use crate::list;

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Literal(String),
    Attr(String),
    Quote(Quote),
    Counter {
        name: String,
        style: String,
    },
    Counters {
        name: String,
        separator: String,
        style: String,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Quote {
    Open,
    Close,
    NoOpen,
    NoClose,
}

pub const DEFAULT_QUOTES: [(&str, &str); 2] = [("\u{201C}", "\u{201D}"), ("\u{2018}", "\u{2019}")];

pub fn parse_strings(text: &str) -> Option<Vec<String>> {
    let mut strings = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        let (literal, after) = string(rest)?;
        strings.push(literal);
        rest = after.trim_start();
    }
    Some(strings)
}

pub fn parse(text: &str) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if rest.starts_with('"') || rest.starts_with('\'') {
            let (literal, after) = string(rest)?;
            parts.push(Part::Literal(literal));
            rest = after.trim_start();
        } else if let Some((quote, after)) = quote_keyword(rest) {
            parts.push(Part::Quote(quote));
            rest = after.trim_start();
        } else if let Some(inner) = rest.strip_prefix("attr(") {
            let close = inner.find(')')?;
            let name = inner[..close].trim().to_ascii_lowercase();
            if !is_name(&name) {
                return None;
            }
            parts.push(Part::Attr(name));
            rest = inner[close + 1..].trim_start();
        } else {
            let (separated, inner) = match rest.strip_prefix("counters(") {
                Some(inner) => (true, inner),
                None => (false, rest.strip_prefix("counter(")?),
            };
            let close = closing_paren(inner)?;
            parts.push(counter_part(&inner[..close], separated)?);
            rest = inner[close + 1..].trim_start();
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts)
}

fn quote_keyword(text: &str) -> Option<(Quote, &str)> {
    let end = text.find(char::is_whitespace).unwrap_or(text.len());
    let quote = match text[..end].to_ascii_lowercase().as_str() {
        "open-quote" => Quote::Open,
        "close-quote" => Quote::Close,
        "no-open-quote" => Quote::NoOpen,
        "no-close-quote" => Quote::NoClose,
        _ => return None,
    };
    Some((quote, &text[end..]))
}

fn is_name(name: &str) -> bool {
    !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

fn closing_paren(text: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, c) in text.char_indices() {
        if let Some(open) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == open {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == ')' {
            return Some(index);
        }
    }
    None
}

fn counter_part(arguments: &str, separated: bool) -> Option<Part> {
    let mut items = Vec::new();
    let mut rest = arguments;
    loop {
        let rest_trimmed = rest.trim_start();
        let end = if rest_trimmed.starts_with('"') || rest_trimmed.starts_with('\'') {
            let (_, after) = string(rest_trimmed)?;
            rest_trimmed.len() - after.len()
        } else {
            rest_trimmed.find(',').unwrap_or(rest_trimmed.len())
        };
        items.push(rest_trimmed[..end].trim());
        let after = rest_trimmed[end..].trim_start();
        match after.strip_prefix(',') {
            Some(next) => rest = next,
            None if after.is_empty() => break,
            None => return None,
        }
    }
    let name = items.first()?.to_ascii_lowercase();
    if !is_name(&name) {
        return None;
    }
    let style_index = if separated { 2 } else { 1 };
    if items.len() < style_index || items.len() > style_index + 1 {
        return None;
    }
    let style = match items.get(style_index) {
        Some(style) => style.to_ascii_lowercase(),
        None => "decimal".to_string(),
    };
    if !list::is_type(&style) {
        return None;
    }
    if !separated {
        return Some(Part::Counter { name, style });
    }
    let (separator, after) = string(items.get(1)?)?;
    if !after.is_empty() {
        return None;
    }
    Some(Part::Counters {
        name,
        separator,
        style,
    })
}

fn string(text: &str) -> Option<(String, &str)> {
    let mut chars = text.char_indices();
    let (_, quote) = chars.next()?;
    let mut result = String::new();
    while let Some((index, c)) = chars.next() {
        if c == quote {
            return Some((result, &text[index + 1..]));
        }
        if c != '\\' {
            result.push(c);
            continue;
        }
        let (start, escaped) = chars.next()?;
        if escaped == '\n' {
            continue;
        }
        if !escaped.is_ascii_hexdigit() {
            result.push(escaped);
            continue;
        }
        let digits = text[start..]
            .chars()
            .take(6)
            .take_while(char::is_ascii_hexdigit)
            .count();
        let code = u32::from_str_radix(&text[start..start + digits], 16).ok()?;
        result.push(
            char::from_u32(code)
                .filter(|c| *c != '\0')
                .unwrap_or('\u{FFFD}'),
        );
        for _ in 1..digits {
            chars.next();
        }
        let after = start + digits;
        if text[after..].starts_with(|c: char| c.is_ascii_whitespace()) {
            chars.next();
        }
    }
    None
}

pub fn render(
    parts: &[Part],
    element: &ElementData,
    counters: &Counters,
    quotes: &[(String, String)],
    depth: &mut usize,
) -> String {
    let mut text = String::new();
    for part in parts {
        match part {
            Part::Literal(literal) => text.push_str(literal),
            Part::Attr(name) => {
                if let Some(value) = element.attributes.get(name) {
                    text.push_str(value);
                }
            }
            Part::Quote(quote) => {
                let pair = |level: usize| quotes.get(level.min(quotes.len().saturating_sub(1)));
                match quote {
                    Quote::Open => {
                        if let Some(pair) = pair(*depth) {
                            text.push_str(&pair.0);
                        }
                        *depth += 1;
                    }
                    Quote::Close if *depth > 0 => {
                        *depth -= 1;
                        if let Some(pair) = pair(*depth) {
                            text.push_str(&pair.1);
                        }
                    }
                    Quote::NoOpen => *depth += 1,
                    Quote::NoClose => *depth = depth.saturating_sub(1),
                    Quote::Close => {}
                }
            }
            Part::Counter { name, style } => {
                text.push_str(&list::counter_text(style, counters.value(name)));
            }
            Part::Counters {
                name,
                separator,
                style,
            } => {
                let values = counters.values(name);
                let values = if values.is_empty() { vec![0] } else { values };
                let texts: Vec<String> = values
                    .into_iter()
                    .map(|value| list::counter_text(style, value))
                    .collect();
                text.push_str(&texts.join(separator));
            }
        }
    }
    text
}
