// SPDX-License-Identifier: GPL-3.0-only

use crate::css::Value;
use crate::dom::ElementData;
use crate::float::FloatContext;
use crate::layout::{BoxKind, Dimensions, ImageMap, Intrinsic, LayoutBox, Rect};
use crate::style::{Content, Display, StyledNode};
use crate::text::Fonts;

const MAX_SPAN: usize = 1000;

struct Row {
    group: Option<usize>,
    index: usize,
}

struct Cell {
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
    group: Option<usize>,
    row_index: usize,
    index: usize,
}

struct Grid {
    rows: Vec<Row>,
    cells: Vec<Cell>,
    columns: usize,
}

struct Spacing {
    x: f32,
    y: f32,
    collapse: bool,
}

fn spacing(style: &StyledNode) -> Spacing {
    let collapse = style.border_collapse();
    Spacing {
        x: if collapse {
            0.0
        } else {
            style.px("border-spacing-x", 0.0)
        },
        y: if collapse {
            0.0
        } else {
            style.px("border-spacing-y", 0.0)
        },
        collapse,
    }
}

fn span_attribute(element: &ElementData, name: &str) -> usize {
    element
        .attributes
        .get(name)
        .and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|value| *value >= 1)
        .unwrap_or(1)
        .min(MAX_SPAN)
}

fn display_of(layout_box: &LayoutBox) -> Option<Display> {
    match &layout_box.kind {
        BoxKind::Block(style) => Some(style.display()),
        BoxKind::Anonymous(_) => None,
    }
}

fn is_caption(layout_box: &LayoutBox) -> bool {
    match &layout_box.kind {
        BoxKind::Block(style) => {
            matches!(style.content, Content::Element(e) if e.tag_name == "caption")
        }
        BoxKind::Anonymous(_) => false,
    }
}

fn group_order(layout_box: &LayoutBox) -> u8 {
    if let BoxKind::Block(style) = &layout_box.kind {
        if let Content::Element(element) = style.content {
            return match element.tag_name.as_str() {
                "thead" => 0,
                "tfoot" => 2,
                _ => 1,
            };
        }
    }
    1
}

fn distribute(minimum: &[f32], maximum: &[f32], available: f32) -> Vec<f32> {
    let total_min: f32 = minimum.iter().sum();
    let total_max: f32 = maximum.iter().sum();
    if available >= total_max {
        let extra = available - total_max;
        return maximum
            .iter()
            .map(|width| {
                if total_max > 0.0 {
                    width + extra * width / total_max
                } else {
                    extra / maximum.len() as f32
                }
            })
            .collect();
    }
    if available <= total_min {
        return minimum.to_vec();
    }
    let share = (available - total_min) / (total_max - total_min);
    minimum
        .iter()
        .zip(maximum)
        .map(|(low, high)| low + (high - low) * share)
        .collect()
}

impl<'a> LayoutBox<'a> {
    fn table_grid(&self) -> Grid {
        let mut rows: Vec<(u8, Row)> = Vec::new();
        for (index, child) in self.children.iter().enumerate() {
            match display_of(child) {
                Some(Display::TableRowGroup) => {
                    for (row_index, row) in child.children.iter().enumerate() {
                        if display_of(row) == Some(Display::TableRow) {
                            rows.push((
                                group_order(child),
                                Row {
                                    group: Some(index),
                                    index: row_index,
                                },
                            ));
                        }
                    }
                }
                Some(Display::TableRow) => rows.push((1, Row { group: None, index })),
                _ => {}
            }
        }
        rows.sort_by_key(|(order, _)| *order);
        let rows: Vec<Row> = rows.into_iter().map(|(_, row)| row).collect();

        let mut cells = Vec::new();
        let mut occupied: Vec<Vec<bool>> = vec![Vec::new(); rows.len()];
        let mut columns = 0;
        for (row_number, row) in rows.iter().enumerate() {
            let row_box = self.row_box(row);
            let mut cursor = 0;
            for (index, cell_box) in row_box.children.iter().enumerate() {
                if display_of(cell_box) != Some(Display::TableCell) {
                    continue;
                }
                let (column_span, row_span) = match &cell_box.kind {
                    BoxKind::Block(style) => match style.content {
                        Content::Element(element) => (
                            span_attribute(element, "colspan"),
                            span_attribute(element, "rowspan"),
                        ),
                        Content::Text(_) | Content::Pseudo => (1, 1),
                    },
                    BoxKind::Anonymous(_) => (1, 1),
                };
                let row_span = row_span.min(rows.len() - row_number);
                while occupied[row_number].get(cursor).copied().unwrap_or(false) {
                    cursor += 1;
                }
                if cursor + column_span > MAX_SPAN {
                    continue;
                }
                for line in &mut occupied[row_number..row_number + row_span] {
                    if line.len() < cursor + column_span {
                        line.resize(cursor + column_span, false);
                    }
                    for slot in &mut line[cursor..cursor + column_span] {
                        *slot = true;
                    }
                }
                cells.push(Cell {
                    row: row_number,
                    column: cursor,
                    rows: row_span,
                    columns: column_span,
                    group: row.group,
                    row_index: row.index,
                    index,
                });
                columns = columns.max(cursor + column_span);
                cursor += column_span;
            }
        }
        Grid {
            rows,
            cells,
            columns,
        }
    }

    fn row_box(&self, row: &Row) -> &LayoutBox<'a> {
        match row.group {
            Some(group) => &self.children[group].children[row.index],
            None => &self.children[row.index],
        }
    }

    fn row_box_mut(&mut self, row: &Row) -> &mut LayoutBox<'a> {
        match row.group {
            Some(group) => &mut self.children[group].children[row.index],
            None => &mut self.children[row.index],
        }
    }

    fn cell_box(&self, cell: &Cell) -> &LayoutBox<'a> {
        let row = Row {
            group: cell.group,
            index: cell.row_index,
        };
        &self.row_box(&row).children[cell.index]
    }

    fn cell_box_mut(&mut self, cell: &Cell) -> &mut LayoutBox<'a> {
        let row = Row {
            group: cell.group,
            index: cell.row_index,
        };
        &mut self.row_box_mut(&row).children[cell.index]
    }

    fn column_percents(&self, grid: &Grid) -> Vec<Option<f32>> {
        let mut percents = vec![None; grid.columns];
        for cell in grid.cells.iter().filter(|cell| cell.columns == 1) {
            if let BoxKind::Block(style) = &self.cell_box(cell).kind {
                if let Some(Value::Percent(percent)) = style.value("width") {
                    let slot = &mut percents[cell.column];
                    *slot = Some(slot.map_or(*percent, |known: f32| known.max(*percent)));
                }
            }
        }
        percents
    }

    fn caption_width(&self, mode: Intrinsic, fonts: &Fonts, images: &ImageMap, base: f32) -> f32 {
        self.children
            .iter()
            .filter(|child| is_caption(child))
            .map(|child| child.intrinsic_width(mode, fonts, images, base))
            .fold(0.0, f32::max)
    }

    fn column_limits(
        &self,
        grid: &Grid,
        gap: f32,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut minimum = vec![0.0f32; grid.columns];
        let mut maximum = vec![0.0f32; grid.columns];
        let mut measured = Vec::new();
        for cell in &grid.cells {
            let cell_box = self.cell_box(cell);
            let BoxKind::Block(style) = &cell_box.kind else {
                continue;
            };
            let extras = style.px("padding-left", base)
                + style.px("padding-right", base)
                + style.px("border-left-width", base)
                + style.px("border-right-width", base);
            let low = cell_box.intrinsic_width(Intrinsic::Min, fonts, images, base) + extras;
            let high = cell_box
                .intrinsic_width(Intrinsic::Max, fonts, images, base)
                .max(0.0)
                + extras;
            measured.push((cell, low, high.max(low)));
        }
        for (cell, low, high) in &measured {
            if cell.columns == 1 {
                minimum[cell.column] = minimum[cell.column].max(*low);
                maximum[cell.column] = maximum[cell.column].max(*high);
            }
        }
        for (cell, low, high) in &measured {
            if cell.columns == 1 {
                continue;
            }
            let span = cell.column..cell.column + cell.columns;
            let inner_gap = gap * (cell.columns - 1) as f32;
            for (target, wanted) in [(&mut minimum, *low), (&mut maximum, *high)] {
                let current: f32 = target[span.clone()].iter().sum::<f32>() + inner_gap;
                if wanted > current {
                    let extra = (wanted - current) / cell.columns as f32;
                    for width in &mut target[span.clone()] {
                        *width += extra;
                    }
                }
            }
        }
        for (low, high) in minimum.iter().zip(maximum.iter_mut()) {
            *high = high.max(*low);
        }
        (minimum, maximum)
    }

    pub(crate) fn table_intrinsic_width(
        &self,
        style: &StyledNode,
        mode: Intrinsic,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> f32 {
        let grid = self.table_grid();
        let caption = self.caption_width(mode, fonts, images, base);
        if grid.columns == 0 {
            return caption;
        }
        let gaps = spacing(style);
        let (minimum, maximum) = self.column_limits(&grid, gaps.x, fonts, images, base);
        let widths = match mode {
            Intrinsic::Min => minimum,
            Intrinsic::Max => maximum,
        };
        (widths.iter().sum::<f32>() + gaps.x * (grid.columns + 1) as f32).max(caption)
    }

    pub(crate) fn table_width(
        &self,
        style: &StyledNode,
        container: f32,
        fonts: &Fonts,
        images: &ImageMap,
    ) -> f32 {
        let width = self.shrink_to_fit_width(style, container, fonts, images);
        if style
            .length_or_auto("width", None, Some(container))
            .is_some()
        {
            return width;
        }
        let minimum = self.table_intrinsic_width(style, Intrinsic::Min, fonts, images, container);
        width.max(minimum)
    }

    pub(crate) fn layout_table_children(
        &mut self,
        style: &StyledNode,
        fonts: &Fonts,
        images: &ImageMap,
    ) {
        let gaps = spacing(style);
        let width = self.dimensions.content.width;
        let left = self.dimensions.content.x;
        let top = self.dimensions.content.y;

        let mut cursor = 0.0;
        let caption = self.children.iter().position(is_caption);
        if let Some(index) = caption {
            let containing = Dimensions {
                content: Rect {
                    x: left,
                    y: top,
                    width,
                    height: 0.0,
                },
                ..Dimensions::default()
            };
            let child = &mut self.children[index];
            child.layout(&containing, fonts, images, &mut FloatContext::default());
            cursor = child.dimensions.margin_box().height;
        }

        let grid = self.table_grid();
        if grid.rows.is_empty() {
            self.dimensions.content.height = cursor;
            return;
        }
        let columns = grid.columns.max(1);
        let available = (width - gaps.x * (columns + 1) as f32).max(0.0);
        let (minimum, maximum) = self.column_limits(&grid, gaps.x, fonts, images, width);
        let percents = self.column_percents(&grid);
        let mut widths = vec![0.0f32; grid.columns];
        let mut remaining = available;
        let mut flexible = Vec::new();
        for (index, percent) in percents.iter().enumerate() {
            match percent {
                Some(percent) => {
                    widths[index] = (percent / 100.0 * width).max(minimum[index]);
                    remaining -= widths[index];
                }
                None => flexible.push(index),
            }
        }
        if !flexible.is_empty() {
            let low: Vec<f32> = flexible.iter().map(|index| minimum[*index]).collect();
            let high: Vec<f32> = flexible.iter().map(|index| maximum[*index]).collect();
            let shared = distribute(&low, &high, remaining.max(0.0));
            for (index, share) in flexible.iter().zip(shared) {
                widths[*index] = share;
            }
        }
        let mut column_x = Vec::with_capacity(widths.len());
        let mut x = left + gaps.x;
        for column_width in &widths {
            column_x.push(x);
            x += column_width + gaps.x;
        }
        let rows_width = x - gaps.x - (left + gaps.x);

        let table_borders = [
            style.px("border-top-width", 0.0) > 0.0,
            style.px("border-right-width", 0.0) > 0.0,
            style.px("border-bottom-width", 0.0) > 0.0,
            style.px("border-left-width", 0.0) > 0.0,
        ];
        let row_count = grid.rows.len();
        let mut row_heights = vec![0.0f32; row_count];
        for (index, row) in grid.rows.iter().enumerate() {
            if let BoxKind::Block(row_style) = &self.row_box(row).kind {
                row_heights[index] = row_style.px("height", 0.0);
            }
        }

        let mut natural = Vec::with_capacity(grid.cells.len());
        let mut minimum_heights = Vec::with_capacity(grid.cells.len());
        for cell in &grid.cells {
            let column_end = cell.column + cell.columns;
            let cell_width: f32 = widths[cell.column..column_end].iter().sum::<f32>()
                + gaps.x * (cell.columns - 1) as f32;
            let cell_box = self.cell_box_mut(cell);
            cell_box.reset();
            cell_box.suppressed_borders = [
                gaps.collapse && (cell.row > 0 || table_borders[0]),
                gaps.collapse && table_borders[1],
                gaps.collapse && table_borders[2],
                gaps.collapse && (cell.column > 0 || table_borders[3]),
            ];
            let BoxKind::Block(cell_style) = &cell_box.kind else {
                natural.push(0.0);
                minimum_heights.push(0.0);
                continue;
            };
            let cell_style = *cell_style;
            let suppressed = cell_box.suppressed_borders;
            let extras = cell_style.px("padding-left", width)
                + cell_style.px("padding-right", width)
                + if suppressed[3] {
                    0.0
                } else {
                    cell_style.px("border-left-width", width)
                }
                + if suppressed[1] {
                    0.0
                } else {
                    cell_style.px("border-right-width", width)
                };
            cell_box.forced_width = Some((cell_width - extras).max(0.0));
            let containing = Dimensions {
                content: Rect {
                    x: column_x[cell.column],
                    y: 0.0,
                    width: cell_width,
                    height: 0.0,
                },
                ..Dimensions::default()
            };
            cell_box.layout(&containing, fonts, images, &mut FloatContext::default());
            natural.push(cell_box.dimensions.border_box().height);
            let specified = cell_style
                .length_or_auto("height", None, None)
                .map(|height| {
                    let d = &cell_box.dimensions;
                    height + d.padding.top + d.padding.bottom + d.border.top + d.border.bottom
                });
            minimum_heights.push(specified.unwrap_or(0.0));
        }

        for ((cell, height), minimum) in grid.cells.iter().zip(&natural).zip(&minimum_heights) {
            if cell.rows == 1 {
                row_heights[cell.row] = row_heights[cell.row].max(height.max(*minimum));
            }
        }
        for (cell, height) in grid.cells.iter().zip(&natural) {
            if cell.rows == 1 {
                continue;
            }
            let span = cell.row..cell.row + cell.rows;
            let current: f32 =
                row_heights[span.clone()].iter().sum::<f32>() + gaps.y * (cell.rows - 1) as f32;
            if *height > current {
                let extra = (height - current) / cell.rows as f32;
                for row_height in &mut row_heights[span] {
                    *row_height += extra;
                }
            }
        }

        let mut row_y = Vec::with_capacity(row_count);
        let mut y = top + cursor + gaps.y;
        for row_height in &row_heights {
            row_y.push(y);
            y += row_height + gaps.y;
        }

        for (cell, natural_height) in grid.cells.iter().zip(&natural) {
            let row_end = cell.row + cell.rows;
            let target: f32 = row_heights[cell.row..row_end].iter().sum::<f32>()
                + gaps.y * (cell.rows - 1) as f32;
            let row_alignment = match &self.row_box(&grid.rows[cell.row]).kind {
                BoxKind::Block(row_style) => row_style.vertical_align().map(str::to_string),
                BoxKind::Anonymous(_) => None,
            };
            let destination = (column_x[cell.column], row_y[cell.row]);
            let cell_box = self.cell_box_mut(cell);
            let BoxKind::Block(cell_style) = &cell_box.kind else {
                continue;
            };
            let alignment = cell_style
                .vertical_align()
                .map(str::to_string)
                .or(row_alignment)
                .unwrap_or_else(|| "middle".to_string());
            let free = (target - natural_height).max(0.0);
            let shift = match alignment.as_str() {
                "middle" => free / 2.0,
                "bottom" => free,
                _ => 0.0,
            };
            if shift != 0.0 {
                for child in &mut cell_box.children {
                    child.translate(0.0, shift);
                }
            }
            let d = &mut cell_box.dimensions;
            let chrome = d.padding.top + d.padding.bottom + d.border.top + d.border.bottom;
            d.content.height = (target - chrome).max(0.0);
            let border_box = cell_box.dimensions.border_box();
            cell_box.translate(destination.0 - border_box.x, destination.1 - border_box.y);
        }

        for (index, row) in grid.rows.iter().enumerate() {
            let rect = Rect {
                x: left + gaps.x,
                y: row_y[index],
                width: rows_width,
                height: row_heights[index],
            };
            self.row_box_mut(row).dimensions.content = rect;
        }
        let group_indices: Vec<usize> = (0..self.children.len())
            .filter(|index| display_of(&self.children[*index]) == Some(Display::TableRowGroup))
            .collect();
        for group in group_indices {
            let mut top_edge = f32::MAX;
            let mut bottom_edge = f32::MIN;
            for row in &self.children[group].children {
                let content = row.dimensions.content;
                if display_of(row) == Some(Display::TableRow) {
                    top_edge = top_edge.min(content.y);
                    bottom_edge = bottom_edge.max(content.y + content.height);
                }
            }
            if top_edge <= bottom_edge {
                self.children[group].dimensions.content = Rect {
                    x: left + gaps.x,
                    y: top_edge,
                    width: rows_width,
                    height: bottom_edge - top_edge,
                };
            }
        }

        self.dimensions.content.height = y - top;
    }
}
