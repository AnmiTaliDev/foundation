// SPDX-License-Identifier: GPL-3.0-only

use crate::dom::{ElementData, Node, NodeType};

pub struct ControlText {
    pub text: String,
    pub placeholder: bool,
}

pub fn control_text(element: &ElementData, children: &[Node]) -> Option<ControlText> {
    let attribute = |name: &str| element.attributes.get(name).map(String::as_str);
    let plain = |text: String| ControlText {
        text,
        placeholder: false,
    };
    match element.tag_name.as_str() {
        "input" => {
            let kind = attribute("type").unwrap_or("text").to_ascii_lowercase();
            let value = attribute("value").unwrap_or("");
            match kind.as_str() {
                "checkbox" | "radio" | "hidden" | "image" | "range" | "color" | "file" => None,
                "submit" => Some(plain(non_empty(value, "Submit"))),
                "reset" => Some(plain(non_empty(value, "Reset"))),
                "button" => Some(plain(value.to_string())),
                _ if !value.is_empty() => Some(plain(if kind == "password" {
                    "\u{2022}".repeat(value.chars().count())
                } else {
                    value.to_string()
                })),
                _ => attribute("placeholder")
                    .filter(|text| !text.is_empty())
                    .map(|text| ControlText {
                        text: text.to_string(),
                        placeholder: true,
                    }),
            }
        }
        "select" => {
            let options = options(children);
            let chosen = options
                .iter()
                .find(|option| option.0.attributes.contains_key("selected"))
                .or(options.first())?;
            Some(plain(text_of(chosen.1)))
        }
        _ => None,
    }
}

fn non_empty(value: &str, fallback: &str) -> String {
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.to_string()
    }
}

fn options(children: &[Node]) -> Vec<(&ElementData, &Node)> {
    let mut found = Vec::new();
    for child in children {
        if let NodeType::Element(element) = &child.node_type {
            match element.tag_name.as_str() {
                "option" => found.push((element, child)),
                "optgroup" => found.extend(options(&child.children)),
                _ => {}
            }
        }
    }
    found
}

fn text_of(node: &Node) -> String {
    match &node.node_type {
        NodeType::Text(text) => text.clone(),
        NodeType::Element(_) => node.children.iter().map(text_of).collect(),
    }
}
