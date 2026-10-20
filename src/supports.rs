// SPDX-License-Identifier: GPL-3.0-only

use crate::css::{self, Value};
use crate::media::split_terms;

const PROPERTIES: [&str; 90] = [
    "opacity",
    "transform-origin-x",
    "transform-origin-y",
    "font-family",
    "font-weight",
    "font-style",
    "list-style",
    "list-style-type",
    "transform",
    "align-content",
    "align-items",
    "align-self",
    "background-color",
    "background-image",
    "background-position-x",
    "background-position-y",
    "background-repeat",
    "background-size",
    "background-size-height",
    "background-size-width",
    "border-bottom-width",
    "border-color",
    "border-left-width",
    "border-top-left-radius-x",
    "border-top-left-radius-y",
    "border-top-right-radius-x",
    "border-top-right-radius-y",
    "border-bottom-right-radius-x",
    "box-shadow",
    "border-bottom-right-radius-y",
    "border-bottom-left-radius-x",
    "border-bottom-left-radius-y",
    "border-right-width",
    "border-top-width",
    "bottom",
    "clear",
    "color",
    "column-gap",
    "display",
    "flex-basis",
    "flex-direction",
    "flex-grow",
    "flex-shrink",
    "flex-wrap",
    "float",
    "font-size",
    "height",
    "justify-content",
    "left",
    "line-height",
    "margin-bottom",
    "margin-left",
    "margin-right",
    "margin-top",
    "max-height",
    "max-width",
    "min-height",
    "min-width",
    "order",
    "overflow-x",
    "overflow-y",
    "padding-bottom",
    "padding-left",
    "padding-right",
    "padding-top",
    "position",
    "right",
    "row-gap",
    "letter-spacing",
    "text-align",
    "text-transform",
    "word-spacing",
    "text-decoration-color",
    "text-decoration-line",
    "text-decoration-style",
    "text-decoration-thickness",
    "top",
    "vertical-align",
    "white-space",
    "width",
    "z-index",
    "flex-flow",
    "flex",
    "gap",
    "margin",
    "padding",
    "border",
    "border-width",
    "background",
    "font",
];

const DISPLAYS: [&str; 10] = [
    "grid",
    "inline-grid",
    "table",
    "block",
    "inline",
    "flex",
    "inline-flex",
    "inline-block",
    "list-item",
    "none",
];
const POSITIONS: [&str; 5] = ["static", "relative", "absolute", "fixed", "sticky"];

pub fn evaluate(condition: &str) -> bool {
    let terms = split_terms(condition.trim());
    match terms.as_slice() {
        [] => false,
        ["not", group] => !evaluate_group(group),
        [first, rest @ ..] => {
            let mut result = evaluate_group(first);
            let mut index = 0;
            while index + 1 < rest.len() {
                let next = evaluate_group(rest[index + 1]);
                result = match rest[index] {
                    "and" => result && next,
                    "or" => result || next,
                    _ => return false,
                };
                index += 2;
            }
            index == rest.len() && result
        }
    }
}

fn evaluate_group(group: &str) -> bool {
    let Some(inner) = group
        .strip_prefix('(')
        .and_then(|rest| rest.strip_suffix(')'))
    else {
        return false;
    };
    let inner = inner.trim();
    if inner.starts_with('(') || inner.starts_with("not ") {
        return evaluate(inner);
    }
    supports_declaration(inner)
}

fn supports_declaration(text: &str) -> bool {
    let declarations = css::parse_declaration(text);
    if declarations.is_empty() {
        return false;
    }
    declarations.iter().all(|declaration| {
        if !PROPERTIES.contains(&declaration.name.as_str()) {
            return false;
        }
        let allowed: &[&str] = match declaration.name.as_str() {
            "display" => &DISPLAYS,
            "position" => &POSITIONS,
            _ => return true,
        };
        match &declaration.value {
            Value::Keyword(keyword) => allowed.contains(&keyword.as_str()),
            _ => false,
        }
    })
}
