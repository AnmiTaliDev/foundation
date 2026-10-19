// SPDX-License-Identifier: GPL-3.0-only

use crate::dom::{AttrMap, Node, NodeType};
use crate::entities;

const VOID_ELEMENTS: [&str; 14] = [
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "param", "source",
    "track", "wbr",
];

const RAW_TEXT_ELEMENTS: [&str; 2] = ["script", "style"];

struct OpenElement {
    tag_name: String,
    attributes: AttrMap,
    children: Vec<Node>,
}

struct Parser<'a> {
    input: &'a str,
    pos: usize,
    stack: Vec<OpenElement>,
}

pub fn parse(source: &str) -> Node {
    let mut parser = Parser {
        input: source,
        pos: 0,
        stack: vec![OpenElement {
            tag_name: "html".to_string(),
            attributes: AttrMap::new(),
            children: Vec::new(),
        }],
    };
    parser.run();
    parser.finish()
}

fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == ':'
}

fn is_attr_name_char(c: char) -> bool {
    !c.is_whitespace() && c != '=' && c != '>' && c != '/'
}

fn is_unquoted_value_char(c: char) -> bool {
    !c.is_whitespace() && c != '>'
}

fn is_not_tag_start(c: char) -> bool {
    c != '<'
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn eof(&self) -> bool {
        self.pos >= self.input.len()
    }

    fn next_char(&self) -> Option<char> {
        self.rest().chars().next()
    }

    fn consume_char(&mut self) {
        if let Some(c) = self.next_char() {
            self.pos += c.len_utf8();
        }
    }

    fn consume_while(&mut self, test: fn(char) -> bool) -> &'a str {
        let rest = self.rest();
        let remaining = rest.trim_start_matches(test);
        let consumed = &rest[..rest.len() - remaining.len()];
        self.pos += consumed.len();
        consumed
    }

    fn consume_whitespace(&mut self) {
        self.consume_while(char::is_whitespace);
    }

    fn skip_past(&mut self, pattern: &str) {
        match self.rest().find(pattern) {
            Some(index) => self.pos += index + pattern.len(),
            None => self.pos = self.input.len(),
        }
    }

    fn append(&mut self, node: Node) {
        self.stack.last_mut().unwrap().children.push(node);
    }

    fn close_top(&mut self) {
        let element = self.stack.pop().unwrap();
        let node = Node::element(element.tag_name, element.attributes, element.children);
        self.append(node);
    }

    fn run(&mut self) {
        while !self.eof() {
            let rest = self.rest();
            if rest.starts_with("<!--") {
                self.skip_past("-->");
            } else if rest.starts_with("<!") || rest.starts_with("<?") {
                self.skip_past(">");
            } else if rest.starts_with("</") {
                self.parse_close_tag();
            } else if rest.starts_with('<')
                && rest[1..]
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_ascii_alphabetic())
            {
                self.parse_open_tag();
            } else {
                self.parse_text();
            }
        }
    }

    fn parse_text(&mut self) {
        let start = self.pos;
        self.consume_char();
        self.consume_while(is_not_tag_start);
        let mut text = &self.input[start..self.pos];
        let top = self.stack.last().unwrap();
        if top.children.is_empty() && matches!(top.tag_name.as_str(), "pre" | "listing") {
            text = text.strip_prefix('\n').unwrap_or(text);
        }
        if !text.is_empty() && (self.stack.len() > 1 || !text.trim().is_empty()) {
            self.append(Node::text(entities::decode(text)));
        }
    }

    fn parse_close_tag(&mut self) {
        self.pos += 2;
        let name = self.consume_while(is_name_char).to_ascii_lowercase();
        self.skip_past(">");
        let mut index = self.stack.len();
        while index > 1 {
            index -= 1;
            if self.stack[index].tag_name == name {
                while self.stack.len() > index {
                    self.close_top();
                }
                return;
            }
        }
    }

    fn parse_open_tag(&mut self) {
        self.pos += 1;
        let name = self.consume_while(is_name_char).to_ascii_lowercase();
        let mut attributes = AttrMap::new();
        let mut self_closing = false;
        loop {
            self.consume_whitespace();
            match self.next_char() {
                None => break,
                Some('>') => {
                    self.pos += 1;
                    break;
                }
                Some('/') => {
                    self.pos += 1;
                    self_closing = true;
                }
                Some(_) => {
                    let attr_name = self.consume_while(is_attr_name_char).to_ascii_lowercase();
                    if attr_name.is_empty() {
                        self.consume_char();
                        continue;
                    }
                    let value = self.parse_attr_value();
                    attributes.insert(attr_name, value);
                }
            }
        }

        if self_closing || VOID_ELEMENTS.contains(&name.as_str()) {
            self.append(Node::element(name, attributes, Vec::new()));
        } else if RAW_TEXT_ELEMENTS.contains(&name.as_str()) {
            let children = self.parse_raw_text(&name);
            self.append(Node::element(name, attributes, children));
        } else {
            self.stack.push(OpenElement {
                tag_name: name,
                attributes,
                children: Vec::new(),
            });
        }
    }

    fn parse_attr_value(&mut self) -> String {
        self.consume_whitespace();
        if self.next_char() != Some('=') {
            return String::new();
        }
        self.pos += 1;
        self.consume_whitespace();
        match self.next_char() {
            Some(quote @ ('"' | '\'')) => {
                self.pos += 1;
                let start = self.pos;
                let end = match self.rest().find(quote) {
                    Some(index) => start + index,
                    None => self.input.len(),
                };
                self.pos = (end + 1).min(self.input.len());
                entities::decode(&self.input[start..end])
            }
            _ => entities::decode(self.consume_while(is_unquoted_value_char)),
        }
    }

    fn parse_raw_text(&mut self, name: &str) -> Vec<Node> {
        let rest = self.rest();
        let closing = format!("</{}", name);
        let end = rest
            .to_ascii_lowercase()
            .find(&closing)
            .unwrap_or(rest.len());
        let text = &rest[..end];
        self.pos += end;
        self.skip_past(">");
        if text.is_empty() {
            Vec::new()
        } else {
            vec![Node::text(text.to_string())]
        }
    }

    fn finish(mut self) -> Node {
        while self.stack.len() > 1 {
            self.close_top();
        }
        let mut root = self.stack.pop().unwrap();
        if root.children.len() == 1 && matches!(root.children[0].node_type, NodeType::Element(_)) {
            return root.children.pop().unwrap();
        }
        Node::element(root.tag_name, root.attributes, root.children)
    }
}
