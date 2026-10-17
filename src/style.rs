// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::css::{self, Color, Rule, Selector, Specificity, Stylesheet, Value};
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
        NodeType::Element(_) => Some(style_node(root, stylesheet, &PropertyMap::new())),
        NodeType::Text(_) => None,
    }
}

fn style_node<'a>(node: &'a Node, stylesheet: &Stylesheet, parent: &PropertyMap) -> StyledNode<'a> {
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
            let values = specified_values(element, stylesheet, inherited);
            let mut children = Vec::new();
            for child in &node.children {
                children.push(style_node(child, stylesheet, &values));
            }
            StyledNode {
                content: Content::Element(element),
                specified_values: values,
                children,
            }
        }
    }
}

fn specified_values(
    element: &ElementData,
    stylesheet: &Stylesheet,
    inherited: PropertyMap,
) -> PropertyMap {
    let mut matched: Vec<(Specificity, &Rule)> = Vec::new();
    for rule in &stylesheet.rules {
        if let Some(specificity) = match_rule(element, rule) {
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
    values
}

fn match_rule(element: &ElementData, rule: &Rule) -> Option<Specificity> {
    let mut best: Option<Specificity> = None;
    for selector in &rule.selectors {
        if matches(element, selector) {
            let specificity = selector.specificity();
            if best.is_none_or(|current| specificity > current) {
                best = Some(specificity);
            }
        }
    }
    best
}

fn matches(element: &ElementData, selector: &Selector) -> bool {
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
        self.length_or_auto("font-size", Some(DEFAULT_FONT_SIZE))
            .unwrap_or(DEFAULT_FONT_SIZE)
    }

    pub fn text_color(&self) -> Color {
        self.color("color").unwrap_or(BLACK)
    }

    pub fn length_or_auto(&self, name: &str, default: Option<f32>) -> Option<f32> {
        match self.value(name) {
            Some(Value::Length(px)) => Some(*px),
            Some(Value::Keyword(keyword)) if keyword == "auto" => None,
            _ => default,
        }
    }

    pub fn px(&self, name: &str) -> f32 {
        self.length_or_auto(name, Some(0.0)).unwrap_or(0.0)
    }

    pub fn color(&self, name: &str) -> Option<Color> {
        match self.value(name) {
            Some(Value::Color(color)) => Some(*color),
            _ => None,
        }
    }
}
