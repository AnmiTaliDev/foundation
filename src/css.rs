// SPDX-License-Identifier: GPL-3.0-only

use crate::calc::{self, Linear};
use crate::media::MediaList;

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
    pub media: Vec<MediaList>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub parent: Option<(Combinator, Box<Selector>)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    Descendant,
    Child,
}

pub type Specificity = (usize, usize, usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32),
    Em(f32),
    Percent(f32),
    Number(f32),
    Rem(f32),
    Vw(f32),
    Vh(f32),
    Vmin(f32),
    Vmax(f32),
    Calc(Linear),
    Color(Color),
    Url(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

const MARGIN_PROPERTIES: [&str; 4] = ["margin-top", "margin-right", "margin-bottom", "margin-left"];

const PADDING_PROPERTIES: [&str; 4] = [
    "padding-top",
    "padding-right",
    "padding-bottom",
    "padding-left",
];

const BORDER_WIDTH_PROPERTIES: [&str; 4] = [
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];

const BOX_SHORTHANDS: [(&str, [&str; 4]); 3] = [
    ("margin", MARGIN_PROPERTIES),
    ("padding", PADDING_PROPERTIES),
    ("border-width", BORDER_WIDTH_PROPERTIES),
];

const FONT_SIZE_KEYWORDS: [(&str, f32); 7] = [
    ("xx-small", 9.0),
    ("x-small", 10.0),
    ("small", 13.0),
    ("medium", 16.0),
    ("large", 18.0),
    ("x-large", 24.0),
    ("xx-large", 32.0),
];

const NAMED_COLORS: [(&str, Color); 16] = [
    ("black", Color { r: 0, g: 0, b: 0 }),
    (
        "silver",
        Color {
            r: 192,
            g: 192,
            b: 192,
        },
    ),
    (
        "gray",
        Color {
            r: 128,
            g: 128,
            b: 128,
        },
    ),
    (
        "white",
        Color {
            r: 255,
            g: 255,
            b: 255,
        },
    ),
    ("maroon", Color { r: 128, g: 0, b: 0 }),
    ("red", Color { r: 255, g: 0, b: 0 }),
    (
        "purple",
        Color {
            r: 128,
            g: 0,
            b: 128,
        },
    ),
    (
        "fuchsia",
        Color {
            r: 255,
            g: 0,
            b: 255,
        },
    ),
    ("green", Color { r: 0, g: 128, b: 0 }),
    ("lime", Color { r: 0, g: 255, b: 0 }),
    (
        "olive",
        Color {
            r: 128,
            g: 128,
            b: 0,
        },
    ),
    (
        "yellow",
        Color {
            r: 255,
            g: 255,
            b: 0,
        },
    ),
    ("navy", Color { r: 0, g: 0, b: 128 }),
    ("blue", Color { r: 0, g: 0, b: 255 }),
    (
        "teal",
        Color {
            r: 0,
            g: 128,
            b: 128,
        },
    ),
    (
        "aqua",
        Color {
            r: 0,
            g: 255,
            b: 255,
        },
    ),
];

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let mut specificity = (
            usize::from(self.id.is_some()),
            self.classes.len(),
            usize::from(self.tag_name.is_some()),
        );
        if let Some((_, parent)) = &self.parent {
            let (ids, classes, tags) = parent.specificity();
            specificity.0 += ids;
            specificity.1 += classes;
            specificity.2 += tags;
        }
        specificity
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub url: String,
    pub media: String,
}

enum TopLevel<'a> {
    Statement(&'a str),
    Block { prelude: &'a str, body: &'a str },
}

fn scan_top_level(source: &str) -> Vec<TopLevel<'_>> {
    let mut items = Vec::new();
    let mut start = 0;
    let mut prelude_end = 0;
    let mut body_start = 0;
    let mut braces = 0usize;
    let mut parens = 0usize;
    let mut quote: Option<char> = None;
    for (index, c) in source.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' => parens += 1,
            ')' => parens = parens.saturating_sub(1),
            ';' if braces == 0 && parens == 0 => {
                let text = source[start..index].trim();
                if !text.is_empty() {
                    items.push(TopLevel::Statement(text));
                }
                start = index + 1;
            }
            '{' => {
                if braces == 0 {
                    prelude_end = index;
                    body_start = index + 1;
                }
                braces += 1;
            }
            '}' => {
                if braces == 0 {
                    start = index + 1;
                } else {
                    braces -= 1;
                    if braces == 0 {
                        items.push(TopLevel::Block {
                            prelude: source[start..prelude_end].trim(),
                            body: &source[body_start..index],
                        });
                        start = index + 1;
                    }
                }
            }
            _ => {}
        }
    }
    if braces > 0 {
        items.push(TopLevel::Block {
            prelude: source[start..prelude_end].trim(),
            body: &source[body_start..],
        });
    }
    items
}

const MAX_MEDIA_DEPTH: usize = 8;

pub fn parse(source: &str) -> Stylesheet {
    let source = strip_comments(source);
    let mut rules = Vec::new();
    collect_rules(&source, &[], &mut rules);
    Stylesheet { rules }
}

fn collect_rules(source: &str, media: &[MediaList], rules: &mut Vec<Rule>) {
    for item in scan_top_level(source) {
        let TopLevel::Block { prelude, body } = item else {
            continue;
        };
        if !prelude.starts_with('@') {
            if let Some(mut rule) = parse_rule(prelude, body) {
                rule.media = media.to_vec();
                rules.push(rule);
            }
            continue;
        }
        let Some(condition) = media_condition(prelude) else {
            continue;
        };
        if media.len() >= MAX_MEDIA_DEPTH {
            continue;
        }
        let mut nested = media.to_vec();
        nested.push(MediaList::parse(condition));
        collect_rules(body, &nested, rules);
    }
}

fn media_condition(prelude: &str) -> Option<&str> {
    let head = prelude.get(..6)?;
    if !head.eq_ignore_ascii_case("@media") {
        return None;
    }
    let rest = &prelude[6..];
    if rest.is_empty() || rest.starts_with(|c: char| c.is_whitespace() || c == '(') {
        Some(rest.trim())
    } else {
        None
    }
}

pub fn parse_imports(source: &str) -> Vec<Import> {
    let source = strip_comments(source);
    let mut imports = Vec::new();
    for item in scan_top_level(&source) {
        match item {
            TopLevel::Statement(text) => {
                let Some(prefix) = text.get(..7) else {
                    continue;
                };
                if prefix.eq_ignore_ascii_case("@import") {
                    if let Some(import) = parse_import(&text[7..]) {
                        imports.push(import);
                    }
                }
            }
            TopLevel::Block { .. } => break,
        }
    }
    imports
}

fn parse_import(text: &str) -> Option<Import> {
    let text = text.trim();
    let (url, rest) = if let Some(after) = text.strip_prefix("url(") {
        let close = after.find(')')?;
        (unquote(after[..close].trim())?, &after[close + 1..])
    } else {
        let quote = text.chars().next().filter(|c| *c == '"' || *c == '\'')?;
        let inner = &text[1..];
        let close = inner.find(quote)?;
        (inner[..close].to_string(), &inner[close + 1..])
    };
    if url.trim().is_empty() {
        return None;
    }
    Some(Import {
        url,
        media: rest.trim().to_string(),
    })
}

fn unquote(text: &str) -> Option<String> {
    let first = text.chars().next()?;
    if first == '"' || first == '\'' {
        let inner = &text[1..];
        let close = inner.find(first)?;
        Some(inner[..close].to_string())
    } else {
        Some(text.to_string())
    }
}

fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        let body = &rest[start + 2..];
        rest = match body.find("*/") {
            Some(end) => &body[end + 2..],
            None => "",
        };
    }
    out.push_str(rest);
    out
}

fn parse_rule(selector_text: &str, block: &str) -> Option<Rule> {
    let mut selectors = Vec::new();
    for part in selector_text.split(',') {
        selectors.push(parse_selector(part.trim())?);
    }
    Some(Rule {
        selectors,
        declarations: parse_declarations(block),
        media: Vec::new(),
    })
}

pub fn parse_declarations(block: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for item in block.split(';') {
        declarations.extend(parse_declaration(item));
    }
    declarations
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn take_ident(text: &str) -> &str {
    let rest = text.trim_start_matches(is_ident_char);
    &text[..text.len() - rest.len()]
}

fn parse_selector(text: &str) -> Option<Selector> {
    let spaced = text.replace('>', " > ");
    let mut result: Option<Selector> = None;
    let mut combinator: Option<Combinator> = None;
    for token in spaced.split_whitespace() {
        if token == ">" {
            if result.is_none() || combinator.is_some() {
                return None;
            }
            combinator = Some(Combinator::Child);
            continue;
        }
        let mut compound = parse_compound(token)?;
        if let Some(left) = result {
            let kind = combinator.take().unwrap_or(Combinator::Descendant);
            compound.parent = Some((kind, Box::new(left)));
        }
        result = Some(compound);
    }
    if combinator.is_some() {
        return None;
    }
    result
}

fn parse_compound(text: &str) -> Option<Selector> {
    let mut selector = Selector {
        tag_name: None,
        id: None,
        classes: Vec::new(),
        parent: None,
    };
    let mut rest = text;
    if let Some(after_star) = rest.strip_prefix('*') {
        rest = after_star;
    } else {
        let name = take_ident(rest);
        if !name.is_empty() {
            selector.tag_name = Some(name.to_ascii_lowercase());
            rest = &rest[name.len()..];
        }
    }
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('#') {
            let id = take_ident(after);
            if id.is_empty() {
                return None;
            }
            selector.id = Some(id.to_string());
            rest = &after[id.len()..];
        } else {
            let after = rest.strip_prefix('.')?;
            let class = take_ident(after);
            if class.is_empty() {
                return None;
            }
            selector.classes.push(class.to_string());
            rest = &after[class.len()..];
        }
    }
    Some(selector)
}

fn parse_declaration(item: &str) -> Vec<Declaration> {
    let Some((name, value)) = item.split_once(':') else {
        return Vec::new();
    };
    let name = name.trim().to_ascii_lowercase();
    if name.is_empty() {
        return Vec::new();
    }
    let value = value.trim();

    if name == "border" {
        return expand_border(value);
    }
    if name == "font" {
        return expand_font(value);
    }
    if name == "flex" {
        return expand_flex(value);
    }
    if name == "flex-flow" {
        return expand_flex_flow(value);
    }
    if name == "gap" {
        return expand_gap(value);
    }
    if name == "background" {
        return expand_background(value);
    }
    if name == "background-position" {
        return expand_position(&split_values(value));
    }
    if name == "background-size" {
        return expand_size(&split_values(value));
    }
    if name == "font-size" {
        if let Some(size) = parse_font_size(value) {
            return vec![Declaration { name, value: size }];
        }
    }

    for (shorthand, longhands) in BOX_SHORTHANDS {
        if name == shorthand {
            return expand_box_shorthand(&longhands, value);
        }
    }

    match parse_value(value) {
        Some(value) => vec![Declaration { name, value }],
        None => Vec::new(),
    }
}

fn split_values(text: &str) -> Vec<&str> {
    let mut values = Vec::new();
    let mut depth = 0usize;
    let mut start: Option<usize> = None;
    for (index, c) in text.char_indices() {
        if c.is_whitespace() && depth == 0 {
            if let Some(begin) = start.take() {
                values.push(&text[begin..index]);
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
        values.push(&text[begin..]);
    }
    values
}

fn expand_background(text: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    let mut position = Vec::new();
    let mut size = Vec::new();
    let mut in_size = false;
    for token in split_values(text) {
        if token == "/" {
            in_size = true;
            continue;
        }
        let (token, starts_size) = match token.strip_prefix('/') {
            Some(rest) => (rest, true),
            None => (token, false),
        };
        in_size |= starts_size;
        if token.is_empty() {
            continue;
        }
        match parse_value(token) {
            Some(Value::Color(color)) => declarations.push(Declaration {
                name: "background-color".to_string(),
                value: Value::Color(color),
            }),
            Some(Value::Url(url)) => declarations.push(Declaration {
                name: "background-image".to_string(),
                value: Value::Url(url),
            }),
            Some(Value::Keyword(keyword)) if is_repeat_keyword(&keyword) => {
                declarations.push(Declaration {
                    name: "background-repeat".to_string(),
                    value: Value::Keyword(keyword),
                })
            }
            Some(_) if in_size => size.push(token),
            Some(_) => position.push(token),
            None => {}
        }
    }
    declarations.extend(expand_position(&position));
    declarations.extend(expand_size(&size));
    declarations
}

fn is_repeat_keyword(keyword: &str) -> bool {
    matches!(keyword, "repeat" | "repeat-x" | "repeat-y" | "no-repeat")
}

fn position_component(token: &str) -> Option<(Option<char>, Value)> {
    let edge = |axis, percent| Some((axis, Value::Percent(percent)));
    match token.to_ascii_lowercase().as_str() {
        "left" => edge(Some('x'), 0.0),
        "right" => edge(Some('x'), 100.0),
        "top" => edge(Some('y'), 0.0),
        "bottom" => edge(Some('y'), 100.0),
        "center" => edge(None, 50.0),
        _ => match parse_value(token)? {
            value @ (Value::Length(_)
            | Value::Em(_)
            | Value::Percent(_)
            | Value::Rem(_)
            | Value::Vw(_)
            | Value::Vh(_)
            | Value::Vmin(_)
            | Value::Vmax(_)) => Some((None, value)),
            _ => None,
        },
    }
}

fn expand_position(tokens: &[&str]) -> Vec<Declaration> {
    let center = Value::Percent(50.0);
    let (x, y) = match tokens {
        [one] => match position_component(one) {
            Some((Some('y'), value)) => (center.clone(), value),
            Some((_, value)) => (value, center.clone()),
            None => return Vec::new(),
        },
        [first, second] => {
            let (Some(first), Some(second)) =
                (position_component(first), position_component(second))
            else {
                return Vec::new();
            };
            if first.0 == Some('y') || second.0 == Some('x') {
                (second.1, first.1)
            } else {
                (first.1, second.1)
            }
        }
        _ => return Vec::new(),
    };
    vec![
        Declaration {
            name: "background-position-x".to_string(),
            value: x,
        },
        Declaration {
            name: "background-position-y".to_string(),
            value: y,
        },
    ]
}

fn size_component(token: &str) -> Option<Value> {
    match parse_value(token)? {
        value @ (Value::Length(_)
        | Value::Em(_)
        | Value::Percent(_)
        | Value::Rem(_)
        | Value::Vw(_)
        | Value::Vh(_)
        | Value::Vmin(_)
        | Value::Vmax(_)) => Some(value),
        Value::Keyword(keyword) if keyword == "auto" => Some(Value::Keyword(keyword)),
        _ => None,
    }
}

fn expand_size(tokens: &[&str]) -> Vec<Declaration> {
    let auto = Value::Keyword("auto".to_string());
    let (width, height) = match tokens {
        [one] if matches!(one.to_ascii_lowercase().as_str(), "cover" | "contain") => {
            return vec![Declaration {
                name: "background-size".to_string(),
                value: Value::Keyword(one.to_ascii_lowercase()),
            }];
        }
        [one] => match size_component(one) {
            Some(width) => (width, auto),
            None => return Vec::new(),
        },
        [first, second] => match (size_component(first), size_component(second)) {
            (Some(width), Some(height)) => (width, height),
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    vec![
        Declaration {
            name: "background-size".to_string(),
            value: Value::Keyword("explicit".to_string()),
        },
        Declaration {
            name: "background-size-width".to_string(),
            value: width,
        },
        Declaration {
            name: "background-size-height".to_string(),
            value: height,
        },
    ]
}

fn parse_font_size(text: &str) -> Option<Value> {
    let lower = text.to_ascii_lowercase();
    for (keyword, size) in FONT_SIZE_KEYWORDS {
        if lower == keyword {
            return Some(Value::Length(size));
        }
    }
    match parse_value(text)? {
        size @ (Value::Length(_)
        | Value::Em(_)
        | Value::Percent(_)
        | Value::Rem(_)
        | Value::Vw(_)
        | Value::Vh(_)
        | Value::Vmin(_)
        | Value::Vmax(_)
        | Value::Calc(_)) => Some(size),
        _ => None,
    }
}

fn expand_font(text: &str) -> Vec<Declaration> {
    let tokens = split_values(text);
    for (index, token) in tokens.iter().enumerate() {
        let (size_text, mut line_height_text) = match token.split_once('/') {
            Some((size, line_height)) => (size, Some(line_height)),
            None => (*token, None),
        };
        let Some(size) = parse_font_size(size_text) else {
            continue;
        };
        if line_height_text.is_none() && tokens.get(index + 1) == Some(&"/") {
            line_height_text = tokens.get(index + 2).copied();
        }
        let line_height = match line_height_text {
            Some(line_height) => match parse_value(line_height) {
                Some(value) => value,
                None => return Vec::new(),
            },
            None => Value::Keyword("normal".to_string()),
        };
        return vec![
            Declaration {
                name: "font-size".to_string(),
                value: size,
            },
            Declaration {
                name: "line-height".to_string(),
                value: line_height,
            },
        ];
    }
    Vec::new()
}

fn plain_number(token: &str) -> Option<f32> {
    let first = token.chars().next()?;
    if !(first.is_ascii_digit() || first == '.') {
        return None;
    }
    parse_number(token)
}

fn declaration(name: &str, value: Value) -> Declaration {
    Declaration {
        name: name.to_string(),
        value,
    }
}

fn expand_flex(text: &str) -> Vec<Declaration> {
    let tokens = split_values(text);
    let keyword = |grow: f32, shrink: f32, basis: &str| {
        vec![
            declaration("flex-grow", Value::Number(grow)),
            declaration("flex-shrink", Value::Number(shrink)),
            declaration("flex-basis", Value::Keyword(basis.to_string())),
        ]
    };
    if tokens.len() == 1 {
        match tokens[0].to_ascii_lowercase().as_str() {
            "none" => return keyword(0.0, 0.0, "auto"),
            "auto" => return keyword(1.0, 1.0, "auto"),
            "initial" => return keyword(0.0, 1.0, "auto"),
            _ => {}
        }
    }
    let mut grow = 1.0;
    let mut shrink = 1.0;
    let mut basis = Value::Percent(0.0);
    let mut index = 0;
    if let Some(number) = tokens.first().and_then(|token| plain_number(token)) {
        grow = number;
        index = 1;
        if let Some(number) = tokens.get(1).and_then(|token| plain_number(token)) {
            shrink = number;
            index = 2;
        }
    }
    if index < tokens.len() {
        match parse_value(tokens[index]) {
            Some(
                value @ (Value::Length(_)
                | Value::Em(_)
                | Value::Percent(_)
                | Value::Rem(_)
                | Value::Vw(_)
                | Value::Vh(_)
                | Value::Vmin(_)
                | Value::Vmax(_)
                | Value::Calc(_)),
            ) => basis = value,
            Some(Value::Keyword(keyword)) if keyword == "auto" || keyword == "content" => {
                basis = Value::Keyword(keyword)
            }
            _ => return Vec::new(),
        }
        index += 1;
    }
    if tokens.is_empty() || index != tokens.len() || grow < 0.0 || shrink < 0.0 {
        return Vec::new();
    }
    vec![
        declaration("flex-grow", Value::Number(grow)),
        declaration("flex-shrink", Value::Number(shrink)),
        declaration("flex-basis", basis),
    ]
}

fn expand_flex_flow(text: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for token in split_values(text) {
        let keyword = token.to_ascii_lowercase();
        match keyword.as_str() {
            "row" | "row-reverse" | "column" | "column-reverse" => {
                declarations.push(declaration("flex-direction", Value::Keyword(keyword)));
            }
            "nowrap" | "wrap" | "wrap-reverse" => {
                declarations.push(declaration("flex-wrap", Value::Keyword(keyword)));
            }
            _ => return Vec::new(),
        }
    }
    declarations
}

fn expand_gap(text: &str) -> Vec<Declaration> {
    let mut values = Vec::new();
    for token in split_values(text) {
        match parse_value(token) {
            Some(
                value @ (Value::Length(_)
                | Value::Em(_)
                | Value::Percent(_)
                | Value::Rem(_)
                | Value::Vw(_)
                | Value::Vh(_)
                | Value::Vmin(_)
                | Value::Vmax(_)
                | Value::Calc(_)),
            ) => values.push(value),
            _ => return Vec::new(),
        }
    }
    match values.len() {
        1 => vec![
            declaration("row-gap", values[0].clone()),
            declaration("column-gap", values[0].clone()),
        ],
        2 => vec![
            declaration("row-gap", values[0].clone()),
            declaration("column-gap", values[1].clone()),
        ],
        _ => Vec::new(),
    }
}

fn parse_light_dark(text: &str) -> Option<Value> {
    let inner = text.strip_prefix("light-dark(")?.strip_suffix(')')?;
    let mut depth = 0usize;
    for (index, c) in inner.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => return parse_value(inner[..index].trim()),
            _ => {}
        }
    }
    None
}

fn expand_border(text: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for token in split_values(text) {
        match parse_value(token) {
            Some(Value::Color(color)) => declarations.push(Declaration {
                name: "border-color".to_string(),
                value: Value::Color(color),
            }),
            Some(width @ (Value::Length(_) | Value::Em(_))) => {
                push_border_widths(&mut declarations, width);
            }
            Some(Value::Keyword(keyword)) if keyword == "none" => {
                push_border_widths(&mut declarations, Value::Length(0.0));
            }
            Some(Value::Keyword(_)) => {}
            _ => return Vec::new(),
        }
    }
    declarations
}

fn push_border_widths(declarations: &mut Vec<Declaration>, width: Value) {
    for name in BORDER_WIDTH_PROPERTIES {
        declarations.push(Declaration {
            name: name.to_string(),
            value: width.clone(),
        });
    }
}

fn expand_box_shorthand(longhands: &[&str; 4], text: &str) -> Vec<Declaration> {
    let mut values = Vec::new();
    for token in split_values(text) {
        match parse_value(token) {
            Some(value) => values.push(value),
            None => return Vec::new(),
        }
    }
    let ordered = match values.len() {
        1 => [
            values[0].clone(),
            values[0].clone(),
            values[0].clone(),
            values[0].clone(),
        ],
        2 => [
            values[0].clone(),
            values[1].clone(),
            values[0].clone(),
            values[1].clone(),
        ],
        3 => [
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            values[1].clone(),
        ],
        4 => [
            values[0].clone(),
            values[1].clone(),
            values[2].clone(),
            values[3].clone(),
        ],
        _ => return Vec::new(),
    };
    longhands
        .iter()
        .zip(ordered)
        .map(|(name, value)| Declaration {
            name: name.to_string(),
            value,
        })
        .collect()
}

fn parse_value(text: &str) -> Option<Value> {
    let first = text.chars().next()?;
    if text.starts_with("calc(") {
        let linear = calc::parse_calc(text)?;
        if linear.is_px_only() {
            return Some(Value::Length(linear.px));
        }
        return Some(Value::Calc(linear));
    }
    if let Some(rest) = text.strip_prefix("url(") {
        let inner = rest.strip_suffix(')')?.trim();
        let url = unquote(inner)?;
        if url.is_empty() {
            return None;
        }
        return Some(Value::Url(url));
    }
    if text.starts_with("light-dark(") {
        return parse_light_dark(text);
    }
    if first == '#' {
        return parse_hex_color(&text[1..]).map(Value::Color);
    }
    if first.is_ascii_digit() || first == '.' || first == '-' {
        return parse_dimension(text);
    }
    if text.chars().all(is_ident_char) {
        let keyword = text.to_ascii_lowercase();
        for (name, color) in NAMED_COLORS {
            if keyword == name {
                return Some(Value::Color(color));
            }
        }
        return Some(Value::Keyword(keyword));
    }
    None
}

fn parse_number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|number| number.is_finite())
}

fn parse_dimension(text: &str) -> Option<Value> {
    if let Some(number) = text.strip_suffix("px") {
        return parse_number(number).map(Value::Length);
    }
    if let Some(number) = text.strip_suffix("rem") {
        return parse_number(number).map(Value::Rem);
    }
    if let Some(number) = text.strip_suffix("em") {
        return parse_number(number).map(Value::Em);
    }
    if let Some(number) = text.strip_suffix("vmin") {
        return parse_number(number).map(Value::Vmin);
    }
    if let Some(number) = text.strip_suffix("vmax") {
        return parse_number(number).map(Value::Vmax);
    }
    if let Some(number) = text.strip_suffix("vw") {
        return parse_number(number).map(Value::Vw);
    }
    if let Some(number) = text.strip_suffix("vh") {
        return parse_number(number).map(Value::Vh);
    }
    if let Some(number) = text.strip_suffix('%') {
        return parse_number(number).map(Value::Percent);
    }
    match parse_number(text)? {
        0.0 => Some(Value::Length(0.0)),
        number => Some(Value::Number(number)),
    }
}

fn parse_hex_color(hex: &str) -> Option<Color> {
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match hex.len() {
        6 => Some(Color {
            r: u8::from_str_radix(&hex[0..2], 16).ok()?,
            g: u8::from_str_radix(&hex[2..4], 16).ok()?,
            b: u8::from_str_radix(&hex[4..6], 16).ok()?,
        }),
        3 => Some(Color {
            r: u8::from_str_radix(&hex[0..1], 16).ok()? * 17,
            g: u8::from_str_radix(&hex[1..2], 16).ok()? * 17,
            b: u8::from_str_radix(&hex[2..3], 16).ok()? * 17,
        }),
        _ => None,
    }
}
