// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::css::Color;
use crate::inline::{self, Fragment, InlineContent, Run};
use crate::style::{Content, Display, StyledNode, TextAlign};

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct EdgeSizes {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub struct Dimensions {
    pub content: Rect,
    pub padding: EdgeSizes,
    pub border: EdgeSizes,
    pub margin: EdgeSizes,
}

#[derive(Debug)]
pub enum BoxKind<'a> {
    Block(&'a StyledNode<'a>),
    Anonymous(InlineContent<'a>),
}

#[derive(Debug)]
pub struct LayoutBox<'a> {
    pub dimensions: Dimensions,
    pub kind: BoxKind<'a>,
    pub children: Vec<LayoutBox<'a>>,
    pub fragments: Vec<Fragment>,
}

impl Rect {
    fn expanded_by(&self, edge: &EdgeSizes) -> Rect {
        Rect {
            x: self.x - edge.left,
            y: self.y - edge.top,
            width: self.width + edge.left + edge.right,
            height: self.height + edge.top + edge.bottom,
        }
    }
}

impl Dimensions {
    pub fn padding_box(&self) -> Rect {
        self.content.expanded_by(&self.padding)
    }

    pub fn border_box(&self) -> Rect {
        self.padding_box().expanded_by(&self.border)
    }

    pub fn margin_box(&self) -> Rect {
        self.border_box().expanded_by(&self.margin)
    }
}

pub fn layout_tree<'a>(
    root: &'a StyledNode<'a>,
    viewport_width: f32,
    font: &Font,
) -> LayoutBox<'a> {
    let mut layout_root = build_block_box(root);
    let mut viewport = Dimensions::default();
    viewport.content.width = viewport_width;
    layout_root.layout(&viewport, font);
    layout_root
}

fn build_block_box<'a>(style: &'a StyledNode<'a>) -> LayoutBox<'a> {
    let mut children = Vec::new();
    let mut runs = Vec::new();
    for child in &style.children {
        collect_child(child, None, &mut children, &mut runs);
    }
    flush_runs(style.text_align(), &mut children, &mut runs);
    LayoutBox {
        dimensions: Dimensions::default(),
        kind: BoxKind::Block(style),
        children,
        fragments: Vec::new(),
    }
}

fn collect_child<'a>(
    node: &'a StyledNode<'a>,
    background: Option<Color>,
    children: &mut Vec<LayoutBox<'a>>,
    runs: &mut Vec<Run<'a>>,
) {
    match node.display() {
        Display::None => {}
        Display::Block => {
            flush_runs(node.text_align(), children, runs);
            children.push(build_block_box(node));
        }
        Display::Inline => match node.content {
            Content::Text(text) => runs.push(Run {
                text,
                color: node.text_color(),
                font_size: node.font_size(),
                line_height: node.line_height(),
                background,
            }),
            Content::Element(_) => {
                let background = node.background().or(background);
                for child in &node.children {
                    collect_child(child, background, children, runs);
                }
            }
        },
    }
}

fn flush_runs<'a>(align: TextAlign, children: &mut Vec<LayoutBox<'a>>, runs: &mut Vec<Run<'a>>) {
    let mut has_text = false;
    for run in runs.iter() {
        if !run.text.trim().is_empty() {
            has_text = true;
        }
    }
    if has_text {
        children.push(LayoutBox {
            dimensions: Dimensions::default(),
            kind: BoxKind::Anonymous(InlineContent {
                runs: std::mem::take(runs),
                align,
            }),
            children: Vec::new(),
            fragments: Vec::new(),
        });
    } else {
        runs.clear();
    }
}

fn combine_margins(a: f32, b: f32) -> f32 {
    if a >= 0.0 && b >= 0.0 {
        a.max(b)
    } else if a <= 0.0 && b <= 0.0 {
        a.min(b)
    } else {
        a + b
    }
}

fn resolve_horizontal(
    width: Option<f32>,
    margin_left: Option<f32>,
    margin_right: Option<f32>,
    fixed: f32,
    container: f32,
) -> (f32, f32, f32) {
    let mut margin_left = margin_left;
    let mut margin_right = margin_right;
    let total =
        fixed + width.unwrap_or(0.0) + margin_left.unwrap_or(0.0) + margin_right.unwrap_or(0.0);

    if width.is_some() && total > container {
        margin_left = Some(margin_left.unwrap_or(0.0));
        margin_right = Some(margin_right.unwrap_or(0.0));
    }

    let underflow = container - total;

    match (width, margin_left, margin_right) {
        (Some(w), Some(l), Some(r)) => (w, l, r + underflow),
        (Some(w), Some(l), None) => (w, l, underflow),
        (Some(w), None, Some(r)) => (w, underflow, r),
        (Some(w), None, None) => (w, underflow / 2.0, underflow / 2.0),
        (None, l, r) => {
            let l = l.unwrap_or(0.0);
            let r = r.unwrap_or(0.0);
            if underflow >= 0.0 {
                (underflow, l, r)
            } else {
                (0.0, l, r + underflow)
            }
        }
    }
}

impl<'a> LayoutBox<'a> {
    fn layout(&mut self, containing: &Dimensions, font: &Font) {
        match &self.kind {
            BoxKind::Block(style) => {
                let style = *style;
                self.calculate_width(style, containing);
                self.calculate_position(style, containing);
                self.layout_children(font);
                self.collapse_bottom_margin(style);
                self.calculate_height(style);
            }
            BoxKind::Anonymous(content) => {
                let area = &mut self.dimensions.content;
                area.x = containing.content.x;
                area.y = containing.content.y + containing.content.height;
                area.width = containing.content.width;
                let (fragments, height) =
                    inline::layout_lines(content, area.width, area.x, area.y, font);
                area.height = height;
                self.fragments = fragments;
            }
        }
    }

    fn collapsed_top_margin(&self, base: f32) -> f32 {
        let BoxKind::Block(style) = &self.kind else {
            return 0.0;
        };
        let own = style.px("margin-top", base);
        if style.px("border-top-width", base) != 0.0 || style.px("padding-top", base) != 0.0 {
            return own;
        }
        match self.children.first() {
            Some(first) if matches!(first.kind, BoxKind::Block(_)) => {
                combine_margins(own, first.collapsed_top_margin(base))
            }
            _ => own,
        }
    }

    fn calculate_width(&mut self, style: &StyledNode, containing: &Dimensions) {
        let container = containing.content.width;
        let width = style.length_or_auto("width", None, Some(container));
        let margin_left = style.length_or_auto("margin-left", Some(0.0), Some(container));
        let margin_right = style.length_or_auto("margin-right", Some(0.0), Some(container));

        let border_left = style.px("border-left-width", container);
        let border_right = style.px("border-right-width", container);
        let padding_left = style.px("padding-left", container);
        let padding_right = style.px("padding-right", container);
        let fixed = border_left + border_right + padding_left + padding_right;

        let (mut w, mut ml, mut mr) =
            resolve_horizontal(width, margin_left, margin_right, fixed, container);
        if let Some(max) = style.length_or_auto("max-width", None, Some(container)) {
            if w > max {
                (w, ml, mr) =
                    resolve_horizontal(Some(max), margin_left, margin_right, fixed, container);
            }
        }
        if let Some(min) = style.length_or_auto("min-width", None, Some(container)) {
            if w < min {
                (w, ml, mr) =
                    resolve_horizontal(Some(min), margin_left, margin_right, fixed, container);
            }
        }

        let d = &mut self.dimensions;
        d.content.width = w;
        d.padding.left = padding_left;
        d.padding.right = padding_right;
        d.border.left = border_left;
        d.border.right = border_right;
        d.margin.left = ml;
        d.margin.right = mr;
    }

    fn calculate_position(&mut self, style: &StyledNode, containing: &Dimensions) {
        let base = containing.content.width;
        let margin_top = self.collapsed_top_margin(base);
        let d = &mut self.dimensions;

        d.margin.top = margin_top;
        d.margin.bottom = style.px("margin-bottom", base);
        d.border.top = style.px("border-top-width", base);
        d.border.bottom = style.px("border-bottom-width", base);
        d.padding.top = style.px("padding-top", base);
        d.padding.bottom = style.px("padding-bottom", base);

        d.content.x = containing.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing.content.y
            + containing.content.height
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_children(&mut self, font: &Font) {
        let base = self.dimensions.content.width;
        let top_open = self.dimensions.border.top == 0.0 && self.dimensions.padding.top == 0.0;
        let mut previous_bottom: Option<f32> = None;

        for (index, child) in self.children.iter_mut().enumerate() {
            let is_block = matches!(child.kind, BoxKind::Block(_));
            let mut overlap = 0.0;
            if is_block {
                let top = child.collapsed_top_margin(base);
                if index == 0 {
                    if top_open {
                        overlap = top;
                    }
                } else if let Some(bottom) = previous_bottom {
                    overlap = bottom + top - combine_margins(bottom, top);
                }
            }

            let mut containing = self.dimensions;
            containing.content.height -= overlap;
            child.layout(&containing, font);
            self.dimensions.content.height += child.dimensions.margin_box().height - overlap;

            previous_bottom = if is_block {
                Some(child.dimensions.margin.bottom)
            } else {
                None
            };
        }
    }

    fn collapse_bottom_margin(&mut self, style: &StyledNode) {
        let base = self.dimensions.content.width;
        let closed = style.length_or_auto("height", None, None).is_some()
            || self.dimensions.border.bottom != 0.0
            || self.dimensions.padding.bottom != 0.0
            || style.px("border-bottom-width", base) != 0.0;
        if closed {
            return;
        }
        if let Some(last) = self.children.last() {
            if matches!(last.kind, BoxKind::Block(_)) {
                let bottom = last.dimensions.margin.bottom;
                self.dimensions.content.height -= bottom;
                self.dimensions.margin.bottom =
                    combine_margins(self.dimensions.margin.bottom, bottom);
            }
        }
    }

    fn calculate_height(&mut self, style: &StyledNode) {
        if let Some(height) = style.length_or_auto("height", None, None) {
            self.dimensions.content.height = height;
        }
        if let Some(max) = style.length_or_auto("max-height", None, None) {
            self.dimensions.content.height = self.dimensions.content.height.min(max);
        }
        if let Some(min) = style.length_or_auto("min-height", None, None) {
            self.dimensions.content.height = self.dimensions.content.height.max(min);
        }
    }
}
