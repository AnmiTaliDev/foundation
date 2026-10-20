// SPDX-License-Identifier: GPL-3.0-only

use crate::dom::{ElementData, Node, NodeType};

pub type Specificity = (usize, usize, usize);

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub classes: Vec<String>,
    pub attributes: Vec<Attribute>,
    pub pseudo: Vec<Pseudo>,
    pub pseudo_element: Option<PseudoElement>,
    pub parent: Option<(Combinator, Box<Selector>)>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PseudoElement {
    Before,
    After,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Combinator {
    Descendant,
    Child,
    Adjacent,
    Sibling,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub name: String,
    pub test: Option<(AttributeOp, String)>,
    pub insensitive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum AttributeOp {
    Equals,
    Includes,
    Dash,
    Prefix,
    Suffix,
    Substring,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Pseudo {
    Link,
    Never,
    Root,
    Empty,
    Nth {
        a: i32,
        b: i32,
        last: bool,
        of_type: bool,
    },
    Not(Vec<Selector>),
    Is(Vec<Selector>),
    Where(Vec<Selector>),
}

#[derive(Debug, Clone, Copy)]
pub struct Scope<'a> {
    pub element: &'a ElementData,
    pub siblings: &'a [Node],
    pub position: usize,
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        let mut specificity = (
            usize::from(self.id.is_some()),
            self.classes.len() + self.attributes.len(),
            usize::from(self.tag_name.is_some()) + usize::from(self.pseudo_element.is_some()),
        );
        for pseudo in &self.pseudo {
            match pseudo {
                Pseudo::Where(_) => {}
                Pseudo::Not(list) | Pseudo::Is(list) => {
                    let widest = list.iter().map(Selector::specificity).max();
                    let (ids, classes, tags) = widest.unwrap_or_default();
                    specificity.0 += ids;
                    specificity.1 += classes;
                    specificity.2 += tags;
                }
                _ => specificity.1 += 1,
            }
        }
        if let Some((_, parent)) = &self.parent {
            let (ids, classes, tags) = parent.specificity();
            specificity.0 += ids;
            specificity.1 += classes;
            specificity.2 += tags;
        }
        specificity
    }
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

fn take_ident(text: &str) -> &str {
    let rest = text.trim_start_matches(is_ident_char);
    &text[..text.len() - rest.len()]
}

fn split_top_level<'a>(text: &'a str, separators: &[char]) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut start = 0;
    for (index, c) in text.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' => depth += 1,
            ')' | ']' => depth = depth.saturating_sub(1),
            _ if depth == 0 && separators.contains(&c) => {
                parts.push(&text[start..index]);
                start = index + c.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&text[start..]);
    parts
}

pub fn parse_list(text: &str) -> Option<Vec<Selector>> {
    split_top_level(text, &[','])
        .into_iter()
        .map(|part| parse_selector(part.trim()))
        .collect()
}

fn parse_selector(text: &str) -> Option<Selector> {
    let mut result: Option<Selector> = None;
    let mut combinator: Option<Combinator> = None;
    let mut rest = text;
    loop {
        let trimmed = rest.trim_start();
        let spaced = trimmed.len() != rest.len();
        rest = trimmed;
        if rest.is_empty() {
            break;
        }
        let first = rest.chars().next()?;
        let symbol = match first {
            '>' => Some(Combinator::Child),
            '+' => Some(Combinator::Adjacent),
            '~' => Some(Combinator::Sibling),
            _ => None,
        };
        if let Some(kind) = symbol {
            if result.is_none() || combinator.is_some() {
                return None;
            }
            combinator = Some(kind);
            rest = &rest[1..];
            continue;
        }
        if spaced && result.is_some() && combinator.is_none() {
            combinator = Some(Combinator::Descendant);
        }
        let end = split_top_level(rest, &[' ', '\t', '\n', '\r', '>', '+', '~'])[0].len();
        let mut compound = parse_compound(&rest[..end])?;
        rest = &rest[end..];
        if let Some(left) = result {
            if left.pseudo_element.is_some() {
                return None;
            }
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
        attributes: Vec::new(),
        pseudo: Vec::new(),
        pseudo_element: None,
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
        } else if let Some(after) = rest.strip_prefix('.') {
            let class = take_ident(after);
            if class.is_empty() {
                return None;
            }
            selector.classes.push(class.to_string());
            rest = &after[class.len()..];
        } else if let Some(after) = rest.strip_prefix('[') {
            let close = closing(after, ']')?;
            selector.attributes.push(parse_attribute(&after[..close])?);
            rest = &after[close + 1..];
        } else if let Some(after) = rest.strip_prefix("::") {
            let name = take_ident(after);
            selector.pseudo_element = Some(parse_pseudo_element(name)?);
            rest = &after[name.len()..];
        } else {
            let after = rest.strip_prefix(':')?;
            let name = take_ident(after);
            let after_name = &after[name.len()..];
            if matches!(name.to_ascii_lowercase().as_str(), "before" | "after") {
                selector.pseudo_element = Some(parse_pseudo_element(name)?);
                rest = after_name;
                continue;
            }
            let (argument, remaining) = if let Some(inner) = after_name.strip_prefix('(') {
                let close = closing(inner, ')')?;
                (Some(&inner[..close]), &inner[close + 1..])
            } else {
                (None, after_name)
            };
            selector
                .pseudo
                .extend(parse_pseudo(&name.to_ascii_lowercase(), argument)?);
            rest = remaining;
        }
    }
    Some(selector)
}

fn parse_pseudo_element(name: &str) -> Option<PseudoElement> {
    match name.to_ascii_lowercase().as_str() {
        "before" => Some(PseudoElement::Before),
        "after" => Some(PseudoElement::After),
        _ => None,
    }
}

fn parse_nested(text: &str) -> Option<Vec<Selector>> {
    let list = parse_list(text)?;
    if list
        .iter()
        .any(|selector| selector.pseudo_element.is_some())
    {
        return None;
    }
    Some(list)
}

fn closing(text: &str, close: char) -> Option<usize> {
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    for (index, c) in text.char_indices() {
        if let Some(open) = quote {
            if c == open {
                quote = None;
            }
            continue;
        }
        match c {
            '"' | '\'' => quote = Some(c),
            '(' | '[' => depth += 1,
            ')' | ']' if depth > 0 => depth -= 1,
            _ if c == close => return Some(index),
            _ => {}
        }
    }
    None
}

fn parse_attribute(text: &str) -> Option<Attribute> {
    let text = text.trim();
    let name_end = text
        .find(|c: char| !is_ident_char(c) && c != ':')
        .unwrap_or(text.len());
    let name = text[..name_end].to_ascii_lowercase();
    if name.is_empty() {
        return None;
    }
    let rest = text[name_end..].trim_start();
    if rest.is_empty() {
        return Some(Attribute {
            name,
            test: None,
            insensitive: false,
        });
    }
    let (op, after) = if let Some(after) = rest.strip_prefix('=') {
        (AttributeOp::Equals, after)
    } else {
        let op = match rest.chars().next()? {
            '~' => AttributeOp::Includes,
            '|' => AttributeOp::Dash,
            '^' => AttributeOp::Prefix,
            '$' => AttributeOp::Suffix,
            '*' => AttributeOp::Substring,
            _ => return None,
        };
        (op, rest[1..].strip_prefix('=')?)
    };
    let after = after.trim();
    let (value, flag) = match after.chars().next()? {
        quote @ ('"' | '\'') => {
            let inner = &after[1..];
            let close = inner.find(quote)?;
            (&inner[..close], inner[close + 1..].trim())
        }
        _ => match after.split_once(char::is_whitespace) {
            Some((value, flag)) => (value, flag.trim()),
            None => (after, ""),
        },
    };
    let insensitive = match flag {
        "" | "s" | "S" => false,
        "i" | "I" => true,
        _ => return None,
    };
    Some(Attribute {
        name,
        test: Some((op, value.to_string())),
        insensitive,
    })
}

fn parse_pseudo(name: &str, argument: Option<&str>) -> Option<Vec<Pseudo>> {
    let nth = |a, b, last, of_type| Pseudo::Nth {
        a,
        b,
        last,
        of_type,
    };
    let pseudo = match (name, argument) {
        ("link" | "any-link", None) => Pseudo::Link,
        (
            "visited" | "hover" | "active" | "focus" | "focus-within" | "focus-visible" | "target",
            None,
        ) => Pseudo::Never,
        ("root", None) => Pseudo::Root,
        ("empty", None) => Pseudo::Empty,
        ("first-child", None) => nth(0, 1, false, false),
        ("last-child", None) => nth(0, 1, true, false),
        ("first-of-type", None) => nth(0, 1, false, true),
        ("last-of-type", None) => nth(0, 1, true, true),
        ("only-child", None) => {
            return Some(vec![nth(0, 1, false, false), nth(0, 1, true, false)]);
        }
        ("only-of-type", None) => {
            return Some(vec![nth(0, 1, false, true), nth(0, 1, true, true)]);
        }
        ("nth-child" | "nth-last-child" | "nth-of-type" | "nth-last-of-type", Some(text)) => {
            let (a, b) = parse_nth(text)?;
            nth(a, b, name.contains("last"), name.ends_with("type"))
        }
        ("not", Some(text)) => Pseudo::Not(parse_nested(text)?),
        ("is", Some(text)) => Pseudo::Is(parse_nested(text)?),
        ("where", Some(text)) => Pseudo::Where(parse_nested(text)?),
        _ => return None,
    };
    Some(vec![pseudo])
}

fn parse_nth(text: &str) -> Option<(i32, i32)> {
    let text: String = text
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect::<String>()
        .to_ascii_lowercase();
    match text.as_str() {
        "odd" => return Some((2, 1)),
        "even" => return Some((2, 0)),
        _ => {}
    }
    let Some((a_text, b_text)) = text.split_once('n') else {
        return Some((0, text.parse().ok()?));
    };
    let a = match a_text {
        "" | "+" => 1,
        "-" => -1,
        _ => a_text.parse().ok()?,
    };
    let b = if b_text.is_empty() {
        0
    } else {
        b_text.parse().ok()?
    };
    Some((a, b))
}

fn nth_matches(a: i32, b: i32, position: i32) -> bool {
    if a == 0 {
        return position == b;
    }
    let distance = position - b;
    distance % a == 0 && distance / a >= 0
}

fn is_element(node: &Node) -> Option<&ElementData> {
    match &node.node_type {
        NodeType::Element(element) => Some(element),
        NodeType::Text(_) => None,
    }
}

fn previous_sibling<'a>(scope: Scope<'a>, before: usize) -> Option<Scope<'a>> {
    (0..before).rev().find_map(|position| {
        is_element(&scope.siblings[position]).map(|element| Scope {
            element,
            siblings: scope.siblings,
            position,
        })
    })
}

fn sibling_position(scope: Scope, last: bool, of_type: bool) -> i32 {
    let range = if last {
        scope.position + 1..scope.siblings.len()
    } else {
        0..scope.position
    };
    let count = scope.siblings[range]
        .iter()
        .filter_map(is_element)
        .filter(|element| !of_type || element.tag_name == scope.element.tag_name)
        .count();
    count as i32 + 1
}

pub fn matches(
    scope: Scope,
    ancestors: &[Scope],
    selector: &Selector,
    pseudo_element: Option<PseudoElement>,
) -> bool {
    selector.pseudo_element == pseudo_element && matches_chain(scope, ancestors, selector)
}

fn matches_chain(scope: Scope, ancestors: &[Scope], selector: &Selector) -> bool {
    if !matches_compound(scope, ancestors, selector) {
        return false;
    }
    match &selector.parent {
        None => true,
        Some((Combinator::Child, parent)) => match ancestors.split_last() {
            Some((nearest, rest)) => matches_chain(*nearest, rest, parent),
            None => false,
        },
        Some((Combinator::Descendant, parent)) => {
            let mut end = ancestors.len();
            while end > 0 {
                end -= 1;
                if matches_chain(ancestors[end], &ancestors[..end], parent) {
                    return true;
                }
            }
            false
        }
        Some((Combinator::Adjacent, parent)) => previous_sibling(scope, scope.position)
            .is_some_and(|previous| matches_chain(previous, ancestors, parent)),
        Some((Combinator::Sibling, parent)) => {
            let mut before = scope.position;
            while let Some(previous) = previous_sibling(scope, before) {
                if matches_chain(previous, ancestors, parent) {
                    return true;
                }
                before = previous.position;
            }
            false
        }
    }
}

fn matches_compound(scope: Scope, ancestors: &[Scope], selector: &Selector) -> bool {
    let element = scope.element;
    if let Some(tag_name) = &selector.tag_name {
        if *tag_name != element.tag_name {
            return false;
        }
    }
    if let Some(id) = &selector.id {
        if element.id() != Some(id) {
            return false;
        }
    }
    let classes = element.classes();
    if !selector
        .classes
        .iter()
        .all(|class| classes.contains(class.as_str()))
    {
        return false;
    }
    selector
        .attributes
        .iter()
        .all(|attribute| attribute_matches(element, attribute))
        && selector
            .pseudo
            .iter()
            .all(|pseudo| pseudo_matches(scope, ancestors, pseudo))
}

fn attribute_matches(element: &ElementData, attribute: &Attribute) -> bool {
    let Some(actual) = element.attributes.get(&attribute.name) else {
        return false;
    };
    let Some((op, expected)) = &attribute.test else {
        return true;
    };
    let (actual, expected) = if attribute.insensitive {
        (actual.to_lowercase(), expected.to_lowercase())
    } else {
        (actual.clone(), expected.clone())
    };
    match op {
        AttributeOp::Equals => actual == expected,
        AttributeOp::Includes => {
            !expected.is_empty()
                && !expected.contains(char::is_whitespace)
                && actual.split_whitespace().any(|word| word == expected)
        }
        AttributeOp::Dash => {
            actual == expected
                || actual
                    .strip_prefix(&expected)
                    .is_some_and(|r| r.starts_with('-'))
        }
        AttributeOp::Prefix => !expected.is_empty() && actual.starts_with(&expected),
        AttributeOp::Suffix => !expected.is_empty() && actual.ends_with(&expected),
        AttributeOp::Substring => !expected.is_empty() && actual.contains(&expected),
    }
}

fn pseudo_matches(scope: Scope, ancestors: &[Scope], pseudo: &Pseudo) -> bool {
    match pseudo {
        Pseudo::Link => {
            matches!(scope.element.tag_name.as_str(), "a" | "area")
                && scope.element.attributes.contains_key("href")
        }
        Pseudo::Never => false,
        Pseudo::Root => ancestors.is_empty(),
        Pseudo::Empty => scope.siblings[scope.position].children.is_empty(),
        Pseudo::Nth {
            a,
            b,
            last,
            of_type,
        } => nth_matches(*a, *b, sibling_position(scope, *last, *of_type)),
        Pseudo::Not(list) => !list
            .iter()
            .any(|selector| matches_chain(scope, ancestors, selector)),
        Pseudo::Is(list) | Pseudo::Where(list) => list
            .iter()
            .any(|selector| matches_chain(scope, ancestors, selector)),
    }
}
