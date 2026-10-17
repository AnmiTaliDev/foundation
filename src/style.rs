// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::css::{self, Color, Rule, Selector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeType};

pub const USER_AGENT_STYLESHEET: &str =
    "head, style, script, title, meta, link, base { display: none; } body { margin: 8px; }";

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    Block,
    None,
}

#[derive(Debug)]
pub struct StyledNode<'a> {
    pub element: &'a ElementData,
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
    let NodeType::Element(element) = &root.node_type else {
        return None;
    };
    let mut children = Vec::new();
    for child in &root.children {
        if let Some(styled) = style_tree(child, stylesheet) {
            children.push(styled);
        }
    }
    Some(StyledNode {
        element,
        specified_values: specified_values(element, stylesheet),
        children,
    })
}

fn specified_values(element: &ElementData, stylesheet: &Stylesheet) -> PropertyMap {
    let mut matched: Vec<(Specificity, &Rule)> = Vec::new();
    for rule in &stylesheet.rules {
        if let Some(specificity) = match_rule(element, rule) {
            matched.push((specificity, rule));
        }
    }
    matched.sort_by_key(|entry| entry.0);

    let mut values = PropertyMap::new();
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
        match self.value("display") {
            Some(Value::Keyword(keyword)) if keyword == "none" => Display::None,
            _ => Display::Block,
        }
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
