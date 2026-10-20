// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashMap;
use std::rc::Rc;

use crate::text::Fonts;
use crate::transform_text;

use crate::css::Value;
use crate::dom::ElementData;
use crate::float::{FloatContext, Side};
use crate::inline::{
    self, BlockMetrics, BlockRun, Fragment, ImageFragment, ImageRun, InlineBoxFragment,
    InlineBoxStyle, InlineContent, InlineItem, RuleFragment, Run,
};
use crate::list;
use crate::picture::Picture;
use crate::style::{
    Clear, Content, Display, FlexDirection, Indent, Position, StyledNode, TextAlign, VerticalAlign,
    Viewport,
};

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Intrinsic {
    Min,
    Max,
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
    pub inline_boxes: Vec<InlineBoxFragment>,
    pub rules: Vec<RuleFragment>,
    pub background_pictures: Vec<Option<Rc<Picture>>>,
    pub(crate) marker: Option<String>,
    pub(crate) suppressed_borders: [bool; 4],
    pub(crate) forced_width: Option<f32>,
    pub(crate) forced_height: Option<f32>,
    pub(crate) static_position: Option<(f32, f32)>,
    pub(crate) formatting_root: bool,
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
    viewport: Viewport,
    fonts: &Fonts,
    images: &ImageMap,
) -> LayoutBox<'a> {
    let mut layout_root = build_block_box(root, images);
    let mut initial = Dimensions::default();
    initial.content.width = viewport.width;
    let mut floats = FloatContext::default();
    layout_root.layout(&initial, fonts, images, &mut floats);
    let bottom = floats.max_bottom();
    let content_bottom = layout_root.dimensions.content.y + layout_root.dimensions.content.height;
    if bottom > content_bottom {
        layout_root.dimensions.content.height += bottom - content_bottom;
    }
    let window = Rect {
        x: 0.0,
        y: 0.0,
        width: viewport.width,
        height: viewport.height,
    };
    layout_root.layout_out_of_flow(window, fonts, images);
    layout_root.layout_fixed(window, fonts, images);
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
            inline_boxes: Vec::new(),
            rules: Vec::new(),
            background_pictures: Vec::new(),
            marker: None,
            suppressed_borders: [false; 4],
            forced_width: None,
            forced_height: None,
            static_position: None,
            formatting_root: false,
        }
    }
}

struct Flow<'a> {
    children: Vec<LayoutBox<'a>>,
    items: Vec<InlineItem<'a>>,
    next_box_id: usize,
    counter: i32,
    vertical: VerticalAlign,
    indent: Indent,
    ellipsis: bool,
}

fn list_start(style: &StyledNode) -> i32 {
    let Content::Element(element) = style.content else {
        return 1;
    };
    if element.tag_name != "ol" {
        return 1;
    }
    element
        .attributes
        .get("start")
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(1)
}

fn build_block_box<'a>(style: &'a StyledNode<'a>, images: &ImageMap) -> LayoutBox<'a> {
    let mut flow = Flow {
        children: Vec::new(),
        items: Vec::new(),
        next_box_id: 0,
        counter: list_start(style),
        vertical: VerticalAlign::Raise(0.0),
        indent: style.text_indent(),
        ellipsis: style.ellipsis(),
    };
    let align = style.text_align();
    let blockify = matches!(style.display(), Display::Flex | Display::Grid);
    for child in &style.children {
        collect_child(child, align, blockify, images, &mut flow);
    }
    flush_items(align, &mut flow);
    let mut layout_box = LayoutBox::new(BoxKind::Block(style), flow.children);
    layout_box.background_pictures = (0..style.background_layers())
        .map(|index| {
            style
                .background_image(index)
                .and_then(|url| images.get(url).cloned())
        })
        .collect();
    layout_box
}

fn inline_box_style(node: &StyledNode, id: usize) -> Option<Rc<InlineBoxStyle>> {
    let edge = |names: [&str; 4]| names.map(|name| node.px(name, 0.0));
    let padding = edge([
        "padding-top",
        "padding-right",
        "padding-bottom",
        "padding-left",
    ]);
    let border = edge([
        "border-top-width",
        "border-right-width",
        "border-bottom-width",
        "border-left-width",
    ]);
    let margin_left = node.px("margin-left", 0.0);
    let margin_right = node.px("margin-right", 0.0);
    let background = node.background();
    let decorated = background.is_some()
        || padding.iter().any(|size| *size != 0.0)
        || border.iter().any(|size| *size != 0.0)
        || margin_left != 0.0
        || margin_right != 0.0;
    if !decorated {
        return None;
    }
    Some(Rc::new(InlineBoxStyle {
        id,
        background,
        border_color: node.color("border-top-color"),
        padding,
        border,
        margin_left,
        margin_right,
    }))
}

fn collect_child<'a>(
    node: &'a StyledNode<'a>,
    align: TextAlign,
    blockify: bool,
    images: &ImageMap,
    flow: &mut Flow<'a>,
) {
    let display = node.display();
    let floated = node.float().is_some()
        || node.is_out_of_flow()
        || (blockify && matches!(node.content, Content::Element(_)));
    match display {
        Display::None => {}
        Display::Block
        | Display::Flex
        | Display::Table
        | Display::TableRowGroup
        | Display::TableRow
        | Display::TableCell
        | Display::Grid => {
            flush_items(align, flow);
            flow.children.push(build_block_box(node, images));
        }
        Display::ListItem => {
            flush_items(align, flow);
            let mut item = build_block_box(node, images);
            let value = match node.content {
                Content::Element(element) => element
                    .attributes
                    .get("value")
                    .and_then(|value| value.trim().parse().ok()),
                Content::Text(_) | Content::Pseudo => None,
            }
            .unwrap_or(flow.counter);
            flow.counter = value.saturating_add(1);
            item.marker = list::marker_text(node.list_style_type(), value);
            flow.children.push(item);
        }
        Display::Inline | Display::InlineBlock if floated => {
            flush_items(align, flow);
            flow.children.push(build_block_box(node, images));
        }
        Display::InlineBlock => flow.items.push(InlineItem::Block(BlockRun {
            node,
            font_size: node.font_size(),
            vertical: combine_vertical(flow.vertical, node.inline_vertical_align()),
        })),
        Display::Inline => match &node.content {
            Content::Text(text) => flow.items.push(InlineItem::Text(Run {
                text: transform_text::apply(text, node.text_transform()),
                color: node.text_color(),
                alpha: node.text_alpha(),
                font_size: node.font_size(),
                line_height: node.line_height(),
                families: node.font_families(),
                bold: node.font_bold(),
                italic: node.font_italic(),
                white_space: node.white_space(),
                decorations: node.decorations(),
                vertical: flow.vertical,
                letter_spacing: node.letter_spacing(),
                word_spacing: node.word_spacing(),
                break_all: node.break_all(),
                break_long: node.break_long(),
            })),
            Content::Element(element) if element.tag_name == "img" => {
                let picture = find_picture(element, images);
                let (width, height) = replaced_size(node, element, picture.as_deref(), None);
                flow.items.push(InlineItem::Image(ImageRun {
                    picture,
                    width,
                    height,
                    font_size: node.font_size(),
                    vertical: combine_vertical(flow.vertical, node.inline_vertical_align()),
                }));
            }
            Content::Element(_) | Content::Pseudo => {
                let outer_vertical = flow.vertical;
                flow.vertical = combine_vertical(outer_vertical, node.inline_vertical_align());
                let style = inline_box_style(node, flow.next_box_id);
                flow.next_box_id += 1;
                if let Some(style) = &style {
                    flow.items.push(InlineItem::Open(style.clone()));
                }
                for child in &node.children {
                    collect_child(child, align, false, images, flow);
                }
                if let Some(style) = &style {
                    flow.items.push(InlineItem::Close(style.id));
                }
                flow.vertical = outer_vertical;
            }
        },
    }
}

fn combine_vertical(outer: VerticalAlign, own: VerticalAlign) -> VerticalAlign {
    match (outer, own) {
        (VerticalAlign::Raise(outer), VerticalAlign::Raise(own)) => {
            VerticalAlign::Raise(outer + own)
        }
        (outer, VerticalAlign::Raise(0.0)) => outer,
        (_, own) => own,
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

fn flush_items(align: TextAlign, flow: &mut Flow) {
    let mut has_content = false;
    for item in &flow.items {
        match item {
            InlineItem::Text(run) => {
                if !run.text.trim().is_empty()
                    || (run.white_space.keeps_spaces() && !run.text.is_empty())
                {
                    has_content = true;
                }
            }
            InlineItem::Image(_) | InlineItem::Block(_) => has_content = true,
            InlineItem::Open(_) | InlineItem::Close(_) => {}
        }
    }
    if has_content {
        flow.children.push(LayoutBox::new(
            BoxKind::Anonymous(InlineContent {
                items: std::mem::take(&mut flow.items),
                align,
                indent: if flow.children.is_empty() {
                    flow.indent
                } else {
                    Indent::default()
                },
                ellipsis: flow.ellipsis,
            }),
            Vec::new(),
        ));
    } else {
        flow.items.clear();
    }
}

pub(crate) struct Replaced {
    pub(crate) width: f32,
    pub(crate) height: f32,
    pub(crate) picture: Option<Rc<Picture>>,
}

pub(crate) fn block_replaced(
    style: &StyledNode,
    images: &ImageMap,
    container: f32,
) -> Option<Replaced> {
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

pub(crate) fn horizontal_extras(style: &StyledNode, base: f32) -> f32 {
    style.px("margin-left", base)
        + style.px("margin-right", base)
        + style.px("border-left-width", base)
        + style.px("border-right-width", base)
        + style.px("padding-left", base)
        + style.px("padding-right", base)
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
    pub(crate) fn layout(
        &mut self,
        containing: &Dimensions,
        fonts: &Fonts,
        images: &ImageMap,
        floats: &mut FloatContext,
    ) {
        match &self.kind {
            BoxKind::Block(style) => {
                let style = *style;
                let replaced = block_replaced(style, images, containing.content.width);
                let table = style.display() == Display::Table;
                let given_width = self.forced_width;
                if table && given_width.is_none() {
                    self.forced_width =
                        Some(self.table_width(style, containing.content.width, fonts, images));
                }
                self.calculate_width(style, containing, replaced.as_ref());
                self.forced_width = given_width;
                if table {
                    self.center_table(style, containing.content.width);
                }
                self.calculate_position(style, containing);
                if style.display() == Display::Flex {
                    self.layout_flex_children(style, fonts, images);
                } else if table {
                    self.layout_table_children(style, fonts, images);
                } else if style.display() == Display::Grid {
                    self.layout_grid_children(style, fonts, images);
                } else if self.is_formatting_root(style) {
                    let mut own = FloatContext::default();
                    self.layout_children(fonts, images, &mut own);
                    let content_bottom = self.dimensions.content.y + self.dimensions.content.height;
                    let float_bottom = own.max_bottom();
                    if float_bottom > content_bottom {
                        self.dimensions.content.height += float_bottom - content_bottom;
                    }
                } else {
                    self.layout_children(fonts, images, floats);
                }
                self.collapse_bottom_margin(style);
                let table_height = self.dimensions.content.height;
                self.calculate_height(style, replaced.as_ref());
                if table {
                    self.dimensions.content.height =
                        self.dimensions.content.height.max(table_height);
                }
                self.place_marker(style, fonts);
                if let Some(replaced) = replaced {
                    self.place_picture(replaced.picture);
                }
                if style.is_positioned() {
                    let area = self.dimensions.padding_box();
                    self.layout_out_of_flow(area, fonts, images);
                }
            }
            BoxKind::Anonymous(content) => {
                let area = &mut self.dimensions.content;
                area.x = containing.content.x;
                area.y = containing.content.y + containing.content.height;
                area.width = containing.content.width;
                let width = area.width;
                let (x, y) = (area.x, area.y);
                let mut blocks = layout_inline_blocks(content, width, fonts, images);
                let metrics: Vec<BlockMetrics> = blocks.iter().map(|(_, m)| *m).collect();
                let lines = inline::layout_lines(content, width, x, y, fonts, floats, &metrics);
                self.dimensions.content.height = lines.height;
                self.fragments = lines.fragments;
                self.images = lines.images;
                self.inline_boxes = lines.boxes;
                self.rules = lines.rules;
                for placed in lines.blocks {
                    let (mut block, metrics) = std::mem::replace(
                        &mut blocks[placed.index],
                        (
                            LayoutBox::new(
                                BoxKind::Anonymous(InlineContent {
                                    items: Vec::new(),
                                    align: TextAlign::Left,
                                    indent: Indent::default(),
                                    ellipsis: false,
                                }),
                                Vec::new(),
                            ),
                            BlockMetrics {
                                width: 0.0,
                                height: 0.0,
                                baseline: 0.0,
                            },
                        ),
                    );
                    let margin_box = block.dimensions.margin_box();
                    block.translate(
                        placed.x - margin_box.x,
                        placed.baseline - metrics.baseline - margin_box.y,
                    );
                    block.apply_relative_offset(width);
                    self.children.push(block);
                }
            }
        }
    }

    fn collapsed_top_margin(&self, base: f32) -> f32 {
        let BoxKind::Block(style) = &self.kind else {
            return 0.0;
        };
        let own = style.px("margin-top", base);
        if self.is_formatting_root(style)
            || style.px("border-top-width", base) != 0.0
            || style.px("padding-top", base) != 0.0
        {
            return own;
        }
        match self.children.first() {
            Some(first) if first.is_in_flow_block() => {
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
        if let Some(forced) = self.forced_width {
            self.calculate_forced_width(style, container, forced);
            return;
        }
        let width = match replaced {
            Some(replaced) => Some(replaced.width),
            None => style.length_or_auto("width", None, Some(container)),
        };
        let margin_left = style.length_or_auto("margin-left", Some(0.0), Some(container));
        let margin_right = style.length_or_auto("margin-right", Some(0.0), Some(container));

        let border_left = self.border_width(style, "border-left-width", 3, container);
        let border_right = self.border_width(style, "border-right-width", 1, container);
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
        let border_top = self.border_width(style, "border-top-width", 0, base);
        let border_bottom = self.border_width(style, "border-bottom-width", 2, base);
        let d = &mut self.dimensions;

        d.margin.top = margin_top;
        d.margin.bottom = style.px("margin-bottom", base);
        d.border.top = border_top;
        d.border.bottom = border_bottom;
        d.padding.top = style.px("padding-top", base);
        d.padding.bottom = style.px("padding-bottom", base);

        d.content.x = containing.content.x + d.margin.left + d.border.left + d.padding.left;
        d.content.y = containing.content.y
            + containing.content.height
            + d.margin.top
            + d.border.top
            + d.padding.top;
    }

    pub(crate) fn apply_relative_offset(&mut self, base: f32) {
        let BoxKind::Block(style) = &self.kind else {
            return;
        };
        if style.position() != Position::Relative {
            return;
        }
        let left = style.length_or_auto("left", None, Some(base));
        let right = style.length_or_auto("right", None, Some(base));
        let top = style.length_or_auto("top", None, None);
        let bottom = style.length_or_auto("bottom", None, None);
        let dx = left.or(right.map(|value| -value)).unwrap_or(0.0);
        let dy = top.or(bottom.map(|value| -value)).unwrap_or(0.0);
        self.translate(dx, dy);
    }

    fn layout_out_of_flow(&mut self, area: Rect, fonts: &Fonts, images: &ImageMap) {
        for child in &mut self.children {
            let position = match &child.kind {
                BoxKind::Block(style) => style.position(),
                BoxKind::Anonymous(_) => {
                    child.layout_out_of_flow(area, fonts, images);
                    continue;
                }
            };
            match position {
                Position::Absolute => child.layout_absolute(area, fonts, images),
                Position::Static => child.layout_out_of_flow(area, fonts, images),
                Position::Relative | Position::Fixed | Position::Sticky => {}
            }
        }
    }

    fn layout_fixed(&mut self, window: Rect, fonts: &Fonts, images: &ImageMap) {
        for child in &mut self.children {
            let position = match &child.kind {
                BoxKind::Block(style) => style.position(),
                BoxKind::Anonymous(_) => {
                    child.layout_fixed(window, fonts, images);
                    continue;
                }
            };
            if position == Position::Fixed {
                child.layout_absolute(window, fonts, images);
            }
            child.layout_fixed(window, fonts, images);
        }
    }

    fn layout_absolute(&mut self, area: Rect, fonts: &Fonts, images: &ImageMap) {
        let Some(static_position) = self.static_position.take() else {
            return;
        };
        let BoxKind::Block(style) = &self.kind else {
            return;
        };
        let style = *style;
        let replaced = block_replaced(style, images, area.width);

        let left = style.length_or_auto("left", None, Some(area.width));
        let right = style.length_or_auto("right", None, Some(area.width));
        let top = style.length_or_auto("top", None, Some(area.height));
        let bottom = style.length_or_auto("bottom", None, Some(area.height));
        let auto_margin = |name: &str| {
            style
                .length_or_auto(name, Some(0.0), Some(area.width))
                .is_none()
        };
        let margin_left_auto = auto_margin("margin-left");
        let margin_right_auto = auto_margin("margin-right");
        let margin_top_auto = auto_margin("margin-top");
        let margin_bottom_auto = auto_margin("margin-bottom");
        let margin_left = style.px("margin-left", area.width);
        let margin_right = style.px("margin-right", area.width);
        let margin_top = style.px("margin-top", area.width);
        let margin_bottom = style.px("margin-bottom", area.width);
        let horizontal_extras = style.px("border-left-width", area.width)
            + style.px("border-right-width", area.width)
            + style.px("padding-left", area.width)
            + style.px("padding-right", area.width);
        let vertical_extras = style.px("border-top-width", area.width)
            + style.px("border-bottom-width", area.width)
            + style.px("padding-top", area.width)
            + style.px("padding-bottom", area.width);

        let specified_width = match &replaced {
            Some(replaced) => Some(replaced.width),
            None => style.length_or_auto("width", None, Some(area.width)),
        };
        let mut width = match (specified_width, left, right) {
            (Some(width), _, _) => width,
            (None, Some(left), Some(right)) => {
                (area.width - left - right - margin_left - margin_right - horizontal_extras)
                    .max(0.0)
            }
            (None, left, right) => {
                let available = (area.width
                    - left.unwrap_or(0.0)
                    - right.unwrap_or(0.0)
                    - margin_left
                    - margin_right
                    - horizontal_extras)
                    .max(0.0);
                self.preferred_width(fonts, images, area.width)
                    .min(available)
            }
        };
        if let Some(max) = style.length_or_auto("max-width", None, Some(area.width)) {
            width = width.min(max);
        }
        if let Some(min) = style.length_or_auto("min-width", None, Some(area.width)) {
            width = width.max(min);
        }
        self.forced_width = Some(width);

        let mut origin = Dimensions::default();
        origin.content.width = area.width;
        self.layout(&origin, fonts, images, &mut FloatContext::default());

        let mut height = match &replaced {
            Some(_) => None,
            None => style.length_or_auto("height", None, Some(area.height)),
        };
        if height.is_none() && replaced.is_none() {
            if let (Some(top), Some(bottom)) = (top, bottom) {
                height = Some(
                    (area.height - top - bottom - margin_top - margin_bottom - vertical_extras)
                        .max(0.0),
                );
            }
        }
        if let Some(mut height) = height {
            if let Some(max) = style.length_or_auto("max-height", None, Some(area.height)) {
                height = height.min(max);
            }
            if let Some(min) = style.length_or_auto("min-height", None, Some(area.height)) {
                height = height.max(min);
            }
            self.dimensions.content.height = height;
        }

        let border_box = self.dimensions.border_box();
        let specified_height = height.is_some() || replaced.is_some();

        let x = match (left, right) {
            (Some(left), Some(right)) if specified_width.is_some() => {
                if margin_left_auto && margin_right_auto {
                    let free = (area.width - left - right - border_box.width).max(0.0);
                    area.x + left + free / 2.0
                } else if margin_left_auto {
                    area.x + area.width - right - margin_right - border_box.width
                } else {
                    area.x + left + margin_left
                }
            }
            (Some(left), _) => area.x + left + margin_left,
            (None, Some(right)) => area.x + area.width - right - margin_right - border_box.width,
            (None, None) => static_position.0 + margin_left,
        };
        let y = match (top, bottom) {
            (Some(top), Some(bottom)) if specified_height => {
                if margin_top_auto && margin_bottom_auto {
                    let free = (area.height - top - bottom - border_box.height).max(0.0);
                    area.y + top + free / 2.0
                } else if margin_top_auto {
                    area.y + area.height - bottom - margin_bottom - border_box.height
                } else {
                    area.y + top + margin_top
                }
            }
            (Some(top), _) => area.y + top + margin_top,
            (None, Some(bottom)) => {
                area.y + area.height - bottom - margin_bottom - border_box.height
            }
            (None, None) => static_position.1 + margin_top,
        };
        self.translate(x - border_box.x, y - border_box.y);
    }

    fn flex_preferred_width(
        &self,
        style: &StyledNode,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> f32 {
        let row = matches!(
            style.flex_direction(),
            FlexDirection::Row | FlexDirection::RowReverse
        );
        let gap = if row {
            style.px("column-gap", base)
        } else {
            style.px("row-gap", base)
        };
        let mut total: f32 = 0.0;
        let mut count = 0;
        for child in &self.children {
            let extras = match &child.kind {
                BoxKind::Block(child_style) => {
                    if child_style.is_out_of_flow() {
                        continue;
                    }
                    horizontal_extras(child_style, base)
                }
                BoxKind::Anonymous(_) => 0.0,
            };
            let size = child.preferred_width(fonts, images, base) + extras;
            total = if row { total + size } else { total.max(size) };
            count += 1;
        }
        if row && count > 1 {
            total += gap * (count - 1) as f32;
        }
        total
    }

    fn is_in_flow_block(&self) -> bool {
        match &self.kind {
            BoxKind::Block(style) => style.float().is_none() && !style.is_out_of_flow(),
            BoxKind::Anonymous(_) => false,
        }
    }

    fn center_table(&mut self, style: &StyledNode, container: f32) {
        let is_auto = |name: &str| matches!(style.value(name), Some(Value::Keyword(keyword)) if keyword == "auto");
        let auto_left = is_auto("margin-left");
        let auto_right = is_auto("margin-right");
        if !auto_left && !auto_right {
            return;
        }
        let d = &mut self.dimensions;
        let used =
            d.content.width + d.padding.left + d.padding.right + d.border.left + d.border.right;
        let fixed_left = if auto_left { 0.0 } else { d.margin.left };
        let fixed_right = if auto_right { 0.0 } else { d.margin.right };
        let free = (container - used - fixed_left - fixed_right).max(0.0);
        match (auto_left, auto_right) {
            (true, true) => {
                d.margin.left = free / 2.0;
                d.margin.right = free / 2.0;
            }
            (true, false) => d.margin.left = free,
            _ => d.margin.right = free,
        }
    }

    fn border_width(&self, style: &StyledNode, name: &str, side: usize, base: f32) -> f32 {
        if self.suppressed_borders[side] {
            0.0
        } else {
            style.px(name, base)
        }
    }

    fn calculate_forced_width(&mut self, style: &StyledNode, container: f32, width: f32) {
        let d = &mut self.dimensions;
        d.content.width = width;
        d.padding.left = style.px("padding-left", container);
        d.padding.right = style.px("padding-right", container);
        d.border.left = if self.suppressed_borders[3] {
            0.0
        } else {
            style.px("border-left-width", container)
        };
        d.border.right = if self.suppressed_borders[1] {
            0.0
        } else {
            style.px("border-right-width", container)
        };
        d.margin.left = style.px("margin-left", container);
        d.margin.right = style.px("margin-right", container);
    }

    pub(crate) fn preferred_width(&self, fonts: &Fonts, images: &ImageMap, base: f32) -> f32 {
        self.intrinsic_width(Intrinsic::Max, fonts, images, base)
    }

    pub(crate) fn intrinsic_width(
        &self,
        mode: Intrinsic,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> f32 {
        match &self.kind {
            BoxKind::Anonymous(content) => {
                let blocks = layout_inline_blocks(content, base, fonts, images);
                let metrics: Vec<BlockMetrics> = blocks.iter().map(|(_, m)| *m).collect();
                match mode {
                    Intrinsic::Max => inline::max_content_width(content, fonts, &metrics),
                    Intrinsic::Min => inline::min_content_width(content, fonts, &metrics),
                }
            }
            BoxKind::Block(style) => {
                if let Some(replaced) = block_replaced(style, images, base) {
                    return replaced.width;
                }
                let is_cell = style.display() == Display::TableCell;
                if !(is_cell && mode == Intrinsic::Min) {
                    if let Some(width) = style.length_or_auto("width", None, Some(base)) {
                        return width;
                    }
                }
                self.block_content_width(style, mode, fonts, images, base)
            }
        }
    }

    fn block_content_width(
        &self,
        style: &StyledNode,
        mode: Intrinsic,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> f32 {
        if style.display() == Display::Flex {
            return self.flex_preferred_width(style, fonts, images, base);
        }
        if style.display() == Display::Table {
            return self.table_intrinsic_width(style, mode, fonts, images, base);
        }
        if style.display() == Display::Grid {
            return self.grid_intrinsic_width(style, mode, fonts, images, base);
        }
        let mut widest: f32 = 0.0;
        for child in &self.children {
            if let BoxKind::Block(child_style) = &child.kind {
                if child_style.is_out_of_flow() {
                    continue;
                }
            }
            let extras = match &child.kind {
                BoxKind::Block(child_style) => horizontal_extras(child_style, base),
                BoxKind::Anonymous(_) => 0.0,
            };
            widest = widest.max(child.intrinsic_width(mode, fonts, images, base) + extras);
        }
        widest
    }

    pub(crate) fn min_content_width(&self, fonts: &Fonts, images: &ImageMap, base: f32) -> f32 {
        match &self.kind {
            BoxKind::Block(style) => match block_replaced(style, images, base) {
                Some(replaced) => replaced.width,
                None => self.block_content_width(style, Intrinsic::Min, fonts, images, base),
            },
            BoxKind::Anonymous(_) => self.intrinsic_width(Intrinsic::Min, fonts, images, base),
        }
    }

    pub(crate) fn reset(&mut self) {
        self.dimensions = Dimensions::default();
        self.fragments.clear();
        self.images.clear();
        self.inline_boxes.clear();
        self.rules.clear();
        for child in &mut self.children {
            child.reset();
        }
    }

    fn is_formatting_root(&self, style: &StyledNode) -> bool {
        self.formatting_root || style.establishes_block_context()
    }

    pub(crate) fn translate(&mut self, dx: f32, dy: f32) {
        self.dimensions.content.x += dx;
        self.dimensions.content.y += dy;
        if let Some((x, y)) = &mut self.static_position {
            *x += dx;
            *y += dy;
        }
        for fragment in &mut self.fragments {
            fragment.x += dx;
            fragment.baseline += dy;
        }
        for inline_box in &mut self.inline_boxes {
            inline_box.rect.x += dx;
            inline_box.rect.y += dy;
        }
        for rule in &mut self.rules {
            rule.rect.x += dx;
            rule.rect.y += dy;
        }
        for image in &mut self.images {
            image.rect.x += dx;
            image.rect.y += dy;
        }
        for child in &mut self.children {
            child.translate(dx, dy);
        }
    }

    fn place_marker(&mut self, style: &StyledNode, fonts: &Fonts) {
        self.fragments.clear();
        let Some(text) = &self.marker else {
            return;
        };
        let size = style.font_size();
        let face = fonts.select(
            style.font_families(),
            style.font_bold(),
            style.font_italic(),
        );
        let baseline =
            first_baseline(self).unwrap_or(self.dimensions.content.y + fonts.ascent(face, size));
        let width = fonts.width(face, text, size);
        let fragment = Fragment {
            x: self.dimensions.content.x - width,
            baseline,
            text: text.clone(),
            color: style.text_color(),
            alpha: style.text_alpha(),
            font_size: size,
            face,
            letter_spacing: 0.0,
            word_spacing: 0.0,
        };
        self.fragments.push(fragment);
    }

    pub(crate) fn shrink_to_fit_width(
        &self,
        style: &StyledNode,
        container: f32,
        fonts: &Fonts,
        images: &ImageMap,
    ) -> f32 {
        let replaced = block_replaced(style, images, container);
        let extras = horizontal_extras(style, container);
        let specified = match &replaced {
            Some(replaced) => Some(replaced.width),
            None => style.length_or_auto("width", None, Some(container)),
        };
        let mut width = match specified {
            Some(width) => width,
            None => self
                .preferred_width(fonts, images, container)
                .min((container - extras).max(0.0)),
        };
        if let Some(max) = style.length_or_auto("max-width", None, Some(container)) {
            width = width.min(max);
        }
        if let Some(min) = style.length_or_auto("min-width", None, Some(container)) {
            width = width.max(min);
        }
        width
    }

    fn layout_float(
        &mut self,
        side: Side,
        parent: &Dimensions,
        fonts: &Fonts,
        images: &ImageMap,
        floats: &mut FloatContext,
    ) {
        let BoxKind::Block(style) = &self.kind else {
            return;
        };
        let style = *style;
        let container = parent.content.width;
        let width = self.shrink_to_fit_width(style, container, fonts, images);
        self.forced_width = Some(width);

        let mut origin = Dimensions::default();
        origin.content.width = container;
        self.layout(&origin, fonts, images, &mut FloatContext::default());

        let margin_box = self.dimensions.margin_box();
        let flow_y = parent.content.y + parent.content.height;
        let min_y = flow_y
            .max(floats.clearance(style.clear()))
            .max(floats.top_limit());
        let (x, y) = floats.place(
            side,
            margin_box.width,
            margin_box.height,
            min_y,
            parent.content.x,
            parent.content.x + container,
        );
        self.translate(x - margin_box.x, y - margin_box.y);
        self.apply_relative_offset(container);
        floats.add(
            side,
            Rect {
                x,
                y,
                width: margin_box.width,
                height: margin_box.height,
            },
        );
    }

    fn layout_children(&mut self, fonts: &Fonts, images: &ImageMap, floats: &mut FloatContext) {
        let base = self.dimensions.content.width;
        let is_root = match &self.kind {
            BoxKind::Block(style) => self.is_formatting_root(style),
            BoxKind::Anonymous(_) => false,
        };
        let top_open =
            !is_root && self.dimensions.border.top == 0.0 && self.dimensions.padding.top == 0.0;
        let mut previous_bottom: Option<f32> = None;

        for (index, child) in self.children.iter_mut().enumerate() {
            if let BoxKind::Block(style) = &child.kind {
                if style.is_out_of_flow() {
                    child.static_position = Some((
                        self.dimensions.content.x,
                        self.dimensions.content.y + self.dimensions.content.height,
                    ));
                    continue;
                }
                if let Some(side) = style.float() {
                    child.layout_float(side, &self.dimensions, fonts, images, floats);
                    continue;
                }
            }

            let is_block = matches!(child.kind, BoxKind::Block(_));
            let mut overlap = 0.0;
            let mut shift = 0.0;
            if let BoxKind::Block(style) = &child.kind {
                let top = child.collapsed_top_margin(base);
                if index == 0 {
                    if top_open {
                        overlap = top;
                    }
                } else if let Some(bottom) = previous_bottom {
                    overlap = bottom + top - combine_margins(bottom, top);
                }
                let clear = style.clear();
                if clear != Clear::None {
                    let border_top =
                        self.dimensions.content.y + self.dimensions.content.height - overlap + top;
                    let clear_y = floats.clearance(clear);
                    if clear_y > border_top {
                        shift = clear_y - border_top;
                    }
                }
            }

            let mut containing = self.dimensions;
            containing.content.height += shift - overlap;
            child.layout(&containing, fonts, images, floats);
            self.dimensions.content.height +=
                child.dimensions.margin_box().height - overlap + shift;
            child.apply_relative_offset(base);

            previous_bottom = if is_block {
                Some(child.dimensions.margin.bottom)
            } else {
                None
            };
        }
    }

    fn collapse_bottom_margin(&mut self, style: &StyledNode) {
        let base = self.dimensions.content.width;
        let closed = self.is_formatting_root(style)
            || style.length_or_auto("height", None, None).is_some()
            || self.dimensions.border.bottom != 0.0
            || self.dimensions.padding.bottom != 0.0
            || style.px("border-bottom-width", base) != 0.0;
        if closed {
            return;
        }
        if let Some(last) = self.children.last() {
            if last.is_in_flow_block() {
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

    fn ratio_height(&self, style: &StyledNode) -> Option<f32> {
        let ratio = style.aspect_ratio()?;
        if style.length_or_auto("height", None, None).is_some()
            || style.display() == Display::TableCell
        {
            return None;
        }
        let d = &self.dimensions;
        if !style.is_border_box() {
            return Some(d.content.width / ratio);
        }
        let width =
            d.content.width + d.padding.left + d.padding.right + d.border.left + d.border.right;
        let extras = d.padding.top + d.padding.bottom + d.border.top + d.border.bottom;
        Some((width / ratio - extras).max(0.0))
    }

    fn calculate_height(&mut self, style: &StyledNode, replaced: Option<&Replaced>) {
        if let Some(replaced) = replaced {
            self.dimensions.content.height = replaced.height;
        } else if let Some(height) = style
            .length_or_auto("height", None, None)
            .filter(|_| style.display() != Display::TableCell)
        {
            self.dimensions.content.height = height;
        }
        if let Some(height) = self.ratio_height(style).filter(|_| replaced.is_none()) {
            self.dimensions.content.height = height;
        }
        if let Some(max) = style.length_or_auto("max-height", None, None) {
            self.dimensions.content.height = self.dimensions.content.height.min(max);
        }
        if let Some(min) = style.length_or_auto("min-height", None, None) {
            self.dimensions.content.height = self.dimensions.content.height.max(min);
        }
        if let Some(height) = self.forced_height {
            self.dimensions.content.height = height;
        }
    }
}

fn layout_inline_blocks<'a>(
    content: &InlineContent<'a>,
    available: f32,
    fonts: &Fonts,
    images: &ImageMap,
) -> Vec<(LayoutBox<'a>, BlockMetrics)> {
    let mut blocks = Vec::new();
    for item in &content.items {
        let InlineItem::Block(run) = item else {
            continue;
        };
        let style = run.node;
        let mut block = build_block_box(style, images);
        block.forced_width = Some(block.shrink_to_fit_width(style, available, fonts, images));
        let mut origin = Dimensions::default();
        origin.content.width = available;
        block.layout(&origin, fonts, images, &mut FloatContext::default());
        let margin_box = block.dimensions.margin_box();
        block.translate(-margin_box.x, -margin_box.y);
        let height = margin_box.height;
        let baseline = if style.establishes_clip() {
            height
        } else {
            last_baseline(&block).unwrap_or(height)
        };
        blocks.push((
            block,
            BlockMetrics {
                width: margin_box.width,
                height,
                baseline,
            },
        ));
    }
    blocks
}

fn last_baseline(layout_box: &LayoutBox) -> Option<f32> {
    for child in layout_box.children.iter().rev() {
        if let BoxKind::Block(style) = &child.kind {
            if style.is_out_of_flow() || style.float().is_some() {
                continue;
            }
        }
        if let Some(baseline) = last_baseline(child) {
            return Some(baseline);
        }
    }
    layout_box
        .fragments
        .last()
        .map(|fragment| fragment.baseline)
}

fn first_baseline(layout_box: &LayoutBox) -> Option<f32> {
    if let Some(fragment) = layout_box.fragments.first() {
        return Some(fragment.baseline);
    }
    for child in &layout_box.children {
        if let BoxKind::Block(style) = &child.kind {
            if style.is_out_of_flow() || style.float().is_some() {
                continue;
            }
        }
        if let Some(baseline) = first_baseline(child) {
            return Some(baseline);
        }
    }
    None
}
