// SPDX-License-Identifier: GPL-3.0-only

use std::borrow::Cow;
use std::collections::HashMap;

use crate::collapse::Edge;
use crate::color::Rgba;
use crate::control::control_text;
use crate::counters::Counters;
use crate::css::{self, Color, Declaration, Rule, Specificity, Stylesheet, Value};
use crate::decoration::{self, Decoration, LineStyle};
use crate::dom::{ElementData, Node, NodeType};
use crate::float::Side;
use crate::generated;
use crate::gradient::Gradient;
use crate::gridspec::{Line, TrackList};
use crate::layout::Rect;
use crate::radius::Radii;
use crate::selector::{self, PseudoElement, Scope};
use crate::shadow::Shadow;
use crate::transform::{self, Matrix};

pub const USER_AGENT_STYLESHEET: &str =
    "head, style, script, title, meta, link, base { display: none; } \
    span, a, b, i, em, strong, code, small, u, label, img, mark, abbr, cite, kbd, samp, var, del, ins, s, q, sub, sup, time { display: inline; } \
    body { margin: 8px; } \
    h1 { font-size: 32px; } h2 { font-size: 24px; } h3 { font-size: 19px; } \
    p, h1, h2, h3 { margin-bottom: 12px; } \
    b, strong, h1, h2, h3, h4, h5, h6, th { font-weight: bold; } \
    i, em, cite, var, address { font-style: italic; } \
    code, kbd, samp, pre, tt { font-family: monospace; } \
    sub { vertical-align: sub; font-size: 13px; } sup { vertical-align: super; font-size: 13px; } \
    q::before { content: open-quote; } q::after { content: close-quote; } \
    u, ins { text-decoration: underline; } \
    a:link { color: #0000ee; text-decoration: underline; } \
    s, del { text-decoration: line-through; } \
    pre { margin-top: 16px; margin-bottom: 16px; white-space: pre; } \
    table { display: table; border-spacing: 2px; } \
    thead, tbody, tfoot { display: table-row-group; } \
    tr { display: table-row; } \
    td, th { display: table-cell; padding: 1px; } \
    th { font-weight: bold; text-align: center; } \
    caption { text-align: center; } \
    colgroup, col { display: none; } \
    ul, ol { margin-top: 16px; margin-bottom: 16px; padding-left: 40px; } \
    ul ul, ul ol, ol ul, ol ol { margin-top: 0; margin-bottom: 0; } \
    li { display: list-item; } \
    ul { list-style-type: disc; } ol { list-style-type: decimal; } \
    ul ul, ol ul { list-style-type: circle; } \
    ul ul ul, ol ul ul, ul ol ul, ol ol ul { list-style-type: square; } \
    ul ol, ol ol { list-style-type: decimal; } \
    form { display: block; } \
    fieldset { display: block; margin: 0 2px; padding: 6px 12px 10px; border: 2px groove #c0c0c0; } \
    legend { display: block; padding: 0 2px; } \
    button, input, select, textarea { display: inline-block; font-size: 13px; color: #000000; background: #ffffff; border: 1px solid #767676; border-radius: 2px; padding: 1px 2px; box-sizing: border-box; } \
    input { width: 160px; height: 21px; white-space: pre; overflow: hidden; } \
    textarea { width: 180px; height: 40px; padding: 2px; white-space: pre-wrap; overflow: hidden; font-family: monospace; vertical-align: text-bottom; } \
    select { width: 80px; height: 21px; overflow: hidden; white-space: pre; } \
    option, optgroup, input[type=hidden] { display: none; } \
    button, input[type=submit], input[type=reset], input[type=button] { width: auto; min-width: 0; height: auto; padding: 1px 6px; background: #efefef; text-align: center; } \
    input[type=checkbox], input[type=radio] { width: 13px; height: 13px; padding: 0; margin: 3px 3px 3px 4px; } \
    input[type=radio] { border-radius: 50%; } \
    input[type=checkbox]:checked { background: #0075ff; border-color: #0075ff; } \
    input[type=radio]:checked { background: radial-gradient(#0075ff 40%, #ffffff 45%); border-color: #0075ff; } \
    button:disabled, input:disabled, select:disabled, textarea:disabled { color: #6d6d6d; background: #ebebeb; border-color: #c4c4c4; }";

pub const DEFAULT_FONT_SIZE: f32 = 16.0;
pub const SCROLLBAR_THUMB: Color = Color {
    r: 193,
    g: 193,
    b: 193,
};
pub const SCROLLBAR_TRACK: Color = Color {
    r: 241,
    g: 241,
    b: 241,
};
const PLACEHOLDER_COLOR: Color = Color {
    r: 117,
    g: 117,
    b: 117,
};

pub const BLACK: Color = Color { r: 0, g: 0, b: 0 };

const INHERITED_PROPERTIES: [&str; 19] = [
    "visibility",
    "scrollbar-thumb-color",
    "scrollbar-track-color",
    "quotes",
    "word-break",
    "overflow-wrap",
    "text-indent",
    "text-transform",
    "letter-spacing",
    "word-spacing",
    "white-space",
    "list-style-type",
    "color",
    "font-size",
    "text-align",
    "line-height",
    "font-family",
    "font-weight",
    "font-style",
];

const BORDER_STYLES: [&str; 4] = [
    "border-top-style",
    "border-right-style",
    "border-bottom-style",
    "border-left-style",
];

const BORDER_COLORS: [&str; 4] = [
    "border-top-color",
    "border-right-color",
    "border-bottom-color",
    "border-left-color",
];

const BORDER_WIDTHS: [&str; 4] = [
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Display {
    Block,
    Flex,
    Inline,
    InlineBlock,
    ListItem,
    Table,
    TableRowGroup,
    TableRow,
    TableCell,
    Grid,
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
    Sticky,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Clear {
    None,
    Left,
    Right,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VerticalAlign {
    Raise(f32),
    Middle,
    TextTop,
    TextBottom,
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextTransform {
    None,
    Uppercase,
    Lowercase,
    Capitalize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum WhiteSpace {
    Normal,
    Nowrap,
    Pre,
    PreWrap,
    PreLine,
}

impl WhiteSpace {
    pub fn wraps(self) -> bool {
        matches!(
            self,
            WhiteSpace::Normal | WhiteSpace::PreWrap | WhiteSpace::PreLine
        )
    }

    pub fn keeps_spaces(self) -> bool {
        matches!(self, WhiteSpace::Pre | WhiteSpace::PreWrap)
    }

    pub fn keeps_breaks(self) -> bool {
        !matches!(self, WhiteSpace::Normal | WhiteSpace::Nowrap)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TextAlign {
    Left,
    Center,
    Right,
    Justify,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
    Double,
}

fn stroke_style(keyword: Option<&str>) -> StrokeStyle {
    match keyword {
        Some("dashed") => StrokeStyle::Dashed,
        Some("dotted") => StrokeStyle::Dotted,
        Some("double") => StrokeStyle::Double,
        _ => StrokeStyle::Solid,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ContentSize {
    Min,
    Max,
    Fit(Option<f32>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackgroundBox {
    Border,
    Padding,
    Content,
}

fn background_box(keyword: Option<&str>) -> BackgroundBox {
    match keyword {
        Some("padding-box") => BackgroundBox::Padding,
        Some("content-box") => BackgroundBox::Content,
        _ => BackgroundBox::Border,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outline {
    pub style: StrokeStyle,
    pub width: f32,
    pub offset: f32,
    pub color: Rgba,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Indent {
    pub px: f32,
    pub percent: f32,
}

#[derive(Debug)]
pub enum Content<'a> {
    Element(&'a ElementData),
    Text(Cow<'a, str>),
    Pseudo,
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

const USER_AGENT_LAYER: &str = "(user-agent)";

pub fn build_stylesheet(author_css: &str) -> Stylesheet {
    let author = css::parse(author_css);
    with_author_rules(author.rules, author.layers)
}

pub fn with_author_rules(rules: Vec<Rule>, layers: Vec<String>) -> Stylesheet {
    let mut stylesheet = css::parse(USER_AGENT_STYLESHEET);
    for rule in &mut stylesheet.rules {
        rule.layer = Some(USER_AGENT_LAYER.to_string());
    }
    stylesheet.layers = vec![USER_AGENT_LAYER.to_string()];
    stylesheet.layers.extend(layers);
    stylesheet.rules.extend(rules);
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
            std::slice::from_ref(root),
            0,
            &context,
            &PropertyMap::new(),
            &mut Walk {
                ancestors: Vec::new(),
                counters: Counters::new(),
                quote_depth: 0,
            },
            DEFAULT_FONT_SIZE,
        )),
        NodeType::Text(_) => None,
    }
}

struct Walk<'a> {
    ancestors: Vec<Scope<'a>>,
    counters: Counters,
    quote_depth: usize,
}

fn style_node<'a>(
    node: &'a Node,
    siblings: &'a [Node],
    position: usize,
    context: &StyleContext,
    parent: &PropertyMap,
    walk: &mut Walk<'a>,
    root_font_size: f32,
) -> StyledNode<'a> {
    let inherited = inherit(parent);
    match &node.node_type {
        NodeType::Text(text) => StyledNode {
            content: Content::Text(Cow::Borrowed(text)),
            specified_values: inherited,
            children: Vec::new(),
        },
        NodeType::Element(element) => {
            let scope = Scope {
                element,
                siblings,
                position,
            };
            let mut values = specified_values(scope, &walk.ancestors, context, inherited, None);
            compute(&mut values, parent, context, root_font_size);
            let children_root_font_size = if walk.ancestors.is_empty() {
                font_size_of(&values)
            } else {
                root_font_size
            };
            apply_counters(&mut walk.counters, &values);
            walk.counters.enter();
            let before = pseudo_node(
                scope,
                walk,
                context,
                &values,
                PseudoElement::Before,
                root_font_size,
            );
            walk.ancestors.push(scope);
            let mut children = Vec::new();
            children.extend(before);
            for (index, child) in node.children.iter().enumerate() {
                children.push(style_node(
                    child,
                    &node.children,
                    index,
                    context,
                    &values,
                    walk,
                    children_root_font_size,
                ));
            }
            walk.ancestors.pop();
            if let Some(control) = control_text(element, &node.children) {
                let mut text_values = inherit(&values);
                if control.placeholder {
                    text_values.insert("color".to_string(), Value::Color(PLACEHOLDER_COLOR));
                }
                children.push(StyledNode {
                    content: Content::Text(Cow::Owned(control.text)),
                    specified_values: text_values,
                    children: Vec::new(),
                });
            }
            let after = pseudo_node(
                scope,
                walk,
                context,
                &values,
                PseudoElement::After,
                root_font_size,
            );
            walk.counters.leave();
            children.extend(after);
            StyledNode {
                content: Content::Element(element),
                specified_values: values,
                children,
            }
        }
    }
}

fn inherit(parent: &PropertyMap) -> PropertyMap {
    let mut inherited = PropertyMap::new();
    for name in INHERITED_PROPERTIES {
        if let Some(value) = parent.get(name) {
            inherited.insert(name.to_string(), value.clone());
        }
    }
    if let Some(active) = parent.get(ACTIVE_DECORATIONS) {
        inherited.insert(ACTIVE_DECORATIONS.to_string(), active.clone());
    }
    inherited
}

fn compute(
    values: &mut PropertyMap,
    parent: &PropertyMap,
    context: &StyleContext,
    root_font_size: f32,
) {
    resolve_viewport_units(values, context.viewport, root_font_size);
    resolve_font_size(values, parent);
    resolve_spacing(values);
    resolve_line_height(values);
    resolve_border_styles(values);
    resolve_current_color(values, parent);
    resolve_decorations(values, parent);
}

fn apply_counters(counters: &mut Counters, values: &PropertyMap) {
    if matches!(values.get("display"), Some(Value::Keyword(keyword)) if keyword == "none") {
        return;
    }
    for name in ["counter-reset", "counter-increment", "counter-set"] {
        let Some(Value::Counters(list)) = values.get(name) else {
            continue;
        };
        for (counter, number) in list {
            match name {
                "counter-reset" => counters.reset(counter, *number),
                "counter-increment" => counters.increment(counter, *number),
                _ => counters.set(counter, *number),
            }
        }
    }
}

fn pseudo_node<'a>(
    scope: Scope,
    walk: &mut Walk,
    context: &StyleContext,
    parent: &PropertyMap,
    which: PseudoElement,
    root_font_size: f32,
) -> Option<StyledNode<'a>> {
    let mut inherited = inherit(parent);
    inherited.insert("display".to_string(), Value::Keyword("inline".to_string()));
    let mut values = specified_values(scope, &walk.ancestors, context, inherited, Some(which));
    let Some(Value::Content(parts)) = values.get("content") else {
        return None;
    };
    apply_counters(&mut walk.counters, &values);
    let quotes: Vec<(String, String)> = match values.get("quotes") {
        Some(Value::Quotes(pairs)) => pairs.clone(),
        Some(Value::Keyword(keyword)) if keyword == "none" => Vec::new(),
        _ => generated::DEFAULT_QUOTES
            .iter()
            .map(|(open, close)| (open.to_string(), close.to_string()))
            .collect(),
    };
    let text = generated::render(
        parts,
        scope.element,
        &walk.counters,
        &quotes,
        &mut walk.quote_depth,
    );
    compute(&mut values, parent, context, root_font_size);
    let mut children = Vec::new();
    if !text.is_empty() {
        children.push(StyledNode {
            content: Content::Text(Cow::Owned(text)),
            specified_values: inherit(&values),
            children: Vec::new(),
        });
    }
    Some(StyledNode {
        content: Content::Pseudo,
        specified_values: values,
        children,
    })
}

fn font_size_of(values: &PropertyMap) -> f32 {
    match values.get("font-size") {
        Some(Value::Length(px)) => *px,
        _ => DEFAULT_FONT_SIZE,
    }
}

fn resolve_viewport_units(values: &mut PropertyMap, viewport: Viewport, root_font_size: f32) {
    for value in values.values_mut() {
        if let Some(resolved) = viewport_value(value, viewport, root_font_size) {
            *value = resolved;
        }
    }
}

fn viewport_value(value: &Value, viewport: Viewport, root_font_size: f32) -> Option<Value> {
    match value {
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
        Value::Math(math) => {
            let folded = math.fold_viewport(root_font_size, viewport.width, viewport.height);
            match folded
                .is_px_only()
                .then(|| folded.resolve(0.0, None))
                .flatten()
            {
                Some(px) => Some(Value::Length(px)),
                None => Some(Value::Math(folded)),
            }
        }
        Value::FitContent(limit) => viewport_value(limit, viewport, root_font_size)
            .map(|limit| Value::FitContent(Box::new(limit))),
        _ => None,
    }
}

fn resolve_font_size(values: &mut PropertyMap, parent: &PropertyMap) {
    let parent_size = font_size_of(parent);
    let resolved = match values.get("font-size") {
        Some(Value::Length(px)) => Some(*px),
        Some(Value::Em(em)) => Some(em * parent_size),
        Some(Value::Percent(percent)) => Some(percent / 100.0 * parent_size),
        Some(Value::Calc(linear)) => linear.resolve(parent_size, Some(parent_size)),
        Some(Value::Math(math)) => math.resolve(parent_size, Some(parent_size)),
        _ => None,
    };
    match resolved {
        Some(px) => values.insert("font-size".to_string(), Value::Length(px)),
        None => values.remove("font-size"),
    };
}

const ACTIVE_DECORATIONS: &str = "text-decoration-active";

fn resolve_decorations(values: &mut PropertyMap, parent: &PropertyMap) {
    let keyword_is = |values: &PropertyMap, name: &str, keywords: &[&str]| {
        matches!(
            values.get(name),
            Some(Value::Keyword(keyword)) if keywords.contains(&keyword.as_str())
        )
    };
    let detached = keyword_is(values, "display", &["inline-block"])
        || keyword_is(values, "float", &["left", "right"])
        || keyword_is(values, "position", &["absolute", "fixed"]);
    let mut active = match parent.get(ACTIVE_DECORATIONS) {
        Some(Value::Decorations(list)) if !detached => list.clone(),
        _ => Vec::new(),
    };
    let font_size = font_size_of(values);
    let current = match values.get("color") {
        Some(Value::Color(color)) => Rgba {
            color: *color,
            alpha: 255,
        },
        Some(Value::Rgba(color, alpha)) => Rgba {
            color: *color,
            alpha: *alpha,
        },
        _ => Rgba {
            color: BLACK,
            alpha: 255,
        },
    };
    let color = match values.get("text-decoration-color") {
        Some(Value::Color(color)) => Rgba {
            color: *color,
            alpha: 255,
        },
        Some(Value::Rgba(color, alpha)) => Rgba {
            color: *color,
            alpha: *alpha,
        },
        _ => current,
    };
    let style = match values.get("text-decoration-style") {
        Some(Value::Keyword(keyword)) => decoration::parse_style(keyword),
        _ => None,
    }
    .unwrap_or(LineStyle::Solid);
    let thickness = match values.get("text-decoration-thickness") {
        Some(Value::Length(px)) => Some(*px),
        Some(Value::Em(em)) => Some(em * font_size),
        _ => None,
    };
    if let Some(Value::Keyword(text)) = values.get("text-decoration-line") {
        for line in decoration::parse_lines(text).unwrap_or_default() {
            active.push(Decoration {
                line,
                style,
                color,
                thickness,
            });
        }
    }
    if active.is_empty() {
        values.remove(ACTIVE_DECORATIONS);
    } else {
        values.insert(ACTIVE_DECORATIONS.to_string(), Value::Decorations(active));
    }
}

fn is_current_color(value: Option<&Value>) -> bool {
    matches!(value, Some(Value::Keyword(keyword)) if keyword == "currentcolor")
}

fn resolve_border_styles(values: &mut PropertyMap) {
    for (style, width) in BORDER_STYLES.iter().zip(BORDER_WIDTHS) {
        if matches!(
            values.get(*style),
            Some(Value::Keyword(keyword)) if keyword == "none" || keyword == "hidden"
        ) {
            values.insert(width.to_string(), Value::Length(0.0));
        }
    }
}

fn resolve_current_color(values: &mut PropertyMap, parent: &PropertyMap) {
    if is_current_color(values.get("color")) {
        match parent.get("color") {
            Some(color) => values.insert("color".to_string(), color.clone()),
            None => values.remove("color"),
        };
    }
    let current = values.get("color").cloned().unwrap_or(Value::Color(BLACK));
    for name in ["background-color", "outline-color"]
        .into_iter()
        .chain(BORDER_COLORS)
    {
        if is_current_color(values.get(name)) {
            values.insert(name.to_string(), current.clone());
        }
    }
    for (width, color) in BORDER_WIDTHS.iter().zip(BORDER_COLORS) {
        let visible = matches!(values.get(*width), Some(Value::Length(px)) if *px > 0.0)
            || matches!(values.get(*width), Some(Value::Em(em)) if *em > 0.0);
        if visible && !values.contains_key(color) {
            values.insert(color.to_string(), current.clone());
        }
    }
}

fn resolve_spacing(values: &mut PropertyMap) {
    let font_size = font_size_of(values);
    for name in [
        "letter-spacing",
        "word-spacing",
        "text-indent",
        "outline-width",
        "outline-offset",
    ] {
        if let Some(Value::Em(em)) = values.get(name) {
            let px = em * font_size;
            values.insert(name.to_string(), Value::Length(px));
        }
    }
}

fn resolve_line_height(values: &mut PropertyMap) {
    let font_size = font_size_of(values);
    let resolved = match values.get("line-height") {
        Some(Value::Em(em)) => Some(em * font_size),
        Some(Value::Percent(percent)) => Some(percent / 100.0 * font_size),
        Some(Value::Calc(linear)) => linear.resolve(font_size, Some(font_size)),
        Some(Value::Math(math)) => math.resolve(font_size, Some(font_size)),
        _ => None,
    };
    if let Some(px) = resolved {
        values.insert("line-height".to_string(), Value::Length(px));
    }
}

type CascadeKey = (bool, usize, bool, Specificity);

fn specified_values(
    scope: Scope,
    ancestors: &[Scope],
    context: &StyleContext,
    inherited: PropertyMap,
    pseudo_element: Option<PseudoElement>,
) -> PropertyMap {
    let viewport = context.viewport;
    let layers = &context.stylesheet.layers;
    let mut entries: Vec<(CascadeKey, &Declaration)> = Vec::new();
    for rule in &context.stylesheet.rules {
        if !rule
            .media
            .iter()
            .all(|list| list.matches(viewport.width, viewport.height))
        {
            continue;
        }
        let Some(specificity) = match_rule(scope, ancestors, rule, pseudo_element) else {
            continue;
        };
        let layer = rule
            .layer
            .as_ref()
            .and_then(|path| layers.iter().position(|layer| layer == path));
        let normal_rank = layer.unwrap_or(usize::MAX);
        let important_rank = layer.map_or(0, |index| layers.len() - index);
        for declaration in &rule.declarations {
            entries.push(((false, normal_rank, false, specificity), declaration));
        }
        for declaration in &rule.important {
            entries.push(((true, important_rank, false, specificity), declaration));
        }
    }
    let inline = scope
        .element
        .attributes
        .get("style")
        .filter(|_| pseudo_element.is_none())
        .map(|text| css::parse_declaration_block(text))
        .unwrap_or_default();
    for declaration in &inline.normal {
        entries.push(((false, usize::MAX, true, (0, 0, 0)), declaration));
    }
    for declaration in &inline.important {
        entries.push(((true, usize::MAX, true, (0, 0, 0)), declaration));
    }
    entries.sort_by_key(|entry| entry.0);

    let mut values = inherited;
    for (_, declaration) in entries {
        values.insert(declaration.name.clone(), declaration.value.clone());
    }
    values
}

fn match_rule(
    scope: Scope,
    ancestors: &[Scope],
    rule: &Rule,
    pseudo_element: Option<PseudoElement>,
) -> Option<Specificity> {
    let mut best: Option<Specificity> = None;
    for selector in &rule.selectors {
        if selector::matches(scope, ancestors, selector, pseudo_element) {
            let specificity = selector.specificity();
            if best.is_none_or(|current| specificity > current) {
                best = Some(specificity);
            }
        }
    }
    best
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
            Some(Value::Keyword(keyword)) if keyword == "inline-block" => Display::InlineBlock,
            Some(Value::Keyword(keyword)) if keyword == "list-item" => Display::ListItem,
            Some(Value::Keyword(keyword)) if keyword == "grid" || keyword == "inline-grid" => {
                Display::Grid
            }
            Some(Value::Keyword(keyword)) if keyword == "table" => Display::Table,
            Some(Value::Keyword(keyword))
                if matches!(
                    keyword.as_str(),
                    "table-row-group" | "table-header-group" | "table-footer-group"
                ) =>
            {
                Display::TableRowGroup
            }
            Some(Value::Keyword(keyword)) if keyword == "table-row" => Display::TableRow,
            Some(Value::Keyword(keyword)) if keyword == "table-cell" => Display::TableCell,
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

    pub fn text_transform(&self) -> TextTransform {
        match self.value("text-transform") {
            Some(Value::Keyword(keyword)) if keyword == "uppercase" => TextTransform::Uppercase,
            Some(Value::Keyword(keyword)) if keyword == "lowercase" => TextTransform::Lowercase,
            Some(Value::Keyword(keyword)) if keyword == "capitalize" => TextTransform::Capitalize,
            _ => TextTransform::None,
        }
    }

    pub fn letter_spacing(&self) -> f32 {
        match self.value("letter-spacing") {
            Some(Value::Length(px)) => *px,
            _ => 0.0,
        }
    }

    pub fn word_spacing(&self) -> f32 {
        match self.value("word-spacing") {
            Some(Value::Length(px)) => *px,
            _ => 0.0,
        }
    }

    pub fn white_space(&self) -> WhiteSpace {
        match self.value("white-space") {
            Some(Value::Keyword(keyword)) if keyword == "nowrap" => WhiteSpace::Nowrap,
            Some(Value::Keyword(keyword)) if keyword == "pre" => WhiteSpace::Pre,
            Some(Value::Keyword(keyword)) if keyword == "pre-wrap" => WhiteSpace::PreWrap,
            Some(Value::Keyword(keyword)) if keyword == "pre-line" => WhiteSpace::PreLine,
            _ => WhiteSpace::Normal,
        }
    }

    pub fn text_align(&self) -> TextAlign {
        match self.value("text-align") {
            Some(Value::Keyword(keyword)) if keyword == "center" => TextAlign::Center,
            Some(Value::Keyword(keyword)) if keyword == "right" || keyword == "end" => {
                TextAlign::Right
            }
            Some(Value::Keyword(keyword)) if keyword == "justify" => TextAlign::Justify,
            _ => TextAlign::Left,
        }
    }

    pub fn border_color(&self, side: usize) -> Option<Rgba> {
        self.rgba(BORDER_COLORS[side])
    }

    pub fn border_style(&self, side: usize) -> StrokeStyle {
        stroke_style(self.keyword(BORDER_STYLES[side]))
    }

    pub fn border_edge(&self, side: usize) -> Edge {
        let keyword = self.keyword(BORDER_STYLES[side]);
        let rank = match keyword {
            Some("double") => 8,
            Some("solid") => 7,
            Some("dashed") => 6,
            Some("dotted") => 5,
            Some("ridge") => 4,
            Some("outset") => 3,
            Some("groove") => 2,
            Some("inset") => 1,
            _ => 0,
        };
        Edge {
            width: if rank > 0 {
                self.px(BORDER_WIDTHS[side], 0.0)
            } else {
                0.0
            },
            rank,
            hidden: keyword == Some("hidden"),
            priority: 0,
            stroke: self.border_style(side),
            color: self.border_color(side),
        }
    }

    pub fn outline(&self) -> Option<Outline> {
        if matches!(
            self.keyword("outline-style"),
            None | Some("none" | "hidden")
        ) {
            return None;
        }
        let width = match self.value("outline-width") {
            Some(Value::Length(px)) => *px,
            Some(Value::Keyword(keyword)) if keyword == "thin" => 1.0,
            Some(Value::Keyword(keyword)) if keyword == "thick" => 5.0,
            _ => 3.0,
        };
        if width <= 0.0 {
            return None;
        }
        let offset = match self.value("outline-offset") {
            Some(Value::Length(px)) => *px,
            _ => 0.0,
        };
        let color = self.rgba("outline-color").unwrap_or(Rgba {
            color: self.text_color(),
            alpha: self.text_alpha(),
        });
        Some(Outline {
            style: stroke_style(self.keyword("outline-style")),
            width,
            offset,
            color,
        })
    }

    pub fn break_all(&self) -> bool {
        self.keyword("word-break") == Some("break-all")
    }

    pub fn break_long(&self) -> bool {
        self.break_all()
            || self.keyword("word-break") == Some("break-word")
            || matches!(
                self.keyword("overflow-wrap"),
                Some("break-word" | "anywhere")
            )
    }

    pub fn ellipsis(&self) -> bool {
        self.keyword("text-overflow") == Some("ellipsis") && self.establishes_clip()
    }

    pub fn text_indent(&self) -> Indent {
        match self.value("text-indent") {
            Some(Value::Length(px)) => Indent {
                px: *px,
                percent: 0.0,
            },
            Some(Value::Percent(percent)) => Indent {
                px: 0.0,
                percent: *percent,
            },
            _ => Indent::default(),
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
            Some(Value::Keyword(keyword)) if keyword == "sticky" => Position::Sticky,
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

    pub fn content_size(&self, name: &str, percent_base: f32) -> Option<ContentSize> {
        match self.value(name) {
            Some(Value::Keyword(keyword)) => match keyword.as_str() {
                "min-content" => Some(ContentSize::Min),
                "max-content" => Some(ContentSize::Max),
                "fit-content" => Some(ContentSize::Fit(None)),
                _ => None,
            },
            Some(Value::FitContent(limit)) => Some(ContentSize::Fit(self.length_from(
                Some(limit),
                None,
                Some(percent_base),
            ))),
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

    pub fn decorations(&self) -> &[Decoration] {
        if !self.visible() {
            return &[];
        }
        match self.value(ACTIVE_DECORATIONS) {
            Some(Value::Decorations(list)) => list,
            _ => &[],
        }
    }

    pub fn box_shadows(&self) -> &[Shadow] {
        match self.value("box-shadow") {
            Some(Value::Shadows(shadows)) => shadows,
            _ => &[],
        }
    }

    pub fn border_radii(&self, width: f32, height: f32) -> Radii {
        let corner = |name: &str| {
            (
                self.px(&format!("{}-x", name), width),
                self.px(&format!("{}-y", name), height),
            )
        };
        Radii {
            corners: [
                corner("border-top-left-radius"),
                corner("border-top-right-radius"),
                corner("border-bottom-right-radius"),
                corner("border-bottom-left-radius"),
            ],
        }
        .fit(width, height)
    }

    pub fn establishes_clip(&self) -> bool {
        self.overflow_is(&["hidden", "clip", "auto", "scroll"])
    }

    pub fn scroll_axes(&self) -> (bool, bool) {
        let scrolls = |name: &str| matches!(self.keyword(name), Some("auto" | "scroll"));
        (scrolls("overflow-x"), scrolls("overflow-y"))
    }

    pub fn scrollbars_always(&self) -> (bool, bool) {
        (
            self.keyword("overflow-x") == Some("scroll"),
            self.keyword("overflow-y") == Some("scroll"),
        )
    }

    pub fn scrollbar_colors(&self) -> (Color, Color) {
        (
            self.color("scrollbar-thumb-color")
                .unwrap_or(SCROLLBAR_THUMB),
            self.color("scrollbar-track-color")
                .unwrap_or(SCROLLBAR_TRACK),
        )
    }

    pub fn scrollbar_width(&self) -> f32 {
        match self.keyword("scrollbar-width") {
            Some("none") => 0.0,
            Some("thin") => 4.0,
            _ => 8.0,
        }
    }

    fn overflow_is(&self, keywords: &[&str]) -> bool {
        ["overflow-x", "overflow-y"].iter().any(|name| {
            matches!(
                self.value(name),
                Some(Value::Keyword(keyword)) if keywords.contains(&keyword.as_str())
            )
        })
    }

    pub fn establishes_block_context(&self) -> bool {
        if self.float().is_some()
            || self.is_out_of_flow()
            || matches!(
                self.display(),
                Display::Flex
                    | Display::Grid
                    | Display::InlineBlock
                    | Display::Table
                    | Display::TableCell
            )
        {
            return true;
        }
        self.overflow_is(&["hidden", "auto", "scroll"])
    }

    pub fn background(&self) -> Option<Color> {
        self.color("background-color")
    }

    pub fn opacity(&self) -> f32 {
        match self.value("opacity") {
            Some(Value::Number(number)) => number.clamp(0.0, 1.0),
            Some(Value::Percent(percent)) => (percent / 100.0).clamp(0.0, 1.0),
            Some(Value::Length(_)) => 0.0,
            _ => 1.0,
        }
    }

    pub fn transform_matrix(&self, x: f32, y: f32, width: f32, height: f32) -> Option<Matrix> {
        let Some(Value::Transform(functions)) = self.value("transform") else {
            return None;
        };
        let origin = |name: &str, size: f32| match self.value(name) {
            Some(Value::Percent(percent)) => percent / 100.0 * size,
            Some(_) => self.px(name, 0.0),
            None => size / 2.0,
        };
        let origin = (
            x + origin("transform-origin-x", width),
            y + origin("transform-origin-y", height),
        );
        Some(transform::compose(
            functions,
            origin,
            width,
            height,
            self.font_size(),
        ))
    }

    pub fn has_effect(&self) -> bool {
        self.opacity() < 1.0 || matches!(self.value("transform"), Some(Value::Transform(_)))
    }

    pub fn grid_tracks(&self, name: &str) -> Option<&TrackList> {
        match self.value(name) {
            Some(Value::Tracks(tracks)) => Some(tracks),
            _ => None,
        }
    }

    pub fn grid_areas(&self) -> Option<&Vec<Vec<String>>> {
        match self.value("grid-template-areas") {
            Some(Value::Areas(areas)) => Some(areas),
            _ => None,
        }
    }

    pub fn grid_line(&self, name: &str) -> Line {
        match self.value(name) {
            Some(Value::GridLine(line)) => line.clone(),
            _ => Line::Auto,
        }
    }

    pub fn grid_flow_column(&self) -> bool {
        self.keyword("grid-auto-flow") == Some("column")
    }

    pub fn grid_dense(&self) -> bool {
        self.keyword("grid-auto-dense") == Some("dense")
    }

    pub fn justify_items(&self) -> Align {
        match self.keyword("justify-items") {
            Some(keyword) => StyledNode::align_keyword(keyword).unwrap_or(Align::Stretch),
            None => Align::Stretch,
        }
    }

    pub fn justify_self(&self) -> Option<Align> {
        StyledNode::align_keyword(self.keyword("justify-self")?)
    }

    pub fn content_distribution(&self, name: &str) -> Option<Justify> {
        match self.keyword(name)? {
            "start" | "flex-start" | "left" => Some(Justify::Start),
            "end" | "flex-end" | "right" => Some(Justify::End),
            "center" => Some(Justify::Center),
            "space-between" => Some(Justify::SpaceBetween),
            "space-around" => Some(Justify::SpaceAround),
            "space-evenly" => Some(Justify::SpaceEvenly),
            _ => None,
        }
    }

    pub fn inline_vertical_align(&self) -> VerticalAlign {
        let size = self.font_size();
        match self.value("vertical-align") {
            Some(Value::Keyword(keyword)) => match keyword.as_str() {
                "sub" => VerticalAlign::Raise(-(size / 5.0 + 1.0)),
                "super" => VerticalAlign::Raise(size / 3.0 + 1.0),
                "middle" => VerticalAlign::Middle,
                "text-top" => VerticalAlign::TextTop,
                "text-bottom" => VerticalAlign::TextBottom,
                "top" => VerticalAlign::Top,
                "bottom" => VerticalAlign::Bottom,
                _ => VerticalAlign::Raise(0.0),
            },
            Some(Value::Length(px)) => VerticalAlign::Raise(*px),
            Some(Value::Em(em)) => VerticalAlign::Raise(em * size),
            Some(Value::Percent(percent)) => {
                VerticalAlign::Raise(percent / 100.0 * self.line_height().unwrap_or(size * 1.2))
            }
            _ => VerticalAlign::Raise(0.0),
        }
    }

    pub fn vertical_align(&self) -> Option<&str> {
        self.keyword("vertical-align")
    }

    pub fn border_collapse(&self) -> bool {
        self.keyword("border-collapse") == Some("collapse")
    }

    pub fn table_layout_fixed(&self) -> bool {
        self.keyword("table-layout") == Some("fixed")
            && !matches!(self.value("width"), None | Some(Value::Keyword(_)))
    }

    pub fn list_style_type(&self) -> &str {
        self.keyword("list-style-type").unwrap_or("disc")
    }

    pub fn font_families(&self) -> &[String] {
        match self.value("font-family") {
            Some(Value::Families(families)) => families,
            _ => &[],
        }
    }

    pub fn font_bold(&self) -> bool {
        match self.value("font-weight") {
            Some(Value::Keyword(keyword)) => keyword == "bold" || keyword == "bolder",
            Some(Value::Number(weight)) => *weight >= 600.0,
            _ => false,
        }
    }

    pub fn font_italic(&self) -> bool {
        matches!(self.keyword("font-style"), Some("italic") | Some("oblique"))
    }

    pub fn background_layers(&self) -> usize {
        match self.value("background-image") {
            Some(Value::List(items)) => items.len(),
            _ => 1,
        }
    }

    fn layer(&self, name: &str, index: usize) -> Option<&Value> {
        match self.value(name) {
            Some(Value::List(items)) if !items.is_empty() => Some(&items[index % items.len()]),
            value => value,
        }
    }

    fn layer_keyword(&self, name: &str, index: usize) -> Option<&str> {
        match self.layer(name, index) {
            Some(Value::Keyword(keyword)) => Some(keyword.as_str()),
            _ => None,
        }
    }

    pub fn background_image(&self, index: usize) -> Option<&str> {
        match self.layer("background-image", index) {
            Some(Value::Url(url)) => Some(url),
            _ => None,
        }
    }

    pub fn background_repeat(&self, index: usize) -> (bool, bool) {
        match self.layer_keyword("background-repeat", index) {
            Some("repeat-x") => (true, false),
            Some("repeat-y") => (false, true),
            Some("no-repeat") => (false, false),
            _ => (true, true),
        }
    }

    pub fn background_fixed(&self, index: usize) -> bool {
        self.layer_keyword("background-attachment", index) == Some("fixed")
    }

    pub fn background_local(&self, index: usize) -> bool {
        self.layer_keyword("background-attachment", index) == Some("local")
    }

    pub fn background_clip(&self, index: usize) -> BackgroundBox {
        background_box(self.layer_keyword("background-clip", index))
    }

    pub fn background_origin(&self, index: usize) -> BackgroundBox {
        match self.layer_keyword("background-origin", index) {
            None => BackgroundBox::Padding,
            keyword => background_box(keyword),
        }
    }

    pub fn background_size(
        &self,
        index: usize,
        area: (f32, f32),
        natural: (f32, f32),
    ) -> (f32, f32) {
        let (area_width, area_height) = area;
        let (natural_width, natural_height) = natural;
        let scale = match self.layer_keyword("background-size", index) {
            Some("cover") => Some((area_width / natural_width).max(area_height / natural_height)),
            Some("contain") => Some((area_width / natural_width).min(area_height / natural_height)),
            _ => None,
        };
        if let Some(scale) = scale {
            return (natural_width * scale, natural_height * scale);
        }
        let width = self.length_from(
            self.layer("background-size-width", index),
            None,
            Some(area_width),
        );
        let height = self.length_from(
            self.layer("background-size-height", index),
            None,
            Some(area_height),
        );
        match (width, height) {
            (Some(width), Some(height)) => (width, height),
            (Some(width), None) => (width, natural_height * width / natural_width),
            (None, Some(height)) => (natural_width * height / natural_height, height),
            (None, None) => natural,
        }
    }

    pub fn background_offset(&self, name: &str, index: usize, free: f32) -> f32 {
        match self.layer(name, index) {
            Some(Value::Percent(percent)) => percent / 100.0 * free,
            value => self.length_from(value, Some(0.0), Some(0.0)).unwrap_or(0.0),
        }
    }

    pub fn rgba(&self, name: &str) -> Option<Rgba> {
        match self.value(name) {
            Some(Value::Color(color)) => Some(Rgba {
                color: *color,
                alpha: 255,
            }),
            Some(Value::Rgba(color, alpha)) => Some(Rgba {
                color: *color,
                alpha: *alpha,
            }),
            _ => None,
        }
    }

    pub fn text_color(&self) -> Color {
        self.rgba("color").map_or(BLACK, |rgba| rgba.color)
    }

    pub fn text_alpha(&self) -> u8 {
        if !self.visible() {
            return 0;
        }
        self.rgba("color").map_or(255, |rgba| rgba.alpha)
    }

    pub fn collapsed(&self) -> bool {
        self.keyword("visibility") == Some("collapse")
    }

    pub fn visible(&self) -> bool {
        !matches!(self.keyword("visibility"), Some("hidden" | "collapse"))
    }

    pub fn background_gradient(&self, index: usize) -> Option<&Gradient> {
        match self.layer("background-image", index) {
            Some(Value::Gradient(gradient)) => Some(gradient),
            _ => None,
        }
    }

    pub fn length_or_auto(
        &self,
        name: &str,
        default: Option<f32>,
        percent_base: Option<f32>,
    ) -> Option<f32> {
        let length = self.length_from(self.value(name), default, percent_base);
        let specified = !matches!(self.value(name), None | Some(Value::Keyword(_)));
        match length {
            Some(length) if specified && self.is_border_box() => {
                Some((length - self.box_extras(name, percent_base)).max(0.0))
            }
            length => length,
        }
    }

    pub fn object_rect(&self, rect: Rect, natural: (f32, f32)) -> Rect {
        let (natural_width, natural_height) = natural;
        if natural_width <= 0.0 || natural_height <= 0.0 {
            return rect;
        }
        let contain = (rect.width / natural_width).min(rect.height / natural_height);
        let cover = (rect.width / natural_width).max(rect.height / natural_height);
        let scale = match self.keyword("object-fit") {
            Some("contain") => contain,
            Some("cover") => cover,
            Some("none") => 1.0,
            Some("scale-down") => contain.min(1.0),
            _ => return rect,
        };
        let width = natural_width * scale;
        let height = natural_height * scale;
        let offset = |name: &str, free: f32| match self.value(name) {
            None => free / 2.0,
            Some(Value::Percent(percent)) => percent / 100.0 * free,
            value => self.length_from(value, Some(0.0), Some(0.0)).unwrap_or(0.0),
        };
        Rect {
            x: rect.x + offset("object-position-x", rect.width - width),
            y: rect.y + offset("object-position-y", rect.height - height),
            width,
            height,
        }
    }

    pub fn aspect_ratio(&self) -> Option<f32> {
        match self.value("aspect-ratio") {
            Some(Value::Number(ratio)) => Some(*ratio),
            Some(Value::List(parts)) => match parts.as_slice() {
                [_, Value::Number(ratio)] => Some(*ratio),
                _ => None,
            },
            _ => None,
        }
    }

    pub fn ratio_prefers_natural(&self) -> bool {
        matches!(self.value("aspect-ratio"), Some(Value::List(_)))
    }

    pub fn is_border_box(&self) -> bool {
        self.keyword("box-sizing") == Some("border-box")
    }

    fn box_extras(&self, name: &str, percent_base: Option<f32>) -> f32 {
        let (borders, paddings, base) = match name {
            "width" | "min-width" | "max-width" => (
                ["border-left-width", "border-right-width"],
                ["padding-left", "padding-right"],
                percent_base.unwrap_or(0.0),
            ),
            "height" | "min-height" | "max-height" => (
                ["border-top-width", "border-bottom-width"],
                ["padding-top", "padding-bottom"],
                0.0,
            ),
            _ => return 0.0,
        };
        borders
            .into_iter()
            .chain(paddings)
            .map(|name| self.px(name, base))
            .sum()
    }

    fn length_from(
        &self,
        value: Option<&Value>,
        default: Option<f32>,
        percent_base: Option<f32>,
    ) -> Option<f32> {
        match value {
            Some(Value::Length(px)) => Some(*px),
            Some(Value::Em(em)) => Some(em * self.font_size()),
            Some(Value::Percent(percent)) => percent_base.map(|base| percent / 100.0 * base),
            Some(Value::Calc(linear)) => linear.resolve(self.font_size(), percent_base),
            Some(Value::Math(math)) => math.resolve(self.font_size(), percent_base),
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
