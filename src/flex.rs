// SPDX-License-Identifier: GPL-3.0-only

use crate::text::Fonts;

use crate::float::FloatContext;
use crate::layout::{block_replaced, BoxKind, Dimensions, EdgeSizes, ImageMap, LayoutBox};
use crate::style::{Align, FlexBasis, FlexDirection, FlexWrap, Justify, StyledNode};

const TOP: usize = 0;
const RIGHT: usize = 1;
const BOTTOM: usize = 2;
const LEFT: usize = 3;

struct Axes {
    row: bool,
    main_reversed: bool,
    wrap_reversed: bool,
}

impl Axes {
    fn main_start(&self) -> usize {
        match (self.row, self.main_reversed) {
            (true, false) => LEFT,
            (true, true) => RIGHT,
            (false, false) => TOP,
            (false, true) => BOTTOM,
        }
    }

    fn main_end(&self) -> usize {
        match (self.row, self.main_reversed) {
            (true, false) => RIGHT,
            (true, true) => LEFT,
            (false, false) => BOTTOM,
            (false, true) => TOP,
        }
    }

    fn cross_start(&self) -> usize {
        match (self.row, self.wrap_reversed) {
            (true, false) => TOP,
            (true, true) => BOTTOM,
            (false, false) => LEFT,
            (false, true) => RIGHT,
        }
    }

    fn cross_end(&self) -> usize {
        match (self.row, self.wrap_reversed) {
            (true, false) => BOTTOM,
            (true, true) => TOP,
            (false, false) => RIGHT,
            (false, true) => LEFT,
        }
    }
}

struct Item {
    index: usize,
    order: i32,
    grow: f32,
    shrink: f32,
    basis: f32,
    min: f32,
    max: f32,
    hypothetical: f32,
    target: f32,
    margin: [f32; 4],
    auto: [bool; 4],
    main_extras: f32,
    cross_extras: f32,
    cross_pre: f32,
    cross_auto: bool,
    cross_min: f32,
    cross_max: f32,
    align: Align,
    measured_cross: f32,
    cross_content: f32,
    cross_forced: bool,
    main_pos: f32,
    cross_pos: f32,
}

impl Item {
    fn margin_main(&self, axes: &Axes) -> f32 {
        self.margin[axes.main_start()] + self.margin[axes.main_end()]
    }

    fn margin_cross(&self, axes: &Axes) -> f32 {
        self.margin[axes.cross_start()] + self.margin[axes.cross_end()]
    }

    fn outer_main(&self, axes: &Axes, size: f32) -> f32 {
        size + self.main_extras + self.margin_main(axes)
    }

    fn outer_cross(&self, axes: &Axes, size: f32) -> f32 {
        size + self.cross_extras + self.margin_cross(axes)
    }
}

fn distribute(kind: Justify, free: f32, count: usize) -> (f32, f32) {
    let n = count as f32;
    match kind {
        Justify::Start | Justify::Stretch => (0.0, 0.0),
        Justify::End => (free, 0.0),
        Justify::Center => (free / 2.0, 0.0),
        Justify::SpaceBetween => {
            if free > 0.0 && count > 1 {
                (0.0, free / (n - 1.0))
            } else {
                (0.0, 0.0)
            }
        }
        Justify::SpaceAround => {
            if free > 0.0 {
                (free / n / 2.0, free / n)
            } else {
                (0.0, 0.0)
            }
        }
        Justify::SpaceEvenly => {
            if free > 0.0 {
                (free / (n + 1.0), free / (n + 1.0))
            } else {
                (0.0, 0.0)
            }
        }
    }
}

fn measure(
    child: &mut LayoutBox,
    forced_width: Option<f32>,
    forced_height: Option<f32>,
    base: f32,
    fonts: &Fonts,
    images: &ImageMap,
) -> Dimensions {
    child.forced_width = forced_width;
    child.forced_height = forced_height;
    child.reset();
    let mut origin = Dimensions::default();
    origin.content.width = base;
    child.layout(&origin, fonts, images, &mut FloatContext::default());
    child.dimensions
}

struct Container<'c> {
    style: &'c StyledNode<'c>,
    axes: Axes,
    width: f32,
    height: Option<f32>,
}

fn read_item(
    index: usize,
    child: &mut LayoutBox,
    container: &Container,
    fonts: &Fonts,
    images: &ImageMap,
) -> Item {
    let axes = &container.axes;
    let width = container.width;
    let mut item = Item {
        index,
        order: 0,
        grow: 0.0,
        shrink: 1.0,
        basis: 0.0,
        min: 0.0,
        max: f32::INFINITY,
        hypothetical: 0.0,
        target: 0.0,
        margin: [0.0; 4],
        auto: [false; 4],
        main_extras: 0.0,
        cross_extras: 0.0,
        cross_pre: 0.0,
        cross_auto: true,
        cross_min: 0.0,
        cross_max: f32::INFINITY,
        align: container.style.align_items(),
        measured_cross: 0.0,
        cross_content: 0.0,
        cross_forced: false,
        main_pos: 0.0,
        cross_pos: 0.0,
    };
    let mut specified_main = None;
    let mut specified_cross = None;
    let mut basis_kind = FlexBasis::Auto;

    if let BoxKind::Block(style) = &child.kind {
        let style = *style;
        let names = ["margin-top", "margin-right", "margin-bottom", "margin-left"];
        for (side, name) in names.iter().enumerate() {
            item.auto[side] = style.length_or_auto(name, Some(0.0), Some(width)).is_none();
            item.margin[side] = style.px(name, width);
        }
        let horizontal = style.px("padding-left", width)
            + style.px("padding-right", width)
            + style.px("border-left-width", width)
            + style.px("border-right-width", width);
        let vertical = style.px("padding-top", width)
            + style.px("padding-bottom", width)
            + style.px("border-top-width", width)
            + style.px("border-bottom-width", width);
        let (main_extras, cross_extras) = if axes.row {
            (horizontal, vertical)
        } else {
            (vertical, horizontal)
        };
        item.main_extras = main_extras;
        item.cross_extras = cross_extras;

        let replaced = block_replaced(style, images, width);
        let (specified_width, specified_height) = match &replaced {
            Some(replaced) => (Some(replaced.width), Some(replaced.height)),
            None => (
                style.length_or_auto("width", None, Some(width)),
                style.length_or_auto("height", None, container.height),
            ),
        };
        let limit = |name: &str, base: Option<f32>| style.length_or_auto(name, None, base);
        if axes.row {
            specified_main = specified_width;
            specified_cross = specified_height;
            item.min = limit("min-width", Some(width)).unwrap_or(0.0);
            item.max = limit("max-width", Some(width)).unwrap_or(f32::INFINITY);
            item.cross_min = limit("min-height", container.height).unwrap_or(0.0);
            item.cross_max = limit("max-height", container.height).unwrap_or(f32::INFINITY);
        } else {
            specified_main = specified_height;
            specified_cross = specified_width;
            item.min = limit("min-height", container.height).unwrap_or(0.0);
            item.max = limit("max-height", container.height).unwrap_or(f32::INFINITY);
            item.cross_min = limit("min-width", Some(width)).unwrap_or(0.0);
            item.cross_max = limit("max-width", Some(width)).unwrap_or(f32::INFINITY);
        }
        item.grow = style.flex_grow();
        item.shrink = style.flex_shrink();
        item.order = style.order();
        item.align = style.align_self().unwrap_or(item.align);
        let basis_base = if axes.row {
            Some(width)
        } else {
            container.height
        };
        basis_kind = style.flex_basis(basis_base);
    }
    item.cross_auto = specified_cross.is_none();

    if !axes.row {
        let available = (width - item.cross_extras - item.margin_cross(axes)).max(0.0);
        let pre = match specified_cross {
            Some(size) => size,
            None if item.align == Align::Stretch => available,
            None => child.preferred_width(fonts, images, width).min(available),
        };
        item.cross_pre = pre.min(item.cross_max).max(item.cross_min);
    }

    let basis = match (basis_kind, specified_main) {
        (FlexBasis::Size(size), _) => size,
        (FlexBasis::Auto, Some(size)) => size,
        _ => {
            if axes.row {
                child.preferred_width(fonts, images, width)
            } else {
                measure(child, Some(item.cross_pre), None, width, fonts, images)
                    .content
                    .height
            }
        }
    };
    item.basis = basis;
    item.hypothetical = basis.min(item.max).max(item.min);
    item
}

fn resolve_flexible(items: &mut [Item], line: &[usize], axes: &Axes, available: f32) {
    let extras = |item: &Item| item.main_extras + item.margin_main(axes);
    let hypothetical_sum: f32 = line
        .iter()
        .map(|&i| items[i].hypothetical + extras(&items[i]))
        .sum();
    let growing = hypothetical_sum < available;

    let mut frozen = vec![false; line.len()];
    for (slot, &i) in line.iter().enumerate() {
        let item = &mut items[i];
        let factor = if growing { item.grow } else { item.shrink };
        let inflexible = factor == 0.0
            || (growing && item.basis > item.hypothetical)
            || (!growing && item.basis < item.hypothetical);
        if inflexible {
            frozen[slot] = true;
            item.target = item.hypothetical;
        }
    }

    while frozen.iter().any(|done| !done) {
        let mut used = 0.0;
        let mut factor_sum = 0.0;
        let mut scaled_sum = 0.0;
        for (slot, &i) in line.iter().enumerate() {
            let item = &items[i];
            if frozen[slot] {
                used += item.target + extras(item);
            } else {
                used += item.basis + extras(item);
                factor_sum += item.grow;
                scaled_sum += item.shrink * item.basis;
            }
        }
        let mut free = available - used;
        if growing && factor_sum < 1.0 {
            let scaled = free * factor_sum;
            if scaled.abs() < free.abs() {
                free = scaled;
            }
        }

        let mut violations = vec![0.0; line.len()];
        let mut clamped = vec![0.0; line.len()];
        let mut total_violation = 0.0;
        for (slot, &i) in line.iter().enumerate() {
            if frozen[slot] {
                continue;
            }
            let item = &items[i];
            let size = if growing {
                if factor_sum > 0.0 {
                    item.basis + free * item.grow / factor_sum
                } else {
                    item.basis
                }
            } else if scaled_sum > 0.0 {
                item.basis + free * (item.shrink * item.basis) / scaled_sum
            } else {
                item.basis
            };
            let size = size.max(0.0);
            let limited = size.min(item.max).max(item.min);
            clamped[slot] = limited;
            violations[slot] = limited - size;
            total_violation += violations[slot];
        }

        for (slot, &i) in line.iter().enumerate() {
            if frozen[slot] {
                continue;
            }
            let freeze = if total_violation == 0.0 {
                true
            } else if total_violation > 0.0 {
                violations[slot] > 0.0
            } else {
                violations[slot] < 0.0
            };
            if freeze {
                frozen[slot] = true;
                items[i].target = clamped[slot];
            }
        }
    }
}

impl<'a> LayoutBox<'a> {
    pub(crate) fn layout_flex_children(
        &mut self,
        style: &StyledNode,
        fonts: &Fonts,
        images: &ImageMap,
    ) {
        let direction = style.flex_direction();
        let wrap = style.flex_wrap();
        let container = Container {
            style,
            axes: Axes {
                row: matches!(direction, FlexDirection::Row | FlexDirection::RowReverse),
                main_reversed: matches!(
                    direction,
                    FlexDirection::RowReverse | FlexDirection::ColumnReverse
                ),
                wrap_reversed: wrap == FlexWrap::WrapReverse,
            },
            width: self.dimensions.content.width,
            height: style.length_or_auto("height", None, None),
        };
        let axes = &container.axes;
        let width = container.width;
        let origin_x = self.dimensions.content.x;
        let origin_y = self.dimensions.content.y;
        let main_limit = if axes.row {
            Some(width)
        } else {
            container.height
        };
        let cross_limit = if axes.row {
            container.height
        } else {
            Some(width)
        };
        let (main_gap, cross_gap) = if axes.row {
            (style.px("column-gap", width), style.px("row-gap", width))
        } else {
            (style.px("row-gap", width), style.px("column-gap", width))
        };

        let mut items: Vec<Item> = Vec::new();
        for index in 0..self.children.len() {
            let child = &mut self.children[index];
            if let BoxKind::Block(child_style) = &child.kind {
                if child_style.is_out_of_flow() {
                    child.static_position = Some((origin_x, origin_y));
                    continue;
                }
            }
            child.formatting_root = true;
            items.push(read_item(index, child, &container, fonts, images));
        }
        items.sort_by_key(|item| item.order);
        if items.is_empty() {
            return;
        }

        let mut lines: Vec<Vec<usize>> = Vec::new();
        match main_limit {
            Some(limit) if wrap != FlexWrap::NoWrap => {
                let mut current: Vec<usize> = Vec::new();
                let mut used = 0.0;
                for (slot, item) in items.iter().enumerate() {
                    let outer = item.outer_main(axes, item.hypothetical);
                    let needed = if current.is_empty() {
                        outer
                    } else {
                        used + main_gap + outer
                    };
                    if !current.is_empty() && needed > limit {
                        lines.push(std::mem::take(&mut current));
                        used = outer;
                    } else {
                        used = needed;
                    }
                    current.push(slot);
                }
                lines.push(current);
            }
            _ => lines.push((0..items.len()).collect()),
        }

        for line in &lines {
            let gaps = main_gap * (line.len() - 1) as f32;
            match main_limit {
                Some(limit) => {
                    resolve_flexible(&mut items, line, axes, limit - gaps);
                    let used: f32 = line
                        .iter()
                        .map(|&i| items[i].outer_main(axes, items[i].target))
                        .sum();
                    let free = limit - gaps - used;
                    let mut auto_count = 0;
                    for &i in line {
                        auto_count += usize::from(items[i].auto[axes.main_start()])
                            + usize::from(items[i].auto[axes.main_end()]);
                    }
                    if free > 0.0 && auto_count > 0 {
                        let share = free / auto_count as f32;
                        for &i in line {
                            for side in [axes.main_start(), axes.main_end()] {
                                if items[i].auto[side] {
                                    items[i].margin[side] = share;
                                }
                            }
                        }
                    }
                }
                None => {
                    for &i in line {
                        items[i].target = items[i].hypothetical;
                    }
                }
            }
        }

        for item in items.iter_mut() {
            if axes.row {
                let dims = measure(
                    &mut self.children[item.index],
                    Some(item.target),
                    None,
                    width,
                    fonts,
                    images,
                );
                item.measured_cross = dims.content.height;
            } else {
                item.measured_cross = item.cross_pre;
            }
        }

        let mut line_cross: Vec<f32> = Vec::new();
        for line in &lines {
            let mut size: f32 = 0.0;
            for &i in line {
                size = size.max(items[i].outer_cross(axes, items[i].measured_cross));
            }
            line_cross.push(size);
        }
        if lines.len() == 1 {
            if let Some(limit) = cross_limit {
                line_cross[0] = limit;
            }
        }
        let cross_gaps = cross_gap * (lines.len() - 1) as f32;
        let lines_total: f32 = line_cross.iter().sum::<f32>() + cross_gaps;
        let container_cross = cross_limit.unwrap_or(lines_total);

        let mut line_offsets = Vec::new();
        let (lead, between) = if lines.len() > 1 && cross_limit.is_some() {
            let free = container_cross - lines_total;
            if style.align_content() == Justify::Stretch {
                if free > 0.0 {
                    let share = free / lines.len() as f32;
                    for size in line_cross.iter_mut() {
                        *size += share;
                    }
                }
                (0.0, 0.0)
            } else {
                distribute(style.align_content(), free, lines.len())
            }
        } else {
            (0.0, 0.0)
        };
        let mut cursor = lead;
        for size in &line_cross {
            line_offsets.push(cursor);
            cursor += size + cross_gap + between;
        }

        let mut main_size = main_limit.unwrap_or(0.0);
        if main_limit.is_none() {
            for line in &lines {
                let gaps = main_gap * (line.len() - 1) as f32;
                let total: f32 = line
                    .iter()
                    .map(|&i| items[i].outer_main(axes, items[i].target))
                    .sum();
                main_size = main_size.max(total + gaps);
            }
        }

        for (number, line) in lines.iter().enumerate() {
            let gaps = main_gap * (line.len() - 1) as f32;
            let total: f32 = line
                .iter()
                .map(|&i| items[i].outer_main(axes, items[i].target))
                .sum();
            let (lead, between) = distribute(
                style.justify_content(),
                main_size - total - gaps,
                line.len(),
            );
            let mut position = lead;
            for &i in line {
                items[i].main_pos = position;
                position += items[i].outer_main(axes, items[i].target) + main_gap + between;
            }

            for &i in line {
                let item = &mut items[i];
                let (start, end) = (axes.cross_start(), axes.cross_end());
                let auto_count = usize::from(item.auto[start]) + usize::from(item.auto[end]);
                let outer = item.outer_cross(axes, item.measured_cross);
                let free = line_cross[number] - outer;
                item.cross_content = item.measured_cross;
                let mut offset = 0.0;
                if auto_count > 0 {
                    if free > 0.0 {
                        let share = free / auto_count as f32;
                        for side in [start, end] {
                            if item.auto[side] {
                                item.margin[side] = share;
                            }
                        }
                    }
                } else if item.align == Align::Stretch && item.cross_auto {
                    let stretched =
                        (line_cross[number] - item.cross_extras - item.margin_cross(axes)).max(0.0);
                    item.cross_content = stretched.min(item.cross_max).max(item.cross_min);
                    item.cross_forced = true;
                } else {
                    offset = match item.align {
                        Align::End => free,
                        Align::Center => free / 2.0,
                        Align::Start | Align::Stretch => 0.0,
                    };
                }
                item.cross_pos = line_offsets[number] + offset;
            }
        }

        let mut initial = Dimensions::default();
        initial.content.width = width;
        for item in &items {
            let child = &mut self.children[item.index];
            let (forced_width, forced_height) = if axes.row {
                (
                    Some(item.target),
                    if item.cross_forced {
                        Some(item.cross_content)
                    } else {
                        None
                    },
                )
            } else {
                (Some(item.cross_content), Some(item.target))
            };
            child.forced_width = forced_width;
            child.forced_height = forced_height;
            child.reset();
            child.layout(&initial, fonts, images, &mut FloatContext::default());
            child.dimensions.margin = EdgeSizes {
                top: item.margin[TOP],
                right: item.margin[RIGHT],
                bottom: item.margin[BOTTOM],
                left: item.margin[LEFT],
            };

            let outer_main = item.outer_main(axes, item.target);
            let main_position = if axes.main_reversed {
                main_size - (item.main_pos + outer_main)
            } else {
                item.main_pos
            };
            let outer_cross = item.outer_cross(axes, item.cross_content);
            let cross_position = if axes.wrap_reversed {
                container_cross - (item.cross_pos + outer_cross)
            } else {
                item.cross_pos
            };
            let (x, y) = if axes.row {
                (origin_x + main_position, origin_y + cross_position)
            } else {
                (origin_x + cross_position, origin_y + main_position)
            };
            let margin_box = child.dimensions.margin_box();
            child.translate(x - margin_box.x, y - margin_box.y);
            child.apply_relative_offset(width);
        }

        self.dimensions.content.height = if axes.row { container_cross } else { main_size };
    }
}
