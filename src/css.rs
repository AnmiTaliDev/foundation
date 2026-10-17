// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
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
    Color(Color),
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

pub fn parse(source: &str) -> Stylesheet {
    let source = strip_comments(source);
    let mut rules = Vec::new();
    let mut rest = source.as_str();
    while let Some(open) = rest.find('{') {
        let selector_text = &rest[..open];
        let after = &rest[open + 1..];
        let (block, remaining) = match after.find('}') {
            Some(close) => (&after[..close], &after[close + 1..]),
            None => (after, ""),
        };
        if let Some(rule) = parse_rule(selector_text, block) {
            rules.push(rule);
        }
        rest = remaining;
    }
    Stylesheet { rules }
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

fn expand_border(text: &str) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    for token in text.split_whitespace() {
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
    for token in text.split_whitespace() {
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

fn parse_dimension(text: &str) -> Option<Value> {
    if let Some(number) = text.strip_suffix("px") {
        return number.parse().ok().map(Value::Length);
    }
    if let Some(number) = text.strip_suffix("em") {
        return number.parse().ok().map(Value::Em);
    }
    if let Some(number) = text.strip_suffix('%') {
        return number.parse().ok().map(Value::Percent);
    }
    match text.parse::<f32>() {
        Ok(0.0) => Some(Value::Length(0.0)),
        _ => None,
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
