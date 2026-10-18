// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;

use crate::css::{self, Color, Combinator, Rule, Selector, Specificity, Stylesheet, Value};
use crate::dom::{ElementData, Node, NodeType};
use crate::float::Side;

pub const USER_AGENT_STYLESHEET: &str =
    "head, style, script, title, meta, link, base { display: none; } \
    span, a, b, i, em, strong, code, small, u, label, img, mark, abbr, cite, kbd, samp, var, del, ins, s, q, sub, sup, time { display: inline; } \
    body { margin: 8px; } \
    h1 { font-size: 32px; } h2 { font-size: 24px; } h3 { font-size: 19px; } \
    p, h1, h2, h3 { margin-bottom: 12px; }";

pub const DEFAULT_FONT_SIZE: f32 = 16.0;

pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

const INHERITED_PROPERTIES: [&str; 4] = ["color", "font-size", "text-align", "line-height"];

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    Block,
    Flex,
    Inline,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexDirection {
    Row,
    RowReverse,
    Column,
    ColumnReverse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexWrap {
    NoWrap,
    Wrap,
    WrapReverse,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Align {
    Stretch,
    Start,
    End,
    Center,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Justify {
    Start,
    End,
    Center,
    SpaceBetween,
    SpaceAround,
    SpaceEvenly,
    Stretch,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FlexBasis {
    Auto,
    Content,
    Size(f32),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Position {
    Static,
    Relative,
    Absolute,
    Fixed,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clear {
    None,
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
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

#[derive(Debug, Clone, PartialEq)]
pub enum StyleSource {
    Inline(String),
    Link(String),
}

pub fn style_sources(root: &Node) -> Vec<StyleSource> {
    let mut sources = Vec::new();
    collect_style_sources(root, &mut sources);
    sources
}

fn collect_style_sources(node: &Node, sources: &mut Vec<StyleSource>) {
    if let NodeType::Element(element) = &node.node_type {
        if element.tag_name == "style" {
            let mut text = String::new();
            for child in &node.children {
                if let NodeType::Text(content) = &child.node_type {
                    text.push_str(content);
                    text.push('\n');
                }
            }
            sources.push(StyleSource::Inline(text));
            return;
        }
        if element.tag_name == "link" && is_stylesheet_link(element) {
            if let Some(href) = element.attributes.get("href") {
                sources.push(StyleSource::Link(href.clone()));
            }
        }
    }
    for child in &node.children {
        collect_style_sources(child, sources);
    }
}

fn is_stylesheet_link(element: &ElementData) -> bool {
    let Some(rel) = element.attributes.get("rel") else {
        return false;
    };
    let mut is_stylesheet = false;
    for token in rel.split_whitespace() {
        if token.eq_ignore_ascii_case("stylesheet") {
            is_stylesheet = true;
        }
    }
    let applies = match element.attributes.get("media") {
        Some(media) => media_applies(media),
        None => true,
    };
    is_stylesheet && applies
}

pub fn media_applies(media: &str) -> bool {
    if media.trim().is_empty() {
        return true;
    }
    for query in media.split(',') {
        let query = query.trim().to_ascii_lowercase();
        let query = query.strip_prefix("only ").unwrap_or(&query).trim_start();
        for kind in ["all", "screen"] {
            if let Some(rest) = query.strip_prefix(kind) {
                if rest.is_empty() || rest.starts_with(char::is_whitespace) {
                    return true;
                }
            }
        }
    }
    false
}

pub fn build_stylesheet(author_css: &str) -> Stylesheet {
    let mut stylesheet = css::parse(USER_AGENT_STYLESHEET);
    stylesheet.rules.extend(css::parse(author_css).rules);
    stylesheet
}

pub fn stylesheet_for_document(root: &Node) -> Stylesheet {
    let mut author_css = String::new();
    for source in style_sources(root) {
        if let StyleSource::Inline(text) = source {
            author_css.push_str(&text);
        }
    }
    build_stylesheet(&author_css)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub width: f32,
    pub height: f32,
}

impl Viewport {
    pub fn new(width: f32, height: f32) -> Viewport {
        Viewport { width, height }
    }
}

struct StyleContext<'s> {
    stylesheet: &'s Stylesheet,
    viewport: Viewport,
}

pub fn style_tree<'a>(
    root: &'a Node,
    stylesheet: &Stylesheet,
    viewport: Viewport,
) -> Option<StyledNode<'a>> {
    let context = StyleContext {
        stylesheet,
        viewport,
    };
    match &root.node_type {
        NodeType::Element(_) => Some(style_node(
            root,
            &context,
            &PropertyMap::new(),
            &mut Vec::new(),
            DEFAULT_FONT_SIZE,
        )),
        NodeType::Text(_) => None,
    }
}

fn style_node<'a>(
    node: &'a Node,
    context: &StyleContext,
    parent: &PropertyMap,
    ancestors: &mut Vec<&'a ElementData>,
    root_font_size: f32,
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
            let mut values = specified_values(element, ancestors, context.stylesheet, inherited);
            resolve_viewport_units(&mut values, context.viewport, root_font_size);
            resolve_font_size(&mut values, parent);
            resolve_line_height(&mut values);
            let children_root_font_size = if ancestors.is_empty() {
                font_size_of(&values)
            } else {
                root_font_size
            };
            ancestors.push(element);
            let mut children = Vec::new();
            for child in &node.children {
                children.push(style_node(
                    child,
                    context,
                    &values,
                    ancestors,
                    children_root_font_size,
                ));
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

fn font_size_of(values: &PropertyMap) -> f32 {
    match values.get("font-size") {
        Some(Value::Length(px)) => *px,
        _ => DEFAULT_FONT_SIZE,
    }
}

fn resolve_viewport_units(values: &mut PropertyMap, viewport: Viewport, root_font_size: f32) {
    for value in values.values_mut() {
        let resolved = match value {
            Value::Rem(rem) => Some(Value::Length(*rem * root_font_size)),
            Value::Vw(vw) => Some(Value::Length(*vw * viewport.width / 100.0)),
            Value::Vh(vh) => Some(Value::Length(*vh * viewport.height / 100.0)),
            Value::Vmin(vmin) => Some(Value::Length(
                *vmin * viewport.width.min(viewport.height) / 100.0,
            )),
            Value::Vmax(vmax) => Some(Value::Length(
                *vmax * viewport.width.max(viewport.height) / 100.0,
            )),
            Value::Calc(linear) => {
                let folded = linear.fold_viewport(root_font_size, viewport.width, viewport.height);
                if folded.is_px_only() {
                    Some(Value::Length(folded.px))
                } else {
                    Some(Value::Calc(folded))
                }
            }
            _ => None,
        };
        if let Some(resolved) = resolved {
            *value = resolved;
        }
    }
}

fn resolve_font_size(values: &mut PropertyMap, parent: &PropertyMap) {
    let parent_size = font_size_of(parent);
    let resolved = match values.get("font-size") {
        Some(Value::Length(px)) => Some(*px),
        Some(Value::Em(em)) => Some(em * parent_size),
        Some(Value::Percent(percent)) => Some(percent / 100.0 * parent_size),
        Some(Value::Calc(linear)) => linear.resolve(parent_size, Some(parent_size)),
        _ => None,
    };
    match resolved {
        Some(px) => values.insert("font-size".to_string(), Value::Length(px)),
        None => values.remove("font-size"),
    };
}

fn resolve_line_height(values: &mut PropertyMap) {
    let font_size = font_size_of(values);
    let resolved = match values.get("line-height") {
        Some(Value::Em(em)) => Some(em * font_size),
        Some(Value::Percent(percent)) => Some(percent / 100.0 * font_size),
        Some(Value::Calc(linear)) => linear.resolve(font_size, Some(font_size)),
        _ => None,
    };
    if let Some(px) = resolved {
        values.insert("line-height".to_string(), Value::Length(px));
    }
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
            Some(Value::Keyword(keyword)) if keyword == "flex" || keyword == "inline-flex" => {
                Display::Flex
            }
            _ => Display::Block,
        }
    }

    pub fn font_size(&self) -> f32 {
        match self.value("font-size") {
            Some(Value::Length(px)) => *px,
            _ => DEFAULT_FONT_SIZE,
        }
    }

    pub fn line_height(&self) -> Option<f32> {
        match self.value("line-height") {
            Some(Value::Number(factor)) => Some(factor * self.font_size()),
            Some(Value::Length(px)) => Some(*px),
            _ => None,
        }
    }

    pub fn text_align(&self) -> TextAlign {
        match self.value("text-align") {
            Some(Value::Keyword(keyword)) if keyword == "center" => TextAlign::Center,
            Some(Value::Keyword(keyword)) if keyword == "right" => TextAlign::Right,
            _ => TextAlign::Left,
        }
    }

    pub fn float(&self) -> Option<Side> {
        match self.value("float") {
            Some(Value::Keyword(keyword)) if keyword == "left" => Some(Side::Left),
            Some(Value::Keyword(keyword)) if keyword == "right" => Some(Side::Right),
            _ => None,
        }
    }

    pub fn clear(&self) -> Clear {
        match self.value("clear") {
            Some(Value::Keyword(keyword)) if keyword == "left" => Clear::Left,
            Some(Value::Keyword(keyword)) if keyword == "right" => Clear::Right,
            Some(Value::Keyword(keyword)) if keyword == "both" => Clear::Both,
            _ => Clear::None,
        }
    }

    pub fn position(&self) -> Position {
        match self.value("position") {
            Some(Value::Keyword(keyword)) if keyword == "relative" => Position::Relative,
            Some(Value::Keyword(keyword)) if keyword == "absolute" => Position::Absolute,
            Some(Value::Keyword(keyword)) if keyword == "fixed" => Position::Fixed,
            _ => Position::Static,
        }
    }

    pub fn is_positioned(&self) -> bool {
        self.position() != Position::Static
    }

    pub fn is_out_of_flow(&self) -> bool {
        matches!(self.position(), Position::Absolute | Position::Fixed)
    }

    pub fn z_index(&self) -> Option<i32> {
        match self.value("z-index") {
            Some(Value::Number(number)) => Some(*number as i32),
            Some(Value::Length(px)) if *px == 0.0 => Some(0),
            _ => None,
        }
    }

    fn keyword(&self, name: &str) -> Option<&str> {
        match self.value(name) {
            Some(Value::Keyword(keyword)) => Some(keyword.as_str()),
            _ => None,
        }
    }

    fn number(&self, name: &str, default: f32) -> f32 {
        match self.value(name) {
            Some(Value::Number(number)) => *number,
            Some(Value::Length(px)) if *px == 0.0 => 0.0,
            _ => default,
        }
    }

    pub fn flex_direction(&self) -> FlexDirection {
        match self.keyword("flex-direction") {
            Some("row-reverse") => FlexDirection::RowReverse,
            Some("column") => FlexDirection::Column,
            Some("column-reverse") => FlexDirection::ColumnReverse,
            _ => FlexDirection::Row,
        }
    }

    pub fn flex_wrap(&self) -> FlexWrap {
        match self.keyword("flex-wrap") {
            Some("wrap") => FlexWrap::Wrap,
            Some("wrap-reverse") => FlexWrap::WrapReverse,
            _ => FlexWrap::NoWrap,
        }
    }

    fn align_keyword(keyword: &str) -> Option<Align> {
        match keyword {
            "stretch" | "normal" => Some(Align::Stretch),
            "flex-start" | "start" | "self-start" | "baseline" | "first baseline" => {
                Some(Align::Start)
            }
            "flex-end" | "end" | "self-end" | "last baseline" => Some(Align::End),
            "center" => Some(Align::Center),
            _ => None,
        }
    }

    pub fn align_items(&self) -> Align {
        match self.keyword("align-items") {
            Some(keyword) => StyledNode::align_keyword(keyword).unwrap_or(Align::Stretch),
            None => Align::Stretch,
        }
    }

    pub fn align_self(&self) -> Option<Align> {
        StyledNode::align_keyword(self.keyword("align-self")?)
    }

    pub fn justify_content(&self) -> Justify {
        match self.keyword("justify-content") {
            Some("flex-end") | Some("end") | Some("right") => Justify::End,
            Some("center") => Justify::Center,
            Some("space-between") => Justify::SpaceBetween,
            Some("space-around") => Justify::SpaceAround,
            Some("space-evenly") => Justify::SpaceEvenly,
            _ => Justify::Start,
        }
    }

    pub fn align_content(&self) -> Justify {
        match self.keyword("align-content") {
            Some("flex-start") | Some("start") => Justify::Start,
            Some("flex-end") | Some("end") => Justify::End,
            Some("center") => Justify::Center,
            Some("space-between") => Justify::SpaceBetween,
            Some("space-around") => Justify::SpaceAround,
            Some("space-evenly") => Justify::SpaceEvenly,
            _ => Justify::Stretch,
        }
    }

    pub fn flex_grow(&self) -> f32 {
        self.number("flex-grow", 0.0).max(0.0)
    }

    pub fn flex_shrink(&self) -> f32 {
        self.number("flex-shrink", 1.0).max(0.0)
    }

    pub fn flex_basis(&self, percent_base: Option<f32>) -> FlexBasis {
        if self.keyword("flex-basis") == Some("content") {
            return FlexBasis::Content;
        }
        match self.length_or_auto("flex-basis", None, percent_base) {
            Some(size) => FlexBasis::Size(size),
            None => FlexBasis::Auto,
        }
    }

    pub fn order(&self) -> i32 {
        self.number("order", 0.0) as i32
    }

    pub fn establishes_block_context(&self) -> bool {
        if self.float().is_some() || self.is_out_of_flow() || self.display() == Display::Flex {
            return true;
        }
        match self.value("overflow") {
            Some(Value::Keyword(keyword)) => {
                keyword == "hidden" || keyword == "auto" || keyword == "scroll"
            }
            _ => false,
        }
    }

    pub fn background(&self) -> Option<Color> {
        self.color("background-color")
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
            Some(Value::Calc(linear)) => linear.resolve(self.font_size(), percent_base),
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
