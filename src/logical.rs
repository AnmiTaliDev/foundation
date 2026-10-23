// SPDX-License-Identifier: GPL-3.0-only

use crate::css::{parse_declaration, split_values, Declaration};

const SINGLE: [(&str, &str); 48] = [
    ("margin-inline-start", "margin-left"),
    ("margin-inline-end", "margin-right"),
    ("margin-block-start", "margin-top"),
    ("margin-block-end", "margin-bottom"),
    ("padding-inline-start", "padding-left"),
    ("padding-inline-end", "padding-right"),
    ("padding-block-start", "padding-top"),
    ("padding-block-end", "padding-bottom"),
    ("inset-inline-start", "left"),
    ("inset-inline-end", "right"),
    ("inset-block-start", "top"),
    ("inset-block-end", "bottom"),
    ("scroll-margin-inline-start", "scroll-margin-left"),
    ("scroll-margin-inline-end", "scroll-margin-right"),
    ("scroll-margin-block-start", "scroll-margin-top"),
    ("scroll-margin-block-end", "scroll-margin-bottom"),
    ("scroll-padding-inline-start", "scroll-padding-left"),
    ("scroll-padding-inline-end", "scroll-padding-right"),
    ("scroll-padding-block-start", "scroll-padding-top"),
    ("scroll-padding-block-end", "scroll-padding-bottom"),
    ("border-inline-start-width", "border-left-width"),
    ("border-inline-end-width", "border-right-width"),
    ("border-block-start-width", "border-top-width"),
    ("border-block-end-width", "border-bottom-width"),
    ("border-inline-start-style", "border-left-style"),
    ("border-inline-end-style", "border-right-style"),
    ("border-block-start-style", "border-top-style"),
    ("border-block-end-style", "border-bottom-style"),
    ("border-inline-start-color", "border-left-color"),
    ("border-inline-end-color", "border-right-color"),
    ("border-block-start-color", "border-top-color"),
    ("border-block-end-color", "border-bottom-color"),
    ("border-inline-start", "border-left"),
    ("border-inline-end", "border-right"),
    ("border-block-start", "border-top"),
    ("border-block-end", "border-bottom"),
    ("inline-size", "width"),
    ("block-size", "height"),
    ("min-inline-size", "min-width"),
    ("min-block-size", "min-height"),
    ("max-inline-size", "max-width"),
    ("max-block-size", "max-height"),
    ("overflow-inline", "overflow-x"),
    ("overflow-block", "overflow-y"),
    ("border-start-start-radius", "border-top-left-radius"),
    ("border-start-end-radius", "border-top-right-radius"),
    ("border-end-start-radius", "border-bottom-left-radius"),
    ("border-end-end-radius", "border-bottom-right-radius"),
];

const PAIRS: [(&str, [&str; 2]); 14] = [
    ("margin-inline", ["margin-left", "margin-right"]),
    ("margin-block", ["margin-top", "margin-bottom"]),
    ("padding-inline", ["padding-left", "padding-right"]),
    ("padding-block", ["padding-top", "padding-bottom"]),
    ("inset-inline", ["left", "right"]),
    ("inset-block", ["top", "bottom"]),
    (
        "scroll-margin-inline",
        ["scroll-margin-left", "scroll-margin-right"],
    ),
    (
        "scroll-margin-block",
        ["scroll-margin-top", "scroll-margin-bottom"],
    ),
    (
        "scroll-padding-inline",
        ["scroll-padding-left", "scroll-padding-right"],
    ),
    (
        "scroll-padding-block",
        ["scroll-padding-top", "scroll-padding-bottom"],
    ),
    (
        "border-inline-width",
        ["border-left-width", "border-right-width"],
    ),
    (
        "border-block-width",
        ["border-top-width", "border-bottom-width"],
    ),
    (
        "border-inline-style",
        ["border-left-style", "border-right-style"],
    ),
    (
        "border-block-style",
        ["border-top-style", "border-bottom-style"],
    ),
];

const SHARED: [(&str, [&str; 2]); 2] = [
    ("border-inline", ["border-left", "border-right"]),
    ("border-block", ["border-top", "border-bottom"]),
];

const COLOR_PAIRS: [(&str, [&str; 2]); 2] = [
    (
        "border-inline-color",
        ["border-left-color", "border-right-color"],
    ),
    (
        "border-block-color",
        ["border-top-color", "border-bottom-color"],
    ),
];

const INLINE: [(&str, &str, &str, &str); 8] = [
    (
        "margin-inline-start",
        "margin-left",
        "margin-inline-end",
        "margin-right",
    ),
    (
        "padding-inline-start",
        "padding-left",
        "padding-inline-end",
        "padding-right",
    ),
    (
        "scroll-margin-inline-start",
        "scroll-margin-left",
        "scroll-margin-inline-end",
        "scroll-margin-right",
    ),
    (
        "scroll-padding-inline-start",
        "scroll-padding-left",
        "scroll-padding-inline-end",
        "scroll-padding-right",
    ),
    (
        "border-inline-start-width",
        "border-left-width",
        "border-inline-end-width",
        "border-right-width",
    ),
    (
        "border-inline-start-style",
        "border-left-style",
        "border-inline-end-style",
        "border-right-style",
    ),
    (
        "border-inline-start-color",
        "border-left-color",
        "border-inline-end-color",
        "border-right-color",
    ),
    ("inset-inline-start", "left", "inset-inline-end", "right"),
];

pub(crate) fn physical(name: &str, rtl: bool) -> &str {
    for (start, left, end, right) in INLINE {
        if name == start {
            return if rtl { right } else { left };
        }
        if name == end {
            return if rtl { left } else { right };
        }
    }
    name
}

fn relabel(declarations: Vec<Declaration>, start: bool) -> Vec<Declaration> {
    declarations
        .into_iter()
        .map(|mut declaration| {
            for (logical_start, left, logical_end, right) in INLINE {
                if start && declaration.name == left {
                    declaration.name = logical_start.to_string();
                    break;
                }
                if !start && declaration.name == right {
                    declaration.name = logical_end.to_string();
                    break;
                }
            }
            declaration
        })
        .collect()
}

pub(crate) fn expand(name: &str, value: &str) -> Option<Vec<Declaration>> {
    if let Some((_, target)) = SINGLE.iter().find(|(logical, _)| *logical == name) {
        let declarations = parse_declaration(&format!("{}: {}", target, value));
        return Some(relabel(declarations, !name.contains("inline-end")));
    }
    if let Some((_, sides)) = SHARED.iter().find(|(logical, _)| *logical == name) {
        let start = relabel(parse_declaration(&format!("{}: {}", sides[0], value)), true);
        let end = relabel(
            parse_declaration(&format!("{}: {}", sides[1], value)),
            false,
        );
        return Some(start.into_iter().chain(end).collect());
    }
    let sides = PAIRS
        .iter()
        .chain(COLOR_PAIRS.iter())
        .find(|(logical, _)| *logical == name)
        .map(|(_, sides)| sides)?;
    let tokens = split_values(value);
    let values = match tokens.as_slice() {
        [one] => [*one, *one],
        [start, end] => [*start, *end],
        _ => return Some(Vec::new()),
    };
    let start = parse_declaration(&format!("{}: {}", sides[0], values[0]));
    let end = parse_declaration(&format!("{}: {}", sides[1], values[1]));
    if start.is_empty() || end.is_empty() {
        return Some(Vec::new());
    }
    Some(
        relabel(start, true)
            .into_iter()
            .chain(relabel(end, false))
            .collect(),
    )
}
