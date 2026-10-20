// SPDX-License-Identifier: GPL-3.0-only

use crate::dom::ElementData;

#[derive(Debug, Clone, PartialEq)]
pub enum Part {
    Literal(String),
    Attr(String),
}

pub fn parse(text: &str) -> Option<Vec<Part>> {
    let mut parts = Vec::new();
    let mut rest = text.trim();
    while !rest.is_empty() {
        if rest.starts_with('"') || rest.starts_with('\'') {
            let (literal, after) = string(rest)?;
            parts.push(Part::Literal(literal));
            rest = after.trim_start();
        } else {
            let inner = rest.strip_prefix("attr(")?;
            let close = inner.find(')')?;
            let name = inner[..close].trim().to_ascii_lowercase();
            if name.is_empty() || !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
                return None;
            }
            parts.push(Part::Attr(name));
            rest = inner[close + 1..].trim_start();
        }
    }
    if parts.is_empty() {
        return None;
    }
    Some(parts)
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

pub fn render(parts: &[Part], element: &ElementData) -> String {
    let mut text = String::new();
    for part in parts {
        match part {
            Part::Literal(literal) => text.push_str(literal),
            Part::Attr(name) => {
                if let Some(value) = element.attributes.get(name) {
                    text.push_str(value);
                }
            }
        }
    }
    text
}
