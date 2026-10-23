// SPDX-License-Identifier: GPL-3.0-only

use crate::collapse::Edge;
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

#[derive(Clone, Copy)]
struct Plan {
    suppressed: [bool; 4],
    edges: [Option<Edge>; 4],
}

impl Plan {
    const OPEN: Plan = Plan {
        suppressed: [false; 4],
        edges: [None; 4],
    };

    fn border(&self, style: &StyledNode, side: usize, base: f32) -> f32 {
        if self.suppressed[side] {
            return 0.0;
        }
        match self.edges[side] {
            Some(edge) => edge.width,
            None => style.px(BORDER_WIDTHS[side], base),
        }
    }
}

const CELL: u8 = 5;
const ROW: u8 = 4;
const ROW_GROUP: u8 = 3;
const COLUMN: u8 = 2;
const COLUMN_GROUP: u8 = 1;

struct ColumnSource<'s> {
    col: Option<&'s StyledNode<'s>>,
    group: Option<&'s StyledNode<'s>>,
    group_first: bool,
    group_last: bool,
}

fn column_sources<'s>(table: &'s StyledNode<'s>) -> Vec<ColumnSource<'s>> {
    let mut sources = Vec::new();
    for child in &table.children {
        let Content::Element(element) = child.content else {
            continue;
        };
        match element.tag_name.as_str() {
            "col" => {
                for _ in 0..span_attribute(element, "span") {
                    sources.push(ColumnSource {
                        col: Some(child),
                        group: None,
                        group_first: false,
                        group_last: false,
                    });
                }
            }
            "colgroup" => {
                let start = sources.len();
                for column in &child.children {
                    let Content::Element(col) = column.content else {
                        continue;
                    };
                    if col.tag_name == "col" {
                        for _ in 0..span_attribute(col, "span") {
                            sources.push(ColumnSource {
                                col: Some(column),
                                group: Some(child),
                                group_first: false,
                                group_last: false,
                            });
                        }
                    }
                }
                if sources.len() == start {
                    for _ in 0..span_attribute(element, "span") {
                        sources.push(ColumnSource {
                            col: None,
                            group: Some(child),
                            group_first: false,
                            group_last: false,
                        });
                    }
                }
                if sources.len() > start {
                    sources[start].group_first = true;
                    let last = sources.len() - 1;
                    sources[last].group_last = true;
                }
            }
            _ => {}
        }
        sources.truncate(MAX_SPAN);
    }
    sources
}

const BORDER_WIDTHS: [&str; 4] = [
    "border-top-width",
    "border-right-width",
    "border-bottom-width",
    "border-left-width",
];

struct Spacing {
    x: f32,
    y: f32,
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

enum Declared {
    Length(f32),
    Percent(f32),
}

fn declared_width(node: &StyledNode, fallback: Option<&StyledNode>, base: f32) -> Option<Declared> {
    for style in [Some(node), fallback].into_iter().flatten() {
        if let Some(Value::Percent(percent)) = style.value("width") {
            return Some(Declared::Percent(*percent));
        }
        if let Some(length) = style.length_or_auto("width", None, Some(base)) {
            return Some(Declared::Length(length));
        }
    }
    None
}

fn declared_columns(table: &StyledNode, base: f32) -> Vec<Option<Declared>> {
    let mut columns = Vec::new();
    for child in &table.children {
        let Content::Element(element) = child.content else {
            continue;
        };
        match element.tag_name.as_str() {
            "col" => {
                let span = span_attribute(element, "span");
                columns.extend((0..span).map(|_| declared_width(child, None, base)));
            }
            "colgroup" => {
                let mut inner = false;
                for column in &child.children {
                    let Content::Element(col) = column.content else {
                        continue;
                    };
                    if col.tag_name == "col" {
                        inner = true;
                        let span = span_attribute(col, "span");
                        columns
                            .extend((0..span).map(|_| declared_width(column, Some(child), base)));
                    }
                }
                if !inner {
                    let span = span_attribute(element, "span");
                    columns.extend((0..span).map(|_| declared_width(child, None, base)));
                }
            }
            _ => {}
        }
        columns.truncate(MAX_SPAN);
    }
    columns
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
        if let BoxKind::Block(style) = &self.kind {
            let declared = declared_columns(style, 0.0);
            for (column, width) in declared.iter().enumerate().take(grid.columns) {
                if let Some(Declared::Percent(percent)) = width {
                    let slot = &mut percents[column];
                    *slot = Some(slot.map_or(*percent, |known: f32| known.max(*percent)));
                }
            }
        }
        percents
    }

    fn fixed_widths(
        &self,
        grid: &Grid,
        style: &StyledNode,
        width: f32,
        available: f32,
        gap: f32,
    ) -> Vec<f32> {
        let mut widths: Vec<Option<f32>> = vec![None; grid.columns];
        for (column, declared) in declared_columns(style, width)
            .iter()
            .enumerate()
            .take(grid.columns)
        {
            widths[column] = match declared {
                Some(Declared::Length(length)) => Some(*length),
                Some(Declared::Percent(percent)) => Some(percent / 100.0 * width),
                None => None,
            };
        }
        for cell in grid.cells.iter().filter(|cell| cell.row == 0) {
            let BoxKind::Block(cell_style) = &self.cell_box(cell).kind else {
                continue;
            };
            let total = match cell_style.value("width") {
                Some(Value::Percent(percent)) => Some(percent / 100.0 * width),
                _ => cell_style
                    .length_or_auto("width", None, Some(width))
                    .map(|length| {
                        length
                            + cell_style.px("padding-left", width)
                            + cell_style.px("padding-right", width)
                            + cell_style.px("border-left-width", width)
                            + cell_style.px("border-right-width", width)
                    }),
            };
            let Some(total) = total else {
                continue;
            };
            let share = ((total - gap * (cell.columns - 1) as f32) / cell.columns as f32).max(0.0);
            for slot in &mut widths[cell.column..cell.column + cell.columns] {
                slot.get_or_insert(share);
            }
        }
        let assigned: f32 = widths.iter().flatten().sum();
        let free = widths.iter().filter(|slot| slot.is_none()).count();
        if free == 0 {
            let scale = if assigned > 0.0 && available > assigned {
                available / assigned
            } else {
                1.0
            };
            return widths.into_iter().flatten().map(|w| w * scale).collect();
        }
        let share = ((available - assigned) / free as f32).max(0.0);
        widths.into_iter().map(|w| w.unwrap_or(share)).collect()
    }

    fn column_fixed(&self, grid: &Grid, base: f32) -> Vec<bool> {
        let mut fixed = vec![false; grid.columns];
        if let BoxKind::Block(style) = &self.kind {
            let declared = declared_columns(style, base);
            for (column, width) in declared.iter().enumerate().take(grid.columns) {
                fixed[column] = matches!(width, Some(Declared::Length(_)));
            }
        }
        fixed
    }

    fn collapse_plan(&self, grid: &Grid, table: &StyledNode) -> Vec<Plan> {
        if !table.border_collapse() {
            return vec![Plan::OPEN; grid.cells.len()];
        }
        let table_borders = [
            table.px("border-top-width", 0.0) > 0.0,
            table.px("border-right-width", 0.0) > 0.0,
            table.px("border-bottom-width", 0.0) > 0.0,
            table.px("border-left-width", 0.0) > 0.0,
        ];
        let columns = column_sources(table);
        let rows = grid.rows.len();
        let cell_style = |cell: &Cell| match &self.cell_box(cell).kind {
            BoxKind::Block(style) => Some(*style),
            BoxKind::Anonymous(_) => None,
        };
        let row_style = |row: usize| match &self.row_box(&grid.rows[row]).kind {
            BoxKind::Block(style) => Some(*style),
            BoxKind::Anonymous(_) => None,
        };
        let group_style = |row: usize| {
            let group = grid.rows[row].group?;
            match &self.children[group].kind {
                BoxKind::Block(style) => Some(*style),
                BoxKind::Anonymous(_) => None,
            }
        };
        let first_in_group =
            |row: usize| row == 0 || grid.rows[row - 1].group != grid.rows[row].group;
        let last_in_group =
            |row: usize| row + 1 == rows || grid.rows[row + 1].group != grid.rows[row].group;
        let edge = |style: Option<&StyledNode>, side: usize, priority: u8| {
            style.map(|style| style.border_edge(side).from_source(priority))
        };
        grid.cells
            .iter()
            .map(|cell| {
                let own = cell_style(cell);
                let first_row = cell.row;
                let last_row = cell.row + cell.rows - 1;
                let first_column = cell.column;
                let end_column = cell.column + cell.columns;
                let mut edges = [None; 4];

                let mut right = vec![edge(own, 1, CELL)];
                if end_column < grid.columns {
                    let next = grid.cells.iter().find(|other| {
                        other.column == end_column
                            && other.row <= first_row
                            && first_row < other.row + other.rows
                    });
                    right.push(next.and_then(|other| edge(cell_style(other), 3, CELL)));
                    let before = &columns.get(end_column - 1);
                    right.push(before.and_then(|source| edge(source.col, 1, COLUMN)));
                    right.push(
                        before
                            .filter(|source| source.group_last)
                            .and_then(|source| edge(source.group, 1, COLUMN_GROUP)),
                    );
                    let after = &columns.get(end_column);
                    right.push(after.and_then(|source| edge(source.col, 3, COLUMN)));
                    right.push(
                        after
                            .filter(|source| source.group_first)
                            .and_then(|source| edge(source.group, 3, COLUMN_GROUP)),
                    );
                } else {
                    right.push(edge(row_style(first_row), 1, ROW));
                    right.push(edge(group_style(first_row), 1, ROW_GROUP));
                    let last = &columns.get(end_column - 1);
                    right.push(last.and_then(|source| edge(source.col, 1, COLUMN)));
                    right.push(
                        last.filter(|source| source.group_last)
                            .and_then(|source| edge(source.group, 1, COLUMN_GROUP)),
                    );
                }
                edges[1] = Some(Edge::resolve_all(
                    &right.into_iter().flatten().collect::<Vec<_>>(),
                ));

                let mut bottom = vec![
                    edge(own, 2, CELL),
                    edge(row_style(last_row), 2, ROW),
                    if last_in_group(last_row) {
                        edge(group_style(last_row), 2, ROW_GROUP)
                    } else {
                        None
                    },
                ];
                if last_row + 1 < rows {
                    let below = grid.cells.iter().find(|other| {
                        other.row == last_row + 1
                            && other.column <= first_column
                            && first_column < other.column + other.columns
                    });
                    bottom.push(below.and_then(|other| edge(cell_style(other), 0, CELL)));
                    bottom.push(edge(row_style(last_row + 1), 0, ROW));
                    if first_in_group(last_row + 1) {
                        bottom.push(edge(group_style(last_row + 1), 0, ROW_GROUP));
                    }
                } else if let Some(source) = columns.get(first_column) {
                    bottom.push(edge(source.col, 2, COLUMN));
                    bottom.push(edge(source.group, 2, COLUMN_GROUP));
                }
                edges[2] = Some(Edge::resolve_all(
                    &bottom.into_iter().flatten().collect::<Vec<_>>(),
                ));

                if first_row == 0 {
                    let mut top = vec![
                        edge(own, 0, CELL),
                        edge(row_style(0), 0, ROW),
                        edge(group_style(0), 0, ROW_GROUP),
                    ];
                    if let Some(source) = columns.get(first_column) {
                        top.push(edge(source.col, 0, COLUMN));
                        top.push(edge(source.group, 0, COLUMN_GROUP));
                    }
                    edges[0] = Some(Edge::resolve_all(
                        &top.into_iter().flatten().collect::<Vec<_>>(),
                    ));
                }
                if first_column == 0 {
                    let mut left = vec![
                        edge(own, 3, CELL),
                        edge(row_style(first_row), 3, ROW),
                        edge(group_style(first_row), 3, ROW_GROUP),
                    ];
                    if let Some(source) = columns.first() {
                        left.push(edge(source.col, 3, COLUMN));
                        left.push(edge(source.group, 3, COLUMN_GROUP));
                    }
                    edges[3] = Some(Edge::resolve_all(
                        &left.into_iter().flatten().collect::<Vec<_>>(),
                    ));
                }

                let suppressed = [
                    cell.row > 0 || table_borders[0],
                    table_borders[1],
                    table_borders[2],
                    cell.column > 0 || table_borders[3],
                ];
                for side in 0..4 {
                    if suppressed[side] {
                        edges[side] = None;
                    }
                }
                Plan { suppressed, edges }
            })
            .collect()
    }

    fn cell_alignment(&self, cell: &Cell, grid: &Grid) -> String {
        let own = match &self.cell_box(cell).kind {
            BoxKind::Block(style) => style.vertical_align().map(str::to_string),
            BoxKind::Anonymous(_) => None,
        };
        let row = match &self.row_box(&grid.rows[cell.row]).kind {
            BoxKind::Block(style) => style.vertical_align().map(str::to_string),
            BoxKind::Anonymous(_) => None,
        };
        let group = grid.rows[cell.row]
            .group
            .and_then(|group| match &self.children[group].kind {
                BoxKind::Block(style) => style.vertical_align().map(str::to_string),
                BoxKind::Anonymous(_) => None,
            });
        own.or(row)
            .or(group)
            .unwrap_or_else(|| "middle".to_string())
    }

    fn first_baseline(&self) -> Option<f32> {
        let own = self
            .fragments
            .iter()
            .map(|fragment| fragment.baseline)
            .chain(
                self.images
                    .iter()
                    .map(|image| image.rect.y + image.rect.height),
            )
            .reduce(f32::min);
        own.or_else(|| self.children.iter().find_map(LayoutBox::first_baseline))
    }

    fn collapsed_columns(&self, grid: &Grid) -> Vec<bool> {
        let mut hidden = vec![false; grid.columns];
        if let BoxKind::Block(table) = &self.kind {
            let sources = column_sources(table);
            for (column, source) in sources.iter().enumerate().take(grid.columns) {
                hidden[column] = source.col.is_some_and(StyledNode::collapsed)
                    || source.group.is_some_and(StyledNode::collapsed);
            }
        }
        hidden
    }

    fn collapsed_rows(&self, grid: &Grid) -> Vec<bool> {
        grid.rows
            .iter()
            .map(|row| {
                let own =
                    matches!(&self.row_box(row).kind, BoxKind::Block(style) if style.collapsed());
                let group = row.group.is_some_and(|group| {
                    matches!(&self.children[group].kind, BoxKind::Block(style) if style.collapsed())
                });
                own || group
            })
            .collect()
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
        plans: &[Plan],
        gap: f32,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> (Vec<f32>, Vec<f32>) {
        let mut minimum = vec![0.0f32; grid.columns];
        let mut maximum = vec![0.0f32; grid.columns];
        let mut measured = Vec::new();
        for (cell, plan) in grid.cells.iter().zip(plans) {
            let cell_box = self.cell_box(cell);
            let BoxKind::Block(style) = &cell_box.kind else {
                continue;
            };
            let extras = style.px("padding-left", base)
                + style.px("padding-right", base)
                + plan.border(style, 3, base)
                + plan.border(style, 1, base);
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
        if let BoxKind::Block(style) = &self.kind {
            let declared = declared_columns(style, base);
            for (column, width) in declared.iter().enumerate().take(grid.columns) {
                if let Some(Declared::Length(length)) = width {
                    minimum[column] = minimum[column].max(*length);
                    maximum[column] = maximum[column].max(*length);
                }
            }
        }
        for (column, hidden) in self.collapsed_columns(grid).into_iter().enumerate() {
            if hidden {
                minimum[column] = 0.0;
                maximum[column] = 0.0;
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
        let plans = self.collapse_plan(&grid, style);
        let (minimum, maximum) = self.column_limits(&grid, &plans, gaps.x, fonts, images, base);
        let widths = match mode {
            Intrinsic::Min => minimum,
            Intrinsic::Max => maximum,
        };
        let live = self
            .collapsed_columns(&grid)
            .iter()
            .filter(|hidden| !**hidden)
            .count();
        (widths.iter().sum::<f32>() + gaps.x * (live + 1) as f32).max(caption)
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
        let plans = self.collapse_plan(&grid, style);
        let column_hidden = self.collapsed_columns(&grid);
        let row_hidden = self.collapsed_rows(&grid);
        let columns = column_hidden
            .iter()
            .filter(|hidden| !**hidden)
            .count()
            .max(1);
        let available = (width - gaps.x * (columns + 1) as f32).max(0.0);
        let mut widths = if style.table_layout_fixed() {
            self.fixed_widths(&grid, style, width, available, gaps.x)
        } else {
            let (minimum, maximum) =
                self.column_limits(&grid, &plans, gaps.x, fonts, images, width);
            let percents = self.column_percents(&grid);
            let fixed = self.column_fixed(&grid, width);
            let has_free =
                (0..grid.columns).any(|index| percents[index].is_none() && !fixed[index]);
            let mut widths = vec![0.0f32; grid.columns];
            let mut remaining = available;
            let mut flexible = Vec::new();
            for (index, percent) in percents.iter().enumerate() {
                match percent {
                    Some(percent) => {
                        widths[index] = (percent / 100.0 * width).max(minimum[index]);
                        remaining -= widths[index];
                    }
                    None if fixed[index] && has_free => {
                        widths[index] = minimum[index];
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
            widths
        };
        for (column_width, hidden) in widths.iter_mut().zip(&column_hidden) {
            if *hidden {
                *column_width = 0.0;
            }
        }
        let mut column_x = Vec::with_capacity(widths.len());
        let mut x = left + gaps.x;
        for (column_width, hidden) in widths.iter().zip(&column_hidden) {
            column_x.push(x);
            if !hidden {
                x += column_width + gaps.x;
            }
        }
        let rows_width = (x - gaps.x - (left + gaps.x)).max(0.0);
        let cell_hidden = |cell: &Cell| {
            row_hidden[cell.row..cell.row + cell.rows]
                .iter()
                .all(|h| *h)
                || column_hidden[cell.column..cell.column + cell.columns]
                    .iter()
                    .all(|h| *h)
        };

        let row_count = grid.rows.len();
        let mut row_heights = vec![0.0f32; row_count];
        for (index, row) in grid.rows.iter().enumerate() {
            if let BoxKind::Block(row_style) = &self.row_box(row).kind {
                row_heights[index] = row_style.px("height", 0.0);
            }
        }

        let alignments: Vec<String> = grid
            .cells
            .iter()
            .map(|cell| self.cell_alignment(cell, &grid))
            .collect();
        let mut baselines: Vec<Option<f32>> = Vec::with_capacity(grid.cells.len());
        let mut natural = Vec::with_capacity(grid.cells.len());
        let mut minimum_heights = Vec::with_capacity(grid.cells.len());
        for (index, (cell, plan)) in grid.cells.iter().zip(&plans).enumerate() {
            let column_end = cell.column + cell.columns;
            let live_columns = column_hidden[cell.column..column_end]
                .iter()
                .filter(|hidden| !**hidden)
                .count();
            let cell_width: f32 = widths[cell.column..column_end].iter().sum::<f32>()
                + gaps.x * live_columns.saturating_sub(1) as f32;
            let cell_box = self.cell_box_mut(cell);
            cell_box.reset();
            cell_box.collapsed_borders = plan.edges;
            cell_box.suppressed_borders = plan.suppressed;
            let BoxKind::Block(cell_style) = &cell_box.kind else {
                natural.push(0.0);
                minimum_heights.push(0.0);
                baselines.push(None);
                continue;
            };
            let cell_style = *cell_style;
            let extras = cell_style.px("padding-left", width)
                + cell_style.px("padding-right", width)
                + plan.border(cell_style, 3, width)
                + plan.border(cell_style, 1, width);
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
            baselines.push(if alignments[index] == "baseline" && cell.rows == 1 {
                let top = cell_box.dimensions.border_box().y;
                cell_box.first_baseline().map(|baseline| baseline - top)
            } else {
                None
            });
            let specified = cell_style
                .length_or_auto("height", None, None)
                .map(|height| {
                    let d = &cell_box.dimensions;
                    height + d.padding.top + d.padding.bottom + d.border.top + d.border.bottom
                });
            minimum_heights.push(specified.unwrap_or(0.0));
        }

        for ((cell, height), minimum) in grid.cells.iter().zip(&natural).zip(&minimum_heights) {
            if cell.rows == 1 && !cell_hidden(cell) {
                row_heights[cell.row] = row_heights[cell.row].max(height.max(*minimum));
            }
        }
        let mut row_baseline = vec![0.0f32; row_count];
        let mut row_below = vec![0.0f32; row_count];
        for ((cell, height), baseline) in grid.cells.iter().zip(&natural).zip(&baselines) {
            if let Some(baseline) = baseline.filter(|_| !cell_hidden(cell)) {
                row_baseline[cell.row] = row_baseline[cell.row].max(baseline);
                row_below[cell.row] = row_below[cell.row].max(height - baseline);
            }
        }
        for (row_height, (above, below)) in row_heights
            .iter_mut()
            .zip(row_baseline.iter().zip(&row_below))
        {
            *row_height = row_height.max(above + below);
        }
        for (row_height, hidden) in row_heights.iter_mut().zip(&row_hidden) {
            if *hidden {
                *row_height = 0.0;
            }
        }
        for (cell, height) in grid.cells.iter().zip(&natural) {
            if cell.rows == 1 || cell_hidden(cell) {
                continue;
            }
            let live: Vec<usize> = (cell.row..cell.row + cell.rows)
                .filter(|row| !row_hidden[*row])
                .collect();
            let current: f32 = live.iter().map(|row| row_heights[*row]).sum::<f32>()
                + gaps.y * (live.len() - 1) as f32;
            if *height > current {
                let extra = (height - current) / live.len() as f32;
                for row in live {
                    row_heights[row] += extra;
                }
            }
        }

        let mut row_y = Vec::with_capacity(row_count);
        let mut y = top + cursor + gaps.y;
        for (row_height, hidden) in row_heights.iter().zip(&row_hidden) {
            row_y.push(y);
            if !hidden {
                y += row_height + gaps.y;
            }
        }

        for (index, (cell, natural_height)) in grid.cells.iter().zip(&natural).enumerate() {
            let row_end = cell.row + cell.rows;
            let live_rows = row_hidden[cell.row..row_end]
                .iter()
                .filter(|hidden| !**hidden)
                .count();
            let target: f32 = row_heights[cell.row..row_end].iter().sum::<f32>()
                + gaps.y * live_rows.saturating_sub(1) as f32;
            let destination = (column_x[cell.column], row_y[cell.row]);
            let cell_box = self.cell_box_mut(cell);
            if matches!(cell_box.kind, BoxKind::Anonymous(_)) {
                continue;
            }
            let free = (target - natural_height).max(0.0);
            let shift = match alignments[index].as_str() {
                "middle" => free / 2.0,
                "bottom" => free,
                "baseline" => baselines[index].map_or(0.0, |own| row_baseline[cell.row] - own),
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
            if cell_hidden(cell) {
                cell_box.collapsed = true;
                cell_box.children.clear();
                cell_box.fragments.clear();
                cell_box.images.clear();
                cell_box.inline_boxes.clear();
                cell_box.rules.clear();
            }
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
