// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::css::{self, Color, Combinator, Rule, Selector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeType};

pub const USER_AGENT_STYLESHEET: &str =
    "head, style, script, title, meta, link, base { display: none; } \
    span, a, b, i, em, strong, code, small, u, label { display: inline; } \
    body { margin: 8px; } \
    h1 { font-size: 32px; } h2 { font-size: 24px; } h3 { font-size: 19px; } \
    p, h1, h2, h3 { margin-bottom: 12px; }";

pub const DEFAULT_FONT_SIZE: f32 = 16.0;

pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

const INHERITED_PROPERTIES: [&str; 2] = ["color", "font-size"];

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    Block,
    Inline,
    None,
}

#[derive(Debug)]
pub enum Content<'a> {
    Element(&'a ElementData),
    Text(&'a str),
}

#[derive(Debug)]
pub struct StyledNode<'a> {
    pub content: Content<'a>,
    pub specified_values: PropertyMap,
    pub children: Vec<StyledNode<'a>>,
}

pub fn stylesheet_for_document(root: &Node) -> Stylesheet {
    let mut stylesheet = css::parse(USER_AGENT_STYLESHEET);
    let mut author_css = String::new();
    collect_style_text(root, &mut author_css);
    stylesheet.rules.extend(css::parse(&author_css).rules);
    stylesheet
}

fn collect_style_text(node: &Node, out: &mut String) {
    if let NodeType::Element(element) = &node.node_type {
        if element.tag_name == "style" {
            for child in &node.children {
                if let NodeType::Text(text) = &child.node_type {
                    out.push_str(text);
                    out.push('\n');
                }
            }
            return;
        }
    }
    for child in &node.children {
        collect_style_text(child, out);
    }
}

pub fn style_tree<'a>(root: &'a Node, stylesheet: &Stylesheet) -> Option<StyledNode<'a>> {
    match &root.node_type {
        NodeType::Element(_) => Some(style_node(
            root,
            stylesheet,
            &PropertyMap::new(),
            &mut Vec::new(),
        )),
        NodeType::Text(_) => None,
    }
}

fn style_node<'a>(
    node: &'a Node,
    stylesheet: &Stylesheet,
    parent: &PropertyMap,
    ancestors: &mut Vec<&'a ElementData>,
) -> StyledNode<'a> {
    let mut inherited = PropertyMap::new();
    for name in INHERITED_PROPERTIES {
        if let Some(value) = parent.get(name) {
            inherited.insert(name.to_string(), value.clone());
        }
    }
    match &node.node_type {
        NodeType::Text(text) => StyledNode {
            content: Content::Text(text),
            specified_values: inherited,
            children: Vec::new(),
        },
        NodeType::Element(element) => {
            let mut values = specified_values(element, ancestors, stylesheet, inherited);
            resolve_font_size(&mut values, parent);
            ancestors.push(element);
            let mut children = Vec::new();
            for child in &node.children {
                children.push(style_node(child, stylesheet, &values, ancestors));
            }
            ancestors.pop();
            StyledNode {
                content: Content::Element(element),
                specified_values: values,
                children,
            }
        }
    }
}

fn resolve_font_size(values: &mut PropertyMap, parent: &PropertyMap) {
    let parent_size = match parent.get("font-size") {
        Some(Value::Length(px)) => *px,
        _ => DEFAULT_FONT_SIZE,
    };
    let resolved = match values.get("font-size") {
        Some(Value::Length(px)) => Some(*px),
        Some(Value::Em(em)) => Some(em * parent_size),
        Some(Value::Percent(percent)) => Some(percent / 100.0 * parent_size),
        _ => None,
    };
    match resolved {
        Some(px) => values.insert("font-size".to_string(), Value::Length(px)),
        None => values.remove("font-size"),
    };
}

fn specified_values(
    element: &ElementData,
    ancestors: &[&ElementData],
    stylesheet: &Stylesheet,
    inherited: PropertyMap,
) -> PropertyMap {
    let mut matched: Vec<(Specificity, &Rule)> = Vec::new();
    for rule in &stylesheet.rules {
        if let Some(specificity) = match_rule(element, ancestors, rule) {
            matched.push((specificity, rule));
        }
    }
    matched.sort_by_key(|entry| entry.0);

    let mut values = inherited;
    for (_, rule) in matched {
        for declaration in &rule.declarations {
            values.insert(declaration.name.clone(), declaration.value.clone());
        }
    }
    if let Some(inline) = element.attributes.get("style") {
        for declaration in css::parse_declarations(inline) {
            values.insert(declaration.name, declaration.value);
        }
    }
    values
}

fn match_rule(
    element: &ElementData,
    ancestors: &[&ElementData],
    rule: &Rule,
) -> Option<Specificity> {
    let mut best: Option<Specificity> = None;
    for selector in &rule.selectors {
        if matches_selector(element, ancestors, selector) {
            let specificity = selector.specificity();
            if best.is_none_or(|current| specificity > current) {
                best = Some(specificity);
            }
        }
    }
    best
}

fn matches_selector(
    element: &ElementData,
    ancestors: &[&ElementData],
    selector: &Selector,
) -> bool {
    if !matches_compound(element, selector) {
        return false;
    }
    match &selector.parent {
        None => true,
        Some((Combinator::Child, parent)) => match ancestors.split_last() {
            Some((nearest, rest)) => matches_selector(nearest, rest, parent),
            None => false,
        },
        Some((Combinator::Descendant, parent)) => {
            let mut end = ancestors.len();
            while end > 0 {
                end -= 1;
                if matches_selector(ancestors[end], &ancestors[..end], parent) {
                    return true;
                }
            }
            false
        }
    }
}

fn matches_compound(element: &ElementData, selector: &Selector) -> bool {
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
    for class in &selector.classes {
        if !classes.contains(class.as_str()) {
            return false;
        }
    }
    true
}

impl<'a> StyledNode<'a> {
    pub fn value(&self, name: &str) -> Option<&Value> {
        self.specified_values.get(name)
    }

    pub fn display(&self) -> Display {
        if let Content::Text(_) = self.content {
            return Display::Inline;
        }
        match self.value("display") {
            Some(Value::Keyword(keyword)) if keyword == "none" => Display::None,
            Some(Value::Keyword(keyword)) if keyword == "inline" => Display::Inline,
            _ => Display::Block,
        }
    }

    pub fn font_size(&self) -> f32 {
        match self.value("font-size") {
            Some(Value::Length(px)) => *px,
            _ => DEFAULT_FONT_SIZE,
        }
    }

    pub fn text_color(&self) -> Color {
        self.color("color").unwrap_or(BLACK)
    }

    pub fn length_or_auto(
        &self,
        name: &str,
        default: Option<f32>,
        percent_base: Option<f32>,
    ) -> Option<f32> {
        match self.value(name) {
            Some(Value::Length(px)) => Some(*px),
            Some(Value::Em(em)) => Some(em * self.font_size()),
            Some(Value::Percent(percent)) => percent_base.map(|base| percent / 100.0 * base),
            Some(Value::Keyword(keyword)) if keyword == "auto" => None,
            _ => default,
        }
    }

    pub fn px(&self, name: &str, percent_base: f32) -> f32 {
        self.length_or_auto(name, Some(0.0), Some(percent_base))
            .unwrap_or(0.0)
    }

    pub fn color(&self, name: &str) -> Option<Color> {
        match self.value(name) {
            Some(Value::Color(color)) => Some(*color),
            _ => None,
        }
    }
}
