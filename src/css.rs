// SPDX-License-Identifier: GPL-3.0-only

use crate::calc::{self, Linear};
use crate::color::{self, Rgba};
use crate::decoration::{self, Decoration};
use crate::generated;
use crate::gradient::{self, Gradient};
use crate::gridspec::{self, Line, TrackList};
use crate::media::MediaList;
use crate::selector;
use crate::shadow::{self, Shadow};
use crate::supports;
use crate::transform::{self, Function};

pub use crate::selector::{Combinator, Pseudo, PseudoElement, Selector, Specificity};

#[derive(Debug, Clone, PartialEq)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
    pub layers: Vec<String>,
    pub font_faces: Vec<FontFace>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FontFace {
    pub family: String,
    pub bold: bool,
    pub italic: bool,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
    pub important: Vec<Declaration>,
    pub media: Vec<MediaList>,
    pub layer: Option<String>,
}

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
    Families(Vec<String>),
    Transform(Vec<Function>),
    Rgba(Color, u8),
    Gradient(Gradient),
    Tracks(TrackList),
    GridLine(Line),
    Areas(Vec<Vec<String>>),
    Shadows(Vec<Shadow>),
    Decorations(Vec<Decoration>),
    Content(Vec<generated::Part>),
    Counters(Vec<(String, i32)>),
    Quotes(Vec<(String, String)>),
    List(Vec<Value>),
}

impl Value {
    pub fn urls(&self) -> Vec<&String> {
        match self {
            Value::Url(url) => vec![url],
            Value::List(items) => items.iter().flat_map(Value::urls).collect(),
            _ => Vec::new(),
        }
    }

    pub fn urls_mut(&mut self) -> Vec<&mut String> {
        match self {
            Value::Url(url) => vec![url],
            Value::List(items) => items.iter_mut().flat_map(Value::urls_mut).collect(),
            _ => Vec::new(),
        }
    }
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

const BORDER_STYLE_PROPERTIES: [&str; 4] = [
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
];

const BORDER_COLOR_PROPERTIES: [&str; 4] = [
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
];

const BOX_SHORTHANDS: [(&str, [&str; 4]); 5] = [
    ("margin", MARGIN_PROPERTIES),
    ("padding", PADDING_PROPERTIES),
    ("border-width", BORDER_WIDTH_PROPERTIES),
    ("border-style", BORDER_STYLE_PROPERTIES),
    ("border-color", BORDER_COLOR_PROPERTIES),
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

const NAMED_COLORS: [(&str, u32); 148] = [
    ("aliceblue", 0xF0F8FF),
    ("antiquewhite", 0xFAEBD7),
    ("aqua", 0x00FFFF),
    ("aquamarine", 0x7FFFD4),
    ("azure", 0xF0FFFF),
    ("beige", 0xF5F5DC),
    ("bisque", 0xFFE4C4),
    ("black", 0x000000),
    ("blanchedalmond", 0xFFEBCD),
    ("blue", 0x0000FF),
    ("blueviolet", 0x8A2BE2),
    ("brown", 0xA52A2A),
    ("burlywood", 0xDEB887),
    ("cadetblue", 0x5F9EA0),
    ("chartreuse", 0x7FFF00),
    ("chocolate", 0xD2691E),
    ("coral", 0xFF7F50),
    ("cornflowerblue", 0x6495ED),
    ("cornsilk", 0xFFF8DC),
    ("crimson", 0xDC143C),
    ("cyan", 0x00FFFF),
    ("darkblue", 0x00008B),
    ("darkcyan", 0x008B8B),
    ("darkgoldenrod", 0xB8860B),
    ("darkgray", 0xA9A9A9),
    ("darkgreen", 0x006400),
    ("darkgrey", 0xA9A9A9),
    ("darkkhaki", 0xBDB76B),
    ("darkmagenta", 0x8B008B),
    ("darkolivegreen", 0x556B2F),
    ("darkorange", 0xFF8C00),
    ("darkorchid", 0x9932CC),
    ("darkred", 0x8B0000),
    ("darksalmon", 0xE9967A),
    ("darkseagreen", 0x8FBC8F),
    ("darkslateblue", 0x483D8B),
    ("darkslategray", 0x2F4F4F),
    ("darkslategrey", 0x2F4F4F),
    ("darkturquoise", 0x00CED1),
    ("darkviolet", 0x9400D3),
    ("deeppink", 0xFF1493),
    ("deepskyblue", 0x00BFFF),
    ("dimgray", 0x696969),
    ("dimgrey", 0x696969),
    ("dodgerblue", 0x1E90FF),
    ("firebrick", 0xB22222),
    ("floralwhite", 0xFFFAF0),
    ("forestgreen", 0x228B22),
    ("fuchsia", 0xFF00FF),
    ("gainsboro", 0xDCDCDC),
    ("ghostwhite", 0xF8F8FF),
    ("gold", 0xFFD700),
    ("goldenrod", 0xDAA520),
    ("gray", 0x808080),
    ("green", 0x008000),
    ("greenyellow", 0xADFF2F),
    ("grey", 0x808080),
    ("honeydew", 0xF0FFF0),
    ("hotpink", 0xFF69B4),
    ("indianred", 0xCD5C5C),
    ("indigo", 0x4B0082),
    ("ivory", 0xFFFFF0),
    ("khaki", 0xF0E68C),
    ("lavender", 0xE6E6FA),
    ("lavenderblush", 0xFFF0F5),
    ("lawngreen", 0x7CFC00),
    ("lemonchiffon", 0xFFFACD),
    ("lightblue", 0xADD8E6),
    ("lightcoral", 0xF08080),
    ("lightcyan", 0xE0FFFF),
    ("lightgoldenrodyellow", 0xFAFAD2),
    ("lightgray", 0xD3D3D3),
    ("lightgreen", 0x90EE90),
    ("lightgrey", 0xD3D3D3),
    ("lightpink", 0xFFB6C1),
    ("lightsalmon", 0xFFA07A),
    ("lightseagreen", 0x20B2AA),
    ("lightskyblue", 0x87CEFA),
    ("lightslategray", 0x778899),
    ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xB0C4DE),
    ("lightyellow", 0xFFFFE0),
    ("lime", 0x00FF00),
    ("limegreen", 0x32CD32),
    ("linen", 0xFAF0E6),
    ("magenta", 0xFF00FF),
    ("maroon", 0x800000),
    ("mediumaquamarine", 0x66CDAA),
    ("mediumblue", 0x0000CD),
    ("mediumorchid", 0xBA55D3),
    ("mediumpurple", 0x9370DB),
    ("mediumseagreen", 0x3CB371),
    ("mediumslateblue", 0x7B68EE),
    ("mediumspringgreen", 0x00FA9A),
    ("mediumturquoise", 0x48D1CC),
    ("mediumvioletred", 0xC71585),
    ("midnightblue", 0x191970),
    ("mintcream", 0xF5FFFA),
    ("mistyrose", 0xFFE4E1),
    ("moccasin", 0xFFE4B5),
    ("navajowhite", 0xFFDEAD),
    ("navy", 0x000080),
    ("oldlace", 0xFDF5E6),
    ("olive", 0x808000),
    ("olivedrab", 0x6B8E23),
    ("orange", 0xFFA500),
    ("orangered", 0xFF4500),
    ("orchid", 0xDA70D6),
    ("palegoldenrod", 0xEEE8AA),
    ("palegreen", 0x98FB98),
    ("paleturquoise", 0xAFEEEE),
    ("palevioletred", 0xDB7093),
    ("papayawhip", 0xFFEFD5),
    ("peachpuff", 0xFFDAB9),
    ("peru", 0xCD853F),
    ("pink", 0xFFC0CB),
    ("plum", 0xDDA0DD),
    ("powderblue", 0xB0E0E6),
    ("purple", 0x800080),
    ("rebeccapurple", 0x663399),
    ("red", 0xFF0000),
    ("rosybrown", 0xBC8F8F),
    ("royalblue", 0x4169E1),
    ("saddlebrown", 0x8B4513),
    ("salmon", 0xFA8072),
    ("sandybrown", 0xF4A460),
    ("seagreen", 0x2E8B57),
    ("seashell", 0xFFF5EE),
    ("sienna", 0xA0522D),
    ("silver", 0xC0C0C0),
    ("skyblue", 0x87CEEB),
    ("slateblue", 0x6A5ACD),
    ("slategray", 0x708090),
    ("slategrey", 0x708090),
    ("snow", 0xFFFAFA),
    ("springgreen", 0x00FF7F),
    ("steelblue", 0x4682B4),
    ("tan", 0xD2B48C),
    ("teal", 0x008080),
    ("thistle", 0xD8BFD8),
    ("tomato", 0xFF6347),
    ("turquoise", 0x40E0D0),
    ("violet", 0xEE82EE),
    ("wheat", 0xF5DEB3),
    ("white", 0xFFFFFF),
    ("whitesmoke", 0xF5F5F5),
    ("yellow", 0xFFFF00),
    ("yellowgreen", 0x9ACD32),
];

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
    let mut escaped = false;
    for (index, c) in source.char_indices() {
        if let Some(open) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == open {
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

const MAX_NESTING: usize = 8;

pub fn parse(source: &str) -> Stylesheet {
    let source = strip_comments(source);
    let mut sheet = Stylesheet {
        rules: Vec::new(),
        layers: Vec::new(),
        font_faces: Vec::new(),
    };
    let scope = Scope {
        media: Vec::new(),
        layer: None,
        depth: 0,
    };
    collect_rules(&source, &scope, &mut sheet);
    sheet
}

struct Scope {
    media: Vec<MediaList>,
    layer: Option<String>,
    depth: usize,
}

impl Scope {
    fn nested(&self) -> Scope {
        Scope {
            media: self.media.clone(),
            layer: self.layer.clone(),
            depth: self.depth + 1,
        }
    }
}

fn layer_path(parent: &Option<String>, name: &str) -> String {
    match parent {
        Some(parent) => format!("{}.{}", parent, name),
        None => name.to_string(),
    }
}

fn register_layer(sheet: &mut Stylesheet, path: String) {
    if !sheet.layers.contains(&path) {
        sheet.layers.push(path);
    }
}

fn collect_rules(source: &str, scope: &Scope, sheet: &mut Stylesheet) {
    for item in scan_top_level(source) {
        let (prelude, body) = match item {
            TopLevel::Block { prelude, body } => (prelude, body),
            TopLevel::Statement(text) => {
                if let Some(names) = at_rule_condition(text, "@layer") {
                    for name in names.split(',').map(str::trim).filter(|n| !n.is_empty()) {
                        register_layer(sheet, layer_path(&scope.layer, name));
                    }
                }
                continue;
            }
        };
        if !prelude.starts_with('@') {
            if let Some(mut rule) = parse_rule(prelude, body) {
                rule.media = scope.media.clone();
                rule.layer = scope.layer.clone();
                sheet.rules.push(rule);
            }
            continue;
        }
        if at_rule_condition(prelude, "@font-face") == Some("") {
            sheet.font_faces.extend(parse_font_face(body));
            continue;
        }
        if scope.depth >= MAX_NESTING {
            continue;
        }
        let mut nested = scope.nested();
        if let Some(condition) = at_rule_condition(prelude, "@media") {
            nested.media.push(MediaList::parse(condition));
            collect_rules(body, &nested, sheet);
        } else if let Some(condition) = at_rule_condition(prelude, "@supports") {
            if supports::evaluate(condition) {
                collect_rules(body, &nested, sheet);
            }
        } else if let Some(name) = at_rule_condition(prelude, "@layer") {
            let name = if name.is_empty() {
                format!("(anonymous {})", sheet.layers.len())
            } else {
                name.to_string()
            };
            let path = layer_path(&scope.layer, &name);
            register_layer(sheet, path.clone());
            nested.layer = Some(path);
            collect_rules(body, &nested, sheet);
        }
    }
}

fn parse_font_face(body: &str) -> Option<FontFace> {
    let mut family = None;
    let mut bold = false;
    let mut italic = false;
    let mut sources = Vec::new();
    for item in body.split(';') {
        let Some((name, value)) = item.split_once(':') else {
            continue;
        };
        let value = value.trim();
        match name.trim().to_ascii_lowercase().as_str() {
            "font-family" => family = parse_families(value)?.into_iter().next(),
            "font-weight" => {
                bold = match parse_value(value) {
                    Some(Value::Keyword(keyword)) => keyword == "bold" || keyword == "bolder",
                    Some(Value::Number(weight)) => weight >= 600.0,
                    _ => false,
                }
            }
            "font-style" => {
                italic = matches!(value.to_ascii_lowercase().as_str(), "italic" | "oblique")
            }
            "src" => sources = parse_font_sources(value),
            _ => {}
        }
    }
    if sources.is_empty() {
        return None;
    }
    Some(FontFace {
        family: family?,
        bold,
        italic,
        sources,
    })
}

fn parse_font_sources(text: &str) -> Vec<String> {
    let mut sources = Vec::new();
    for part in text.split(',') {
        let tokens = split_values(part);
        let supported = tokens.iter().all(|token| {
            let lower = token.to_ascii_lowercase();
            match lower.strip_prefix("format(") {
                Some(inner) => {
                    let format = inner
                        .trim_end_matches(')')
                        .trim_matches(|c| c == '"' || c == '\'');
                    matches!(format, "truetype" | "opentype" | "ttf" | "otf")
                }
                None => true,
            }
        });
        if !supported {
            continue;
        }
        for token in tokens {
            if let Some(Value::Url(url)) = parse_value(token) {
                sources.push(url);
            }
        }
    }
    sources
}

fn at_rule_condition<'a>(prelude: &'a str, name: &str) -> Option<&'a str> {
    let head = prelude.get(..name.len())?;
    if !head.eq_ignore_ascii_case(name) {
        return None;
    }
    let rest = &prelude[name.len()..];
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
    let selectors = selector::parse_list(selector_text)?;
    let block = parse_declaration_block(block);
    Some(Rule {
        selectors,
        declarations: block.normal,
        important: block.important,
        media: Vec::new(),
        layer: None,
    })
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct DeclarationBlock {
    pub normal: Vec<Declaration>,
    pub important: Vec<Declaration>,
}

pub fn parse_declaration_block(block: &str) -> DeclarationBlock {
    let mut result = DeclarationBlock::default();
    for item in split_declarations(block) {
        let (item, important) = split_important(item);
        let target = if important {
            &mut result.important
        } else {
            &mut result.normal
        };
        target.extend(parse_declaration(item));
    }
    result
}

pub fn parse_declarations(block: &str) -> Vec<Declaration> {
    let mut block = parse_declaration_block(block);
    block.normal.append(&mut block.important);
    block.normal
}

fn split_important(item: &str) -> (&str, bool) {
    let trimmed = item.trim_end();
    if let Some(position) = trimmed.rfind('!') {
        if trimmed[position + 1..]
            .trim()
            .eq_ignore_ascii_case("important")
        {
            return (&trimmed[..position], true);
        }
    }
    (item, false)
}

fn expand_content(name: String, value: &str) -> Vec<Declaration> {
    let lower = value.to_ascii_lowercase();
    let value = if lower == "none" || lower == "normal" {
        Value::Keyword(lower)
    } else {
        match generated::parse(value) {
            Some(parts) => Value::Content(parts),
            None => return Vec::new(),
        }
    };
    vec![Declaration { name, value }]
}

fn expand_outline(text: &str) -> Vec<Declaration> {
    let mut width = Value::Length(3.0);
    let mut style = Value::Keyword("none".to_string());
    let mut color = Value::Keyword("currentcolor".to_string());
    for token in split_values(text) {
        match parse_value(token) {
            Some(value @ (Value::Color(_) | Value::Rgba(..))) => color = value,
            Some(value @ (Value::Length(_) | Value::Em(_))) => width = value,
            Some(Value::Keyword(keyword)) => match keyword.as_str() {
                "currentcolor" => color = Value::Keyword(keyword),
                "thin" | "medium" | "thick" => width = Value::Keyword(keyword),
                "auto" => style = Value::Keyword(keyword),
                _ if is_line_style(&keyword) => style = Value::Keyword(keyword),
                _ => return Vec::new(),
            },
            _ => return Vec::new(),
        }
    }
    vec![
        declaration("outline-width", width),
        declaration("outline-style", style),
        declaration("outline-color", color),
    ]
}

fn expand_quotes(name: String, value: &str) -> Vec<Declaration> {
    let lower = value.to_ascii_lowercase();
    let value = if lower == "none" || lower == "auto" {
        Value::Keyword(lower)
    } else {
        match generated::parse_strings(value) {
            Some(strings) if !strings.is_empty() && strings.len() % 2 == 0 => {
                let mut pairs = Vec::new();
                let mut iter = strings.into_iter();
                while let (Some(open), Some(close)) = (iter.next(), iter.next()) {
                    pairs.push((open, close));
                }
                Value::Quotes(pairs)
            }
            _ => return Vec::new(),
        }
    };
    vec![Declaration { name, value }]
}

fn expand_counters(name: String, value: &str) -> Vec<Declaration> {
    if value.eq_ignore_ascii_case("none") {
        return vec![Declaration {
            name,
            value: Value::Keyword("none".to_string()),
        }];
    }
    let default = if name == "counter-increment" { 1 } else { 0 };
    let tokens = split_values(value);
    let mut counters = Vec::new();
    let mut index = 0;
    while index < tokens.len() {
        let counter = tokens[index];
        if counter.is_empty()
            || !counter.chars().all(is_ident_char)
            || counter.starts_with(|c: char| c.is_ascii_digit())
            || counter.eq_ignore_ascii_case("none")
        {
            return Vec::new();
        }
        let number = tokens
            .get(index + 1)
            .and_then(|token| token.parse::<i32>().ok());
        counters.push((counter.to_string(), number.unwrap_or(default)));
        index += if number.is_some() { 2 } else { 1 };
    }
    if counters.is_empty() {
        return Vec::new();
    }
    vec![Declaration {
        name,
        value: Value::Counters(counters),
    }]
}

fn split_declarations(block: &str) -> Vec<&str> {
    let mut items = Vec::new();
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let mut start = 0;
    for (index, c) in block.char_indices() {
        if escaped {
            escaped = false;
        } else if c == '\\' {
            escaped = true;
        } else if let Some(open) = quote {
            if c == open {
                quote = None;
            }
        } else if c == '"' || c == '\'' {
            quote = Some(c);
        } else if c == ';' {
            items.push(&block[start..index]);
            start = index + 1;
        }
    }
    items.push(&block[start..]);
    items
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

pub(crate) fn parse_declaration(item: &str) -> Vec<Declaration> {
    let Some((name, value)) = item.split_once(':') else {
        return Vec::new();
    };
    let name = match name.trim().to_ascii_lowercase().as_str() {
        "grid-gap" => "gap".to_string(),
        "grid-row-gap" => "row-gap".to_string(),
        "grid-column-gap" => "column-gap".to_string(),
        "word-wrap" => "overflow-wrap".to_string(),
        other => other.to_string(),
    };
    if name.is_empty() {
        return Vec::new();
    }
    let value = value.trim();

    if name == "content" {
        return expand_content(name, value);
    }
    if name == "outline" {
        return expand_outline(value);
    }
    if name == "quotes" {
        return expand_quotes(name, value);
    }
    if matches!(
        name.as_str(),
        "counter-reset" | "counter-increment" | "counter-set"
    ) {
        return expand_counters(name, value);
    }
    if name == "border" {
        return expand_border(value, &[0, 1, 2, 3]);
    }
    for (side, shorthand) in ["border-top", "border-right", "border-bottom", "border-left"]
        .iter()
        .enumerate()
    {
        if name == *shorthand {
            return expand_border(value, &[side]);
        }
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
    if let Some(declarations) = expand_text_decoration(&name, value) {
        return declarations;
    }
    if let Some(declarations) = expand_radius(&name, value) {
        return declarations;
    }
    if name == "box-shadow" {
        return match shadow::parse(value) {
            Some(shadows) => vec![Declaration {
                name,
                value: Value::Shadows(shadows),
            }],
            None => parse_value(value)
                .filter(|value| matches!(value, Value::Keyword(keyword) if keyword == "none"))
                .map(|value| vec![Declaration { name, value }])
                .unwrap_or_default(),
        };
    }
    if name == "overflow" {
        return expand_overflow(value);
    }
    if LAYERED.contains(&name.as_str()) {
        let parts = gradient::split_top_level(value);
        if parts.len() > 1 {
            let mut layers = Vec::new();
            for part in parts {
                let declarations = parse_declaration(&format!("{}: {}", name, part));
                if declarations.is_empty() {
                    return Vec::new();
                }
                layers.push(declarations);
            }
            return merge_layers(layers);
        }
    }
    if name == "background" {
        let parts = gradient::split_top_level(value);
        if parts.len() == 1 {
            return expand_background(parts[0]);
        }
        let layers: Vec<Vec<Declaration>> = parts
            .iter()
            .map(|part| {
                let mut declarations = expand_background(part);
                declarations.retain(|d| d.name != "background-color");
                if !declarations.iter().any(|d| d.name == "background-image") {
                    declarations.push(declaration("background-image", none()));
                }
                declarations
            })
            .collect();
        let color = expand_background(parts[parts.len() - 1])
            .into_iter()
            .filter(|d| d.name == "background-color");
        let mut declarations = merge_layers(layers);
        declarations.extend(color);
        return declarations;
    }
    if name == "background-image" {
        return match parse_value(gradient::split_top_level(value)[0]) {
            Some(value @ (Value::Url(_) | Value::Gradient(_) | Value::Keyword(_))) => {
                vec![Declaration { name, value }]
            }
            _ => Vec::new(),
        };
    }
    if name == "font-family" {
        return match parse_families(value) {
            Some(families) => vec![Declaration {
                name,
                value: Value::Families(families),
            }],
            None => Vec::new(),
        };
    }
    if name == "transform" {
        return match transform::parse(value) {
            Some(functions) => vec![Declaration {
                name,
                value: Value::Transform(functions),
            }],
            None => parse_value(value)
                .map(|value| vec![Declaration { name, value }])
                .unwrap_or_default(),
        };
    }
    if let Some(declarations) = expand_grid(&name, value) {
        return declarations;
    }
    if name == "border-spacing" {
        let lengths: Vec<Value> = split_values(value)
            .into_iter()
            .filter_map(|token| match parse_value(token) {
                Some(value @ (Value::Length(_) | Value::Em(_) | Value::Rem(_))) => Some(value),
                _ => None,
            })
            .collect();
        return match lengths.as_slice() {
            [one] => vec![
                declaration("border-spacing-x", one.clone()),
                declaration("border-spacing-y", one.clone()),
            ],
            [x, y] => vec![
                declaration("border-spacing-x", x.clone()),
                declaration("border-spacing-y", y.clone()),
            ],
            _ => Vec::new(),
        };
    }
    if name == "list-style" {
        return split_values(value)
            .into_iter()
            .map(str::to_ascii_lowercase)
            .find(|token| crate::list::is_type(token))
            .map(|kind| {
                vec![Declaration {
                    name: "list-style-type".to_string(),
                    value: Value::Keyword(kind),
                }]
            })
            .unwrap_or_default();
    }
    if name == "background-clip" || name == "background-origin" {
        return match parse_value(value) {
            Some(Value::Keyword(keyword)) if is_box_keyword(&keyword) => {
                vec![Declaration {
                    name,
                    value: Value::Keyword(keyword),
                }]
            }
            _ => Vec::new(),
        };
    }
    if name == "object-fit" {
        return match parse_value(value) {
            Some(Value::Keyword(keyword))
                if matches!(
                    keyword.as_str(),
                    "fill" | "contain" | "cover" | "none" | "scale-down"
                ) =>
            {
                vec![Declaration {
                    name,
                    value: Value::Keyword(keyword),
                }]
            }
            _ => Vec::new(),
        };
    }
    if name == "object-position" {
        return expand_position(&split_values(value), "object-position");
    }
    if name == "aspect-ratio" {
        return match parse_ratio(value) {
            Some(value) => vec![Declaration { name, value }],
            None => Vec::new(),
        };
    }
    if name == "background-position" {
        return expand_position(&split_values(value), "background-position");
    }
    if name == "transform-origin" {
        return expand_position(&split_values(value), "transform-origin");
    }
    if name == "background-size" {
        return expand_size(&split_values(value));
    }
    if name == "font-size" {
        if let Some(size) = parse_font_size(value) {
            return vec![Declaration { name, value: size }];
        }
    }

    if (name == "border-style" || BORDER_STYLE_PROPERTIES.contains(&name.as_str()))
        && !split_values(value)
            .iter()
            .all(|token| is_line_style(&token.to_ascii_lowercase()))
    {
        return Vec::new();
    }

    if (name == "border-color" || BORDER_COLOR_PROPERTIES.contains(&name.as_str()))
        && !split_values(value)
            .iter()
            .all(|token| is_border_color(token))
    {
        return Vec::new();
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

pub(crate) fn split_values(text: &str) -> Vec<&str> {
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
    let mut boxes = Vec::new();
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
            Some(Value::Rgba(color, alpha)) => declarations.push(Declaration {
                name: "background-color".to_string(),
                value: Value::Rgba(color, alpha),
            }),
            Some(value @ (Value::Url(_) | Value::Gradient(_))) => declarations.push(Declaration {
                name: "background-image".to_string(),
                value,
            }),
            Some(Value::Keyword(keyword)) if keyword == "currentcolor" => {
                declarations.push(Declaration {
                    name: "background-color".to_string(),
                    value: Value::Keyword(keyword),
                })
            }
            Some(Value::Keyword(keyword)) if is_box_keyword(&keyword) => boxes.push(keyword),
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
    declarations.extend(expand_position(&position, "background-position"));
    declarations.extend(expand_size(&size));
    if let Some(origin) = boxes.first() {
        let clip = boxes.get(1).unwrap_or(origin);
        for (name, keyword) in [("background-origin", origin), ("background-clip", clip)] {
            declarations.push(Declaration {
                name: name.to_string(),
                value: Value::Keyword(keyword.clone()),
            });
        }
    }
    declarations
}

const LAYERED: [&str; 6] = [
    "background-image",
    "background-repeat",
    "background-position",
    "background-size",
    "background-origin",
    "background-clip",
];

fn none() -> Value {
    Value::Keyword("none".to_string())
}

fn layer_default(name: &str) -> Value {
    let keyword = |text: &str| Value::Keyword(text.to_string());
    match name {
        "background-image" => none(),
        "background-repeat" => keyword("repeat"),
        "background-position-x" | "background-position-y" => Value::Percent(0.0),
        "background-size" => keyword("explicit"),
        "background-size-width" | "background-size-height" => keyword("auto"),
        "background-origin" => keyword("padding-box"),
        _ => keyword("border-box"),
    }
}

fn merge_layers(layers: Vec<Vec<Declaration>>) -> Vec<Declaration> {
    let mut names: Vec<String> = Vec::new();
    for declaration in layers.iter().flatten() {
        if !names.contains(&declaration.name) {
            names.push(declaration.name.clone());
        }
    }
    names
        .into_iter()
        .map(|name| {
            let values = layers
                .iter()
                .map(|layer| {
                    layer
                        .iter()
                        .find(|d| d.name == name)
                        .map_or_else(|| layer_default(&name), |d| d.value.clone())
                })
                .collect();
            Declaration {
                name,
                value: Value::List(values),
            }
        })
        .collect()
}

fn is_box_keyword(keyword: &str) -> bool {
    matches!(keyword, "border-box" | "padding-box" | "content-box")
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

fn expand_position(tokens: &[&str], prefix: &str) -> Vec<Declaration> {
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
            name: format!("{}-x", prefix),
            value: x,
        },
        Declaration {
            name: format!("{}-y", prefix),
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

pub(crate) fn parse_families(text: &str) -> Option<Vec<String>> {
    let mut families = Vec::new();
    let mut start = 0;
    let mut quote: Option<char> = None;
    for (index, c) in text
        .char_indices()
        .chain(std::iter::once((text.len(), ',')))
    {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            ',' => {
                let name = text[start..index].trim();
                if name.is_empty() {
                    return None;
                }
                families.push(unquote(name)?);
                start = index + 1;
            }
            _ => {}
        }
    }
    Some(families)
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
        let mut family_start = index + 1;
        if line_height_text.is_none() && tokens.get(index + 1) == Some(&"/") {
            line_height_text = tokens.get(index + 2).copied();
            family_start = index + 3;
        }
        let line_height = match line_height_text {
            Some(line_height) => match parse_value(line_height) {
                Some(value) => value,
                None => return Vec::new(),
            },
            None => Value::Keyword("normal".to_string()),
        };
        let mut declarations = vec![
            Declaration {
                name: "font-size".to_string(),
                value: size,
            },
            Declaration {
                name: "line-height".to_string(),
                value: line_height,
            },
        ];
        for before in &tokens[..index] {
            match before.to_ascii_lowercase().as_str() {
                "italic" | "oblique" => declarations.push(Declaration {
                    name: "font-style".to_string(),
                    value: Value::Keyword("italic".to_string()),
                }),
                "bold" | "bolder" => declarations.push(Declaration {
                    name: "font-weight".to_string(),
                    value: Value::Keyword("bold".to_string()),
                }),
                number => {
                    if let Some(Value::Number(weight)) = parse_value(number) {
                        declarations.push(Declaration {
                            name: "font-weight".to_string(),
                            value: Value::Number(weight),
                        });
                    }
                }
            }
        }
        if let Some(families) = tokens
            .get(family_start..)
            .filter(|rest| !rest.is_empty())
            .and_then(|rest| parse_families(&rest.join(" ")))
        {
            declarations.push(Declaration {
                name: "font-family".to_string(),
                value: Value::Families(families),
            });
        }
        return declarations;
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

fn expand_grid(name: &str, value: &str) -> Option<Vec<Declaration>> {
    let single = |name: &str, value: Value| vec![declaration(name, value)];
    let result = match name {
        "grid-template-columns" | "grid-template-rows" | "grid-auto-columns" | "grid-auto-rows" => {
            match gridspec::parse_tracks(value) {
                Some(tracks) => single(name, Value::Tracks(tracks)),
                None if value.eq_ignore_ascii_case("none") => {
                    single(name, Value::Keyword("none".to_string()))
                }
                None => Vec::new(),
            }
        }
        "grid-template-areas" => match gridspec::parse_areas(value) {
            Some(areas) => single(name, Value::Areas(areas)),
            None => Vec::new(),
        },
        "grid-column-start" | "grid-column-end" | "grid-row-start" | "grid-row-end" => {
            match gridspec::parse_line(value) {
                Some(line) => single(name, Value::GridLine(line)),
                None => Vec::new(),
            }
        }
        "grid-column" | "grid-row" => expand_line_pair(name, value),
        "grid-area" => expand_area(value),
        "grid-auto-flow" => {
            let lower = value.to_ascii_lowercase();
            let words: Vec<&str> = lower.split_whitespace().collect();
            let mut declarations = Vec::new();
            let flow = if words.contains(&"column") {
                "column"
            } else {
                "row"
            };
            if words
                .iter()
                .all(|w| matches!(*w, "row" | "column" | "dense"))
                && !words.is_empty()
            {
                declarations.push(declaration(name, Value::Keyword(flow.to_string())));
                if words.contains(&"dense") {
                    declarations.push(declaration(
                        "grid-auto-dense",
                        Value::Keyword("dense".to_string()),
                    ));
                }
            }
            declarations
        }
        "place-items" | "place-self" | "place-content" => {
            let base = &name["place-".len()..];
            let words: Vec<&str> = value.split_whitespace().collect();
            let (align, justify) = match words.as_slice() {
                [one] => (*one, *one),
                [first, second] => (*first, *second),
                _ => return Some(Vec::new()),
            };
            vec![
                declaration(
                    &format!("align-{}", base),
                    Value::Keyword(align.to_ascii_lowercase()),
                ),
                declaration(
                    &format!("justify-{}", base),
                    Value::Keyword(justify.to_ascii_lowercase()),
                ),
            ]
        }
        _ => return None,
    };
    Some(result)
}

fn expand_line_pair(name: &str, value: &str) -> Vec<Declaration> {
    let parts: Vec<&str> = value.split('/').map(str::trim).collect();
    let (start, end) = match parts.as_slice() {
        [one] => match gridspec::parse_line(one) {
            Some(Line::Area(name)) => (Line::Area(name.clone()), Line::Area(name)),
            Some(line) => (line, Line::Auto),
            None => return Vec::new(),
        },
        [first, second] => match (gridspec::parse_line(first), gridspec::parse_line(second)) {
            (Some(start), Some(end)) => (start, end),
            _ => return Vec::new(),
        },
        _ => return Vec::new(),
    };
    vec![
        declaration(&format!("{}-start", name), Value::GridLine(start)),
        declaration(&format!("{}-end", name), Value::GridLine(end)),
    ]
}

fn expand_area(value: &str) -> Vec<Declaration> {
    let lines: Option<Vec<Line>> = value.split('/').map(gridspec::parse_line).collect();
    let Some(lines) = lines else {
        return Vec::new();
    };
    let get = |index: usize| lines.get(index).cloned().unwrap_or(Line::Auto);
    let (row_start, column_start, row_end, column_end) = match lines.as_slice() {
        [Line::Area(name)] => (
            Line::Area(name.clone()),
            Line::Area(name.clone()),
            Line::Area(name.clone()),
            Line::Area(name.clone()),
        ),
        [_] | [_, _] | [_, _, _] | [_, _, _, _] => (get(0), get(1), get(2), get(3)),
        _ => return Vec::new(),
    };
    vec![
        declaration("grid-row-start", Value::GridLine(row_start)),
        declaration("grid-column-start", Value::GridLine(column_start)),
        declaration("grid-row-end", Value::GridLine(row_end)),
        declaration("grid-column-end", Value::GridLine(column_end)),
    ]
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

fn decoration_color(text: &str) -> Option<Value> {
    match parse_value(text) {
        Some(value @ (Value::Color(_) | Value::Rgba(..))) => Some(value),
        Some(Value::Keyword(keyword)) if keyword == "currentcolor" => Some(Value::Keyword(keyword)),
        _ => None,
    }
}

fn decoration_thickness(text: &str) -> Option<Value> {
    match parse_value(text) {
        Some(value @ (Value::Length(_) | Value::Em(_))) => Some(value),
        Some(Value::Keyword(keyword)) if keyword == "auto" || keyword == "from-font" => {
            Some(Value::Keyword("auto".to_string()))
        }
        _ => None,
    }
}

fn expand_text_decoration(name: &str, value: &str) -> Option<Vec<Declaration>> {
    match name {
        "text-decoration-line" => {
            let lowered = value.to_ascii_lowercase();
            Some(match decoration::parse_lines(&lowered) {
                Some(_) => vec![declaration(name, Value::Keyword(lowered))],
                None => Vec::new(),
            })
        }
        "text-decoration-style" => {
            let lowered = value.to_ascii_lowercase();
            Some(match decoration::parse_style(&lowered) {
                Some(_) => vec![declaration(name, Value::Keyword(lowered))],
                None => Vec::new(),
            })
        }
        "text-decoration-color" => Some(
            decoration_color(value)
                .map(|value| vec![declaration(name, value)])
                .unwrap_or_default(),
        ),
        "text-decoration-thickness" => Some(
            decoration_thickness(value)
                .map(|value| vec![declaration(name, value)])
                .unwrap_or_default(),
        ),
        "text-decoration" => {
            let mut lines = Vec::new();
            let mut style = "solid".to_string();
            let mut color = Value::Keyword("currentcolor".to_string());
            let mut thickness = Value::Keyword("auto".to_string());
            for token in split_values(value) {
                let lowered = token.to_ascii_lowercase();
                if matches!(lowered.as_str(), "underline" | "overline" | "line-through") {
                    lines.push(lowered);
                } else if lowered == "none" {
                    lines.clear();
                } else if decoration::parse_style(&lowered).is_some() {
                    style = lowered;
                } else if let Some(parsed) = decoration_color(token) {
                    color = parsed;
                } else if let Some(parsed) = decoration_thickness(token) {
                    thickness = parsed;
                } else {
                    return Some(Vec::new());
                }
            }
            let line = if lines.is_empty() {
                "none".to_string()
            } else {
                lines.join(" ")
            };
            Some(vec![
                declaration("text-decoration-line", Value::Keyword(line)),
                declaration("text-decoration-style", Value::Keyword(style)),
                declaration("text-decoration-color", color),
                declaration("text-decoration-thickness", thickness),
            ])
        }
        _ => None,
    }
}

const CORNERS: [&str; 4] = [
    "border-top-left-radius",
    "border-top-right-radius",
    "border-bottom-right-radius",
    "border-bottom-left-radius",
];

fn radius_values(text: &str) -> Option<Vec<Value>> {
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
            _ => return None,
        }
    }
    if values.is_empty() || values.len() > 4 {
        return None;
    }
    Some(values)
}

fn corner_values(values: &[Value]) -> [Value; 4] {
    let pick = |index: usize| values[index].clone();
    match values.len() {
        1 => [pick(0), pick(0), pick(0), pick(0)],
        2 => [pick(0), pick(1), pick(0), pick(1)],
        3 => [pick(0), pick(1), pick(2), pick(1)],
        _ => [pick(0), pick(1), pick(2), pick(3)],
    }
}

fn expand_radius(name: &str, value: &str) -> Option<Vec<Declaration>> {
    let mut declarations = Vec::new();
    if name == "border-radius" {
        let (horizontal, vertical) = match value.split_once('/') {
            Some((horizontal, vertical)) => (horizontal, vertical),
            None => (value, value),
        };
        let (Some(horizontal), Some(vertical)) =
            (radius_values(horizontal), radius_values(vertical))
        else {
            return Some(Vec::new());
        };
        let horizontal = corner_values(&horizontal);
        let vertical = corner_values(&vertical);
        for (index, corner) in CORNERS.iter().enumerate() {
            declarations.push(declaration(
                &format!("{}-x", corner),
                horizontal[index].clone(),
            ));
            declarations.push(declaration(
                &format!("{}-y", corner),
                vertical[index].clone(),
            ));
        }
        return Some(declarations);
    }
    if CORNERS.contains(&name) {
        let values = match radius_values(value) {
            Some(values) if values.len() <= 2 => values,
            _ => return Some(Vec::new()),
        };
        let vertical = values.get(1).unwrap_or(&values[0]).clone();
        declarations.push(declaration(&format!("{}-x", name), values[0].clone()));
        declarations.push(declaration(&format!("{}-y", name), vertical));
        return Some(declarations);
    }
    None
}

fn expand_overflow(text: &str) -> Vec<Declaration> {
    let mut keywords = Vec::new();
    for token in split_values(text) {
        match parse_value(token) {
            Some(Value::Keyword(keyword))
                if matches!(
                    keyword.as_str(),
                    "visible" | "hidden" | "clip" | "scroll" | "auto"
                ) =>
            {
                keywords.push(keyword)
            }
            _ => return Vec::new(),
        }
    }
    match keywords.len() {
        1 => vec![
            declaration("overflow-x", Value::Keyword(keywords[0].clone())),
            declaration("overflow-y", Value::Keyword(keywords[0].clone())),
        ],
        2 => vec![
            declaration("overflow-x", Value::Keyword(keywords[0].clone())),
            declaration("overflow-y", Value::Keyword(keywords[1].clone())),
        ],
        _ => Vec::new(),
    }
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

fn expand_border(text: &str, sides: &[usize]) -> Vec<Declaration> {
    let mut declarations = Vec::new();
    let mut push = |names: &[&str; 4], value: Value| {
        for side in sides {
            declarations.push(declaration(names[*side], value.clone()));
        }
    };
    for token in split_values(text) {
        match parse_value(token) {
            Some(color @ (Value::Color(_) | Value::Rgba(..))) => {
                push(&BORDER_COLOR_PROPERTIES, color)
            }
            Some(width @ (Value::Length(_) | Value::Em(_))) => {
                push(&BORDER_WIDTH_PROPERTIES, width)
            }
            Some(Value::Keyword(keyword)) if keyword == "none" || keyword == "hidden" => {
                push(&BORDER_WIDTH_PROPERTIES, Value::Length(0.0));
                push(&BORDER_STYLE_PROPERTIES, Value::Keyword(keyword));
            }
            Some(Value::Keyword(keyword)) if is_line_style(&keyword) => {
                push(&BORDER_STYLE_PROPERTIES, Value::Keyword(keyword))
            }
            Some(Value::Keyword(keyword)) if keyword == "currentcolor" => {
                push(&BORDER_COLOR_PROPERTIES, Value::Keyword(keyword))
            }
            Some(Value::Keyword(_)) => {}
            _ => return Vec::new(),
        }
    }
    declarations
}

fn is_border_color(token: &str) -> bool {
    matches!(
        parse_value(token),
        Some(Value::Color(_) | Value::Rgba(..)) | Some(Value::Keyword(_))
    ) && !is_line_style(&token.to_ascii_lowercase())
        || token.eq_ignore_ascii_case("currentcolor")
}

fn parse_ratio(text: &str) -> Option<Value> {
    let lower = text.trim().to_ascii_lowercase();
    if lower == "auto" {
        return Some(Value::Keyword(lower));
    }
    let ratio = lower.strip_prefix("auto").map_or(lower.as_str(), str::trim);
    let (first, second) = match ratio.split_once('/') {
        Some((first, second)) => (first.trim(), second.trim()),
        None => (ratio, "1"),
    };
    let first: f32 = first.parse().ok()?;
    let second: f32 = second.parse().ok()?;
    if first <= 0.0 || second <= 0.0 || !first.is_finite() || !second.is_finite() {
        return None;
    }
    Some(Value::Number(first / second))
}

fn is_line_style(keyword: &str) -> bool {
    matches!(
        keyword,
        "none"
            | "hidden"
            | "solid"
            | "dashed"
            | "dotted"
            | "double"
            | "groove"
            | "ridge"
            | "inset"
            | "outset"
    )
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
    if first == '#' || is_color_function(text) {
        return color::parse(text).map(color_value);
    }
    if is_gradient_function(text) {
        return gradient::parse(text).map(Value::Gradient);
    }
    if first.is_ascii_digit() || first == '.' || first == '-' {
        return parse_dimension(text);
    }
    if text.chars().all(is_ident_char) {
        let keyword = text.to_ascii_lowercase();
        if let Some(rgba) = color::parse(&keyword) {
            return Some(color_value(rgba));
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

fn color_value(rgba: Rgba) -> Value {
    if rgba.alpha == 255 {
        Value::Color(rgba.color)
    } else {
        Value::Rgba(rgba.color, rgba.alpha)
    }
}

fn is_color_function(text: &str) -> bool {
    let lower = text.get(..5).unwrap_or(text).to_ascii_lowercase();
    ["rgb(", "rgba(", "hsl(", "hsla("]
        .iter()
        .any(|name| lower.starts_with(name))
}

fn is_gradient_function(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    [
        "linear-gradient(",
        "radial-gradient(",
        "repeating-linear-gradient(",
        "repeating-radial-gradient(",
    ]
    .iter()
    .any(|name| lower.starts_with(name))
}

pub(crate) fn named_color(name: &str) -> Option<Color> {
    let (_, rgb) = NAMED_COLORS.iter().find(|(known, _)| *known == name)?;
    Some(Color {
        r: (rgb >> 16) as u8,
        g: (rgb >> 8) as u8,
        b: *rgb as u8,
    })
}
