// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::rc::Rc;

use fontdue::Font;

use crate::css::Color;
use crate::dom::ElementData;
use crate::inline::{self, Fragment, ImageFragment, ImageRun, InlineContent, InlineItem, Run};
use crate::picture::Picture;
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

pub type ImageMap = HashMap<String, Rc<Picture>>;

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
    pub images: Vec<ImageFragment>,
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
    images: &ImageMap,
) -> LayoutBox<'a> {
    let mut layout_root = build_block_box(root, images);
    let mut viewport = Dimensions::default();
    viewport.content.width = viewport_width;
    layout_root.layout(&viewport, font, images);
    layout_root
}

impl<'a> LayoutBox<'a> {
    fn new(kind: BoxKind<'a>, children: Vec<LayoutBox<'a>>) -> LayoutBox<'a> {
        LayoutBox {
            dimensions: Dimensions::default(),
            kind,
            children,
            fragments: Vec::new(),
            images: Vec::new(),
        }
    }
}

fn build_block_box<'a>(style: &'a StyledNode<'a>, images: &ImageMap) -> LayoutBox<'a> {
    let mut children = Vec::new();
    let mut items = Vec::new();
    for child in &style.children {
        collect_child(child, None, images, &mut children, &mut items);
    }
    flush_items(style.text_align(), &mut children, &mut items);
    LayoutBox::new(BoxKind::Block(style), children)
}

fn collect_child<'a>(
    node: &'a StyledNode<'a>,
    background: Option<Color>,
    images: &ImageMap,
    children: &mut Vec<LayoutBox<'a>>,
    items: &mut Vec<InlineItem<'a>>,
) {
    match node.display() {
        Display::None => {}
        Display::Block => {
            flush_items(node.text_align(), children, items);
            children.push(build_block_box(node, images));
        }
        Display::Inline => match node.content {
            Content::Text(text) => items.push(InlineItem::Text(Run {
                text,
                color: node.text_color(),
                font_size: node.font_size(),
                line_height: node.line_height(),
                background,
            })),
            Content::Element(element) if element.tag_name == "img" => {
                let picture = find_picture(element, images);
                let (width, height) = replaced_size(node, element, picture.as_deref(), None);
                items.push(InlineItem::Image(ImageRun {
                    picture,
                    width,
                    height,
                    font_size: node.font_size(),
                }));
            }
            Content::Element(_) => {
                let background = node.background().or(background);
                for child in &node.children {
                    collect_child(child, background, images, children, items);
                }
            }
        },
    }
}

fn find_picture(element: &ElementData, images: &ImageMap) -> Option<Rc<Picture>> {
    images.get(element.attributes.get("src")?).cloned()
}

fn attribute_px(element: &ElementData, name: &str) -> Option<f32> {
    let value = element.attributes.get(name)?.trim();
    let number = value.strip_suffix("px").unwrap_or(value);
    number
        .parse::<f32>()
        .ok()
        .filter(|size| size.is_finite() && *size >= 0.0)
}

fn replaced_size(
    style: &StyledNode,
    element: &ElementData,
    picture: Option<&Picture>,
    percent_base: Option<f32>,
) -> (f32, f32) {
    let width = style
        .length_or_auto("width", None, percent_base)
        .or(attribute_px(element, "width"));
    let height = style
        .length_or_auto("height", None, None)
        .or(attribute_px(element, "height"));
    let (natural_width, natural_height) = match picture {
        Some(picture) => (picture.width as f32, picture.height as f32),
        None => (0.0, 0.0),
    };
    match (width, height) {
        (Some(w), Some(h)) => (w, h),
        (Some(w), None) if natural_width > 0.0 => (w, w * natural_height / natural_width),
        (None, Some(h)) if natural_height > 0.0 => (h * natural_width / natural_height, h),
        (Some(w), None) => (w, 0.0),
        (None, Some(h)) => (0.0, h),
        (None, None) => (natural_width, natural_height),
    }
}

fn flush_items<'a>(
    align: TextAlign,
    children: &mut Vec<LayoutBox<'a>>,
    items: &mut Vec<InlineItem<'a>>,
) {
    let mut has_content = false;
    for item in items.iter() {
        match item {
            InlineItem::Text(run) => {
                if !run.text.trim().is_empty() {
                    has_content = true;
                }
            }
            InlineItem::Image(_) => has_content = true,
        }
    }
    if has_content {
        children.push(LayoutBox::new(
            BoxKind::Anonymous(InlineContent {
                items: std::mem::take(items),
                align,
            }),
            Vec::new(),
        ));
    } else {
        items.clear();
    }
}

struct Replaced {
    width: f32,
    height: f32,
    picture: Option<Rc<Picture>>,
}

fn block_replaced(style: &StyledNode, images: &ImageMap, container: f32) -> Option<Replaced> {
    let Content::Element(element) = style.content else {
        return None;
    };
    if element.tag_name != "img" {
        return None;
    }
    let picture = find_picture(element, images);
    let (width, height) = replaced_size(style, element, picture.as_deref(), Some(container));
    Some(Replaced {
        width,
        height,
        picture,
    })
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
    fn layout(&mut self, containing: &Dimensions, font: &Font, images: &ImageMap) {
        match &self.kind {
            BoxKind::Block(style) => {
                let style = *style;
                let replaced = block_replaced(style, images, containing.content.width);
                self.calculate_width(style, containing, replaced.as_ref());
                self.calculate_position(style, containing);
                self.layout_children(font, images);
                self.collapse_bottom_margin(style);
                self.calculate_height(style, replaced.as_ref());
                if let Some(replaced) = replaced {
                    self.place_picture(replaced.picture);
                }
            }
            BoxKind::Anonymous(content) => {
                let area = &mut self.dimensions.content;
                area.x = containing.content.x;
                area.y = containing.content.y + containing.content.height;
                area.width = containing.content.width;
                let lines = inline::layout_lines(content, area.width, area.x, area.y, font);
                area.height = lines.height;
                self.fragments = lines.fragments;
                self.images = lines.images;
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

    fn calculate_width(
        &mut self,
        style: &StyledNode,
        containing: &Dimensions,
        replaced: Option<&Replaced>,
    ) {
        let container = containing.content.width;
        let width = match replaced {
            Some(replaced) => Some(replaced.width),
            None => style.length_or_auto("width", None, Some(container)),
        };
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

    fn layout_children(&mut self, font: &Font, images: &ImageMap) {
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
            child.layout(&containing, font, images);
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

    fn place_picture(&mut self, picture: Option<Rc<Picture>>) {
        if let Some(picture) = picture {
            self.images.push(ImageFragment {
                rect: self.dimensions.content,
                picture,
            });
        }
    }

    fn calculate_height(&mut self, style: &StyledNode, replaced: Option<&Replaced>) {
        if let Some(replaced) = replaced {
            self.dimensions.content.height = replaced.height;
        } else if let Some(height) = style.length_or_auto("height", None, None) {
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
