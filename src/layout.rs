// SPDX-License-Identifier: GPL-3.0-only

use fontdue::Font;

use crate::inline::{self, Fragment, Run};
use crate::style::{Content, Display, StyledNode};

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
    Anonymous(Vec<Run<'a>>),
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
        collect_child(child, &mut children, &mut runs);
    }
    flush_runs(&mut children, &mut runs);
    LayoutBox {
        dimensions: Dimensions::default(),
        kind: BoxKind::Block(style),
        children,
        fragments: Vec::new(),
    }
}

fn collect_child<'a>(
    node: &'a StyledNode<'a>,
    children: &mut Vec<LayoutBox<'a>>,
    runs: &mut Vec<Run<'a>>,
) {
    match node.display() {
        Display::None => {}
        Display::Block => {
            flush_runs(children, runs);
            children.push(build_block_box(node));
        }
        Display::Inline => match node.content {
            Content::Text(text) => runs.push(Run {
                text,
                color: node.text_color(),
                font_size: node.font_size(),
            }),
            Content::Element(_) => {
                for child in &node.children {
                    collect_child(child, children, runs);
                }
            }
        },
    }
}

fn flush_runs<'a>(children: &mut Vec<LayoutBox<'a>>, runs: &mut Vec<Run<'a>>) {
    let mut has_text = false;
    for run in runs.iter() {
        if !run.text.trim().is_empty() {
            has_text = true;
        }
    }
    if has_text {
        children.push(LayoutBox {
            dimensions: Dimensions::default(),
            kind: BoxKind::Anonymous(std::mem::take(runs)),
            children: Vec::new(),
            fragments: Vec::new(),
        });
    } else {
        runs.clear();
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
                self.calculate_height(style);
            }
            BoxKind::Anonymous(runs) => {
                let content = &mut self.dimensions.content;
                content.x = containing.content.x;
                content.y = containing.content.y + containing.content.height;
                content.width = containing.content.width;
                let (fragments, height) =
                    inline::layout_lines(runs, content.width, content.x, content.y, font);
                content.height = height;
                self.fragments = fragments;
            }
        }
    }

    fn calculate_width(&mut self, style: &StyledNode, containing: &Dimensions) {
        let width = style.length_or_auto("width", None);
        let mut margin_left = style.length_or_auto("margin-left", Some(0.0));
        let mut margin_right = style.length_or_auto("margin-right", Some(0.0));

        let border_left = style.px("border-left-width");
        let border_right = style.px("border-right-width");
        let padding_left = style.px("padding-left");
        let padding_right = style.px("padding-right");

        let total = border_left
            + border_right
            + padding_left
            + padding_right
            + width.unwrap_or(0.0)
            + margin_left.unwrap_or(0.0)
            + margin_right.unwrap_or(0.0);

        if width.is_some() && total > containing.content.width {
            margin_left = Some(margin_left.unwrap_or(0.0));
            margin_right = Some(margin_right.unwrap_or(0.0));
        }

        let underflow = containing.content.width - total;

        let (width, margin_left, margin_right) = match (width, margin_left, margin_right) {
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
        };

        let d = &mut self.dimensions;
        d.content.width = width;
        d.padding.left = padding_left;
        d.padding.right = padding_right;
        d.border.left = border_left;
        d.border.right = border_right;
        d.margin.left = margin_left;
        d.margin.right = margin_right;
    }

    fn calculate_position(&mut self, style: &StyledNode, containing: &Dimensions) {
        let d = &mut self.dimensions;

        d.margin.top = style.px("margin-top");
        d.margin.bottom = style.px("margin-bottom");
        d.border.top = style.px("border-top-width");
        d.border.bottom = style.px("border-bottom-width");
        d.padding.top = style.px("padding-top");
        d.padding.bottom = style.px("padding-bottom");

        d.content.x = containing.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing.content.y
            + containing.content.height
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    fn layout_children(&mut self, font: &Font) {
        for child in &mut self.children {
            child.layout(&self.dimensions, font);
            self.dimensions.content.height += child.dimensions.margin_box().height;
        }
    }

    fn calculate_height(&mut self, style: &StyledNode) {
        if let Some(height) = style.length_or_auto("height", None) {
            self.dimensions.content.height = height;
        }
    }
}
