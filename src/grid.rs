// SPDX-License-Identifier: GPL-3.0-only

use std::collections::{HashMap, HashSet};

use crate::flex::distribute;
use crate::float::FloatContext;
use crate::gridspec::{Line, Size, Track, TrackList};
use crate::layout::{
    block_replaced, BoxKind, Dimensions, EdgeSizes, ImageMap, Intrinsic, LayoutBox,
};
use crate::style::{Align, StyledNode};
use crate::text::Fonts;

const MAX_SPAN: usize = 100;
const MAX_TRACKS: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Placement {
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
}

#[derive(Debug, Clone, Copy)]
struct Axis {
    start: Option<usize>,
    span: usize,
}

struct Item<'a> {
    index: usize,
    order: i32,
    style: Option<&'a StyledNode<'a>>,
    rows: Axis,
    columns: Axis,
    placement: Placement,
}

struct Contribution {
    start: usize,
    span: usize,
    min: f32,
    max: f32,
}

#[derive(Clone, Copy)]
enum Space {
    Definite(f32),
    MinContent,
    MaxContent,
}

type Areas = HashMap<String, (usize, usize, usize, usize)>;

fn areas_of(style: &StyledNode) -> (Areas, usize, usize) {
    let mut areas = Areas::new();
    let Some(rows) = style.grid_areas() else {
        return (areas, 0, 0);
    };
    for (row, names) in rows.iter().enumerate() {
        for (column, name) in names.iter().enumerate() {
            if name == "." {
                continue;
            }
            let entry = areas
                .entry(name.clone())
                .or_insert((row, column, row + 1, column + 1));
            entry.0 = entry.0.min(row);
            entry.1 = entry.1.min(column);
            entry.2 = entry.2.max(row + 1);
            entry.3 = entry.3.max(column + 1);
        }
    }
    let columns = rows.first().map_or(0, Vec::len);
    (areas, rows.len(), columns)
}

fn line_index(number: i32, explicit: usize) -> usize {
    if number > 0 {
        (number - 1) as usize
    } else {
        (explicit as i32 + 1 + number).max(0) as usize
    }
}

fn resolve_axis(start: &Line, end: &Line, explicit: usize, areas: &Areas, rows: bool) -> Axis {
    enum Kind {
        Auto,
        Index(usize),
        Span(usize),
    }
    let kind = |line: &Line, is_end: bool| -> Kind {
        match line {
            Line::Auto => Kind::Auto,
            Line::Number(number) => Kind::Index(line_index(*number, explicit)),
            Line::Span(count) => Kind::Span((*count as usize).min(MAX_SPAN)),
            Line::Area(name) => match areas.get(name) {
                Some(area) => {
                    let (first, second) = if rows {
                        (area.0, area.2)
                    } else {
                        (area.1, area.3)
                    };
                    Kind::Index(if is_end { second } else { first })
                }
                None => Kind::Auto,
            },
        }
    };
    let (start_kind, end_kind) = (kind(start, false), kind(end, true));
    let (begin, span) = match (start_kind, end_kind) {
        (Kind::Index(a), Kind::Index(b)) => {
            let (low, high) = if a <= b { (a, b) } else { (b, a) };
            (Some(low), (high - low).max(1))
        }
        (Kind::Index(a), Kind::Span(n)) => (Some(a), n),
        (Kind::Index(a), _) => (Some(a), 1),
        (Kind::Span(n), Kind::Index(b)) => (Some(b.saturating_sub(n)), n),
        (_, Kind::Index(b)) => (Some(b.saturating_sub(1)), 1),
        (Kind::Span(n), _) | (_, Kind::Span(n)) => (None, n),
        _ => (None, 1),
    };
    Axis {
        start: begin,
        span: span.clamp(1, MAX_SPAN),
    }
}

fn resolve_size(size: Size, base: Option<f32>, font_size: f32) -> Option<f32> {
    match size {
        Size::Px(px) => Some(px),
        Size::Em(em) => Some(em * font_size),
        Size::Percent(percent) => base.map(|base| percent / 100.0 * base),
        _ => None,
    }
}

fn is_intrinsic_min(size: Size, base: Option<f32>) -> bool {
    match size {
        Size::Auto | Size::MinContent | Size::MaxContent | Size::Fr(_) => true,
        Size::Percent(_) => base.is_none(),
        _ => false,
    }
}

fn fits(
    occupied: &HashSet<(usize, usize)>,
    row: usize,
    column: usize,
    rows: usize,
    columns: usize,
) -> bool {
    for r in row..row + rows {
        for c in column..column + columns {
            if occupied.contains(&(r, c)) {
                return false;
            }
        }
    }
    true
}

fn mark(occupied: &mut HashSet<(usize, usize)>, placement: &Placement) {
    for r in placement.row..placement.row + placement.rows {
        for c in placement.column..placement.column + placement.columns {
            occupied.insert((r, c));
        }
    }
}

fn place_items(
    items: &mut [Item],
    explicit_rows: usize,
    explicit_columns: usize,
    column_flow: bool,
    dense: bool,
) {
    let mut occupied: HashSet<(usize, usize)> = HashSet::new();
    let minor_explicit = if column_flow {
        explicit_rows
    } else {
        explicit_columns
    };
    let axes = |item: &Item| -> (Axis, Axis) {
        if column_flow {
            (item.columns, item.rows)
        } else {
            (item.rows, item.columns)
        }
    };
    let store =
        |item: &mut Item, major: usize, minor: usize, major_span: usize, minor_span: usize| {
            item.placement = if column_flow {
                Placement {
                    row: minor,
                    column: major,
                    rows: minor_span,
                    columns: major_span,
                }
            } else {
                Placement {
                    row: major,
                    column: minor,
                    rows: major_span,
                    columns: minor_span,
                }
            };
        };
    let mut minor_limit = minor_explicit.max(1);
    for item in items.iter() {
        let minor = axes(item).1;
        if let Some(start) = minor.start {
            minor_limit = minor_limit.max(start + minor.span);
        }
        minor_limit = minor_limit.max(minor.span);
    }

    for item in items.iter_mut() {
        let (major, minor) = axes(item);
        if let (Some(major_start), Some(minor_start)) = (major.start, minor.start) {
            store(item, major_start, minor_start, major.span, minor.span);
            mark(&mut occupied, &item.placement);
        }
    }

    for item in items.iter_mut() {
        let (major, minor) = axes(item);
        if let (Some(major_start), None) = (major.start, minor.start) {
            let mut minor_start = 0;
            while !fits_in(
                &occupied,
                column_flow,
                major_start,
                minor_start,
                major.span,
                minor.span,
            ) {
                minor_start += 1;
                if minor_start > MAX_TRACKS {
                    break;
                }
            }
            minor_limit = minor_limit.max(minor_start + minor.span);
            store(item, major_start, minor_start, major.span, minor.span);
            mark(&mut occupied, &item.placement);
        }
    }

    let (mut cursor_major, mut cursor_minor) = (0usize, 0usize);
    for item in items.iter_mut() {
        let (major, minor) = axes(item);
        if major.start.is_some() {
            continue;
        }
        if dense {
            cursor_major = 0;
            cursor_minor = 0;
        }
        if let Some(minor_start) = minor.start {
            if !dense && cursor_minor > minor_start {
                cursor_major += 1;
            }
            let mut row = cursor_major;
            while !fits_in(
                &occupied,
                column_flow,
                row,
                minor_start,
                major.span,
                minor.span,
            ) {
                row += 1;
                if row > MAX_TRACKS {
                    break;
                }
            }
            cursor_major = row;
            store(item, row, minor_start, major.span, minor.span);
            cursor_minor = minor_start + minor.span;
        } else {
            let mut row = cursor_major;
            let mut column = cursor_minor;
            let limit = minor_limit.max(minor.span);
            loop {
                if column + minor.span > limit {
                    row += 1;
                    column = 0;
                }
                if fits_in(&occupied, column_flow, row, column, major.span, minor.span) {
                    break;
                }
                column += 1;
                if row > MAX_TRACKS {
                    break;
                }
            }
            cursor_major = row;
            cursor_minor = column + minor.span;
            store(item, row, column, major.span, minor.span);
        }
        mark(&mut occupied, &item.placement);
    }
}

fn fits_in(
    occupied: &HashSet<(usize, usize)>,
    column_flow: bool,
    major: usize,
    minor: usize,
    major_span: usize,
    minor_span: usize,
) -> bool {
    if column_flow {
        fits(occupied, minor, major, minor_span, major_span)
    } else {
        fits(occupied, major, minor, major_span, minor_span)
    }
}

fn expand_template(
    list: Option<&TrackList>,
    gap: f32,
    space: Option<f32>,
    font_size: f32,
) -> (Vec<Track>, Vec<bool>) {
    let Some(list) = list else {
        return (Vec::new(), Vec::new());
    };
    let Some(repeat) = &list.repeat else {
        let flags = vec![false; list.tracks.len()];
        return (list.tracks.clone(), flags);
    };
    let fixed = |track: &Track| {
        resolve_size(track.min, space, font_size)
            .or_else(|| resolve_size(track.max, space, font_size))
    };
    let repeated: Option<f32> = repeat.tracks.iter().map(fixed).sum::<Option<f32>>();
    let others: Option<f32> = list.tracks.iter().map(fixed).sum::<Option<f32>>();
    let count = match (space, repeated, others) {
        (Some(space), Some(repeated), Some(others))
            if repeated + gap * repeat.tracks.len() as f32 > 0.0 =>
        {
            let available = space - others - gap * (list.tracks.len() as f32 - 1.0);
            ((available / (repeated + gap * repeat.tracks.len() as f32))
                .floor()
                .max(1.0)) as usize
        }
        _ => 1,
    }
    .min(MAX_TRACKS / repeat.tracks.len().max(1));
    let mut tracks = list.tracks[..repeat.at].to_vec();
    let mut flags = vec![false; repeat.at];
    for _ in 0..count {
        tracks.extend(repeat.tracks.iter().copied());
        flags.extend(std::iter::repeat_n(repeat.fit, repeat.tracks.len()));
    }
    tracks.extend(list.tracks[repeat.at..].iter().copied());
    flags.extend(std::iter::repeat_n(false, list.tracks.len() - repeat.at));
    (tracks, flags)
}

fn size_tracks(
    tracks: &[Track],
    collapsed: &[bool],
    contributions: &[Contribution],
    gap: f32,
    space: Space,
    stretch: bool,
    font_size: f32,
) -> Vec<f32> {
    let count = tracks.len();
    let live = collapsed.iter().filter(|flag| !**flag).count();
    let total_gap = gap * live.saturating_sub(1) as f32;
    let base_space = match space {
        Space::Definite(space) => Some(space),
        _ => None,
    };
    let mut base = vec![0.0f32; count];
    let mut limit = vec![f32::INFINITY; count];
    let mut item_limit = vec![0.0f32; count];
    for (index, track) in tracks.iter().enumerate() {
        if collapsed[index] {
            limit[index] = 0.0;
            continue;
        }
        base[index] = resolve_size(track.min, base_space, font_size).unwrap_or(0.0);
        limit[index] = match track.max {
            Size::Auto | Size::MaxContent | Size::Fr(_) => f32::INFINITY,
            Size::MinContent => 0.0,
            fixed => resolve_size(fixed, base_space, font_size).unwrap_or(f32::INFINITY),
        };
        if limit[index] < base[index] {
            limit[index] = base[index];
        }
    }
    let intrinsic_min = |track: &Track| is_intrinsic_min(track.min, base_space);
    let intrinsic_max = |track: &Track| {
        matches!(track.max, Size::Auto | Size::MaxContent | Size::Fr(_))
            || (matches!(track.max, Size::Percent(_)) && base_space.is_none())
    };
    for item in contributions.iter().filter(|item| item.span == 1) {
        let track = &tracks[item.start];
        if collapsed[item.start] {
            continue;
        }
        if intrinsic_min(track) {
            let wanted = if track.min == Size::MaxContent {
                item.max
            } else {
                item.min
            };
            base[item.start] = base[item.start].max(wanted);
        }
        if intrinsic_max(track) {
            item_limit[item.start] = item_limit[item.start].max(item.max);
        }
    }
    for item in contributions.iter().filter(|item| item.span > 1) {
        let range = item.start..(item.start + item.span).min(count);
        let live_in: Vec<usize> = range.clone().filter(|i| !collapsed[*i]).collect();
        if live_in.is_empty() {
            continue;
        }
        let inner_gap = gap * (live_in.len() - 1) as f32;
        let flexible: Vec<usize> = live_in
            .iter()
            .copied()
            .filter(|i| intrinsic_min(&tracks[*i]))
            .collect();
        let current: f32 = live_in.iter().map(|i| base[*i]).sum::<f32>() + inner_gap;
        if item.min > current && !flexible.is_empty() {
            let extra = (item.min - current) / flexible.len() as f32;
            for index in &flexible {
                base[*index] += extra;
            }
        }
        let growable: Vec<usize> = live_in
            .iter()
            .copied()
            .filter(|i| intrinsic_max(&tracks[*i]))
            .collect();
        let current_max: f32 = live_in
            .iter()
            .map(|i| base[*i].max(item_limit[*i]))
            .sum::<f32>()
            + inner_gap;
        if item.max > current_max && !growable.is_empty() {
            let extra = (item.max - current_max) / growable.len() as f32;
            for index in &growable {
                item_limit[*index] = item_limit[*index].max(base[*index]) + extra;
            }
        }
    }
    for index in 0..count {
        if limit[index].is_infinite() {
            limit[index] = base[index].max(item_limit[index]);
        }
    }
    let flex: Vec<f32> = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| match track.max {
            Size::Fr(factor) if !collapsed[index] => factor,
            _ => 0.0,
        })
        .collect();
    let has_flex = flex.iter().any(|factor| *factor > 0.0);

    let mut sizes = base.clone();
    match space {
        Space::MinContent => {}
        Space::MaxContent => {
            for index in 0..count {
                if flex[index] == 0.0 {
                    sizes[index] = limit[index].max(base[index]);
                }
            }
            let fr_size = (0..count)
                .filter(|i| flex[*i] > 0.0)
                .map(|i| limit[i] / flex[i])
                .fold(0.0, f32::max);
            for index in 0..count {
                if flex[index] > 0.0 {
                    sizes[index] = (fr_size * flex[index]).max(base[index]);
                }
            }
        }
        Space::Definite(available) => {
            let mut free = available - total_gap - sizes.iter().sum::<f32>();
            let mut growing: Vec<usize> = (0..count)
                .filter(|i| flex[*i] == 0.0 && sizes[*i] < limit[*i])
                .collect();
            while free > 0.01 && !growing.is_empty() {
                let share = free / growing.len() as f32;
                let mut used = 0.0;
                growing.retain(|index| {
                    let room = limit[*index] - sizes[*index];
                    let grow = share.min(room);
                    sizes[*index] += grow;
                    used += grow;
                    room > share
                });
                if used <= 0.0 {
                    break;
                }
                free -= used;
            }
            if has_flex {
                let mut frozen = vec![false; count];
                let mut fr_size = 0.0;
                for _ in 0..=count {
                    let fixed_total: f32 = (0..count)
                        .filter(|i| flex[*i] == 0.0 || frozen[*i])
                        .map(|i| sizes[i])
                        .sum();
                    let flex_total: f32 = (0..count)
                        .filter(|i| flex[*i] > 0.0 && !frozen[*i])
                        .map(|i| flex[i])
                        .sum();
                    let leftover = (available - total_gap - fixed_total).max(0.0);
                    fr_size = leftover / flex_total.max(1.0);
                    let mut violated = false;
                    for index in 0..count {
                        if flex[index] > 0.0
                            && !frozen[index]
                            && base[index] > fr_size * flex[index]
                        {
                            frozen[index] = true;
                            sizes[index] = base[index];
                            violated = true;
                        }
                    }
                    if !violated {
                        break;
                    }
                }
                for index in 0..count {
                    if flex[index] > 0.0 && !frozen[index] {
                        sizes[index] = fr_size * flex[index];
                    }
                }
            } else if stretch {
                let free = available - total_gap - sizes.iter().sum::<f32>();
                let autos: Vec<usize> = (0..count)
                    .filter(|i| !collapsed[*i] && tracks[*i].max == Size::Auto)
                    .collect();
                if free > 0.0 && !autos.is_empty() {
                    let share = free / autos.len() as f32;
                    for index in autos {
                        sizes[index] += share;
                    }
                }
            }
        }
    }
    sizes
}

fn positions(sizes: &[f32], collapsed: &[bool], gap: f32, start: f32, extra_gap: f32) -> Vec<f32> {
    let mut result = Vec::with_capacity(sizes.len());
    let mut cursor = start;
    let last_live = collapsed.iter().rposition(|flag| !*flag);
    for (index, size) in sizes.iter().enumerate() {
        result.push(cursor);
        cursor += size;
        if !collapsed[index] && Some(index) != last_live {
            cursor += gap + extra_gap;
        }
    }
    result
}

fn span_size(
    sizes: &[f32],
    collapsed: &[bool],
    gap: f32,
    extra_gap: f32,
    start: usize,
    span: usize,
) -> f32 {
    let end = (start + span).min(sizes.len());
    let live: Vec<usize> = (start..end).filter(|i| !collapsed[*i]).collect();
    if live.is_empty() {
        return 0.0;
    }
    live.iter().map(|i| sizes[*i]).sum::<f32>() + (gap + extra_gap) * (live.len() - 1) as f32
}

struct Geometry {
    columns: Vec<Track>,
    rows: Vec<Track>,
    column_collapsed: Vec<bool>,
    row_collapsed: Vec<bool>,
}

impl<'a> LayoutBox<'a> {
    fn grid_setup(
        &self,
        style: &StyledNode,
        width: f32,
        height: Option<f32>,
        column_gap: f32,
        row_gap: f32,
    ) -> (Vec<Item<'a>>, Geometry) {
        let font_size = style.font_size();
        let (areas, area_rows, area_columns) = areas_of(style);
        let (mut columns, column_flags) = expand_template(
            style.grid_tracks("grid-template-columns"),
            column_gap,
            Some(width),
            font_size,
        );
        let (mut rows, row_flags) = expand_template(
            style.grid_tracks("grid-template-rows"),
            row_gap,
            height,
            font_size,
        );
        let mut column_flags = column_flags;
        let mut row_flags = row_flags;
        while columns.len() < area_columns {
            columns.push(Track {
                min: Size::Auto,
                max: Size::Auto,
            });
            column_flags.push(false);
        }
        while rows.len() < area_rows {
            rows.push(Track {
                min: Size::Auto,
                max: Size::Auto,
            });
            row_flags.push(false);
        }
        let explicit_columns = columns.len();
        let explicit_rows = rows.len();

        let mut items: Vec<Item<'a>> = Vec::new();
        for (index, child) in self.children.iter().enumerate() {
            let child_style: Option<&'a StyledNode<'a>> = match &child.kind {
                BoxKind::Block(child_style) => {
                    if child_style.is_out_of_flow() {
                        continue;
                    }
                    Some(*child_style)
                }
                BoxKind::Anonymous(_) => None,
            };
            let (row_axis, column_axis, order) = match child_style {
                Some(child_style) => (
                    resolve_axis(
                        &child_style.grid_line("grid-row-start"),
                        &child_style.grid_line("grid-row-end"),
                        explicit_rows,
                        &areas,
                        true,
                    ),
                    resolve_axis(
                        &child_style.grid_line("grid-column-start"),
                        &child_style.grid_line("grid-column-end"),
                        explicit_columns,
                        &areas,
                        false,
                    ),
                    child_style.order(),
                ),
                None => (
                    Axis {
                        start: None,
                        span: 1,
                    },
                    Axis {
                        start: None,
                        span: 1,
                    },
                    0,
                ),
            };
            items.push(Item {
                index,
                order,
                style: child_style,
                rows: row_axis,
                columns: column_axis,
                placement: Placement {
                    row: 0,
                    column: 0,
                    rows: 1,
                    columns: 1,
                },
            });
        }
        items.sort_by_key(|item| item.order);
        place_items(
            &mut items,
            explicit_rows,
            explicit_columns,
            style.grid_flow_column(),
            style.grid_dense(),
        );

        let total_rows = items
            .iter()
            .map(|item| item.placement.row + item.placement.rows)
            .max()
            .unwrap_or(0)
            .max(explicit_rows)
            .min(MAX_TRACKS);
        let total_columns = items
            .iter()
            .map(|item| item.placement.column + item.placement.columns)
            .max()
            .unwrap_or(0)
            .max(explicit_columns)
            .min(MAX_TRACKS);
        let implicit = |list: Option<&TrackList>| -> Vec<Track> {
            match list {
                Some(list) if !list.tracks.is_empty() => list.tracks.clone(),
                _ => vec![Track {
                    min: Size::Auto,
                    max: Size::Auto,
                }],
            }
        };
        let auto_columns = implicit(style.grid_tracks("grid-auto-columns"));
        let auto_rows = implicit(style.grid_tracks("grid-auto-rows"));
        let mut index = 0;
        while columns.len() < total_columns {
            columns.push(auto_columns[index % auto_columns.len()]);
            column_flags.push(false);
            index += 1;
        }
        index = 0;
        while rows.len() < total_rows {
            rows.push(auto_rows[index % auto_rows.len()]);
            row_flags.push(false);
            index += 1;
        }
        let collapse = |flags: &[bool], used: Vec<bool>| -> Vec<bool> {
            flags
                .iter()
                .zip(used)
                .map(|(fit, used)| *fit && !used)
                .collect()
        };
        let mut used_columns = vec![false; columns.len()];
        let mut used_rows = vec![false; rows.len()];
        for item in &items {
            let placement = item.placement;
            for used in used_columns
                .iter_mut()
                .skip(placement.column)
                .take(placement.columns)
            {
                *used = true;
            }
            for used in used_rows
                .iter_mut()
                .skip(placement.row)
                .take(placement.rows)
            {
                *used = true;
            }
        }
        let geometry = Geometry {
            column_collapsed: collapse(&column_flags, used_columns),
            row_collapsed: collapse(&row_flags, used_rows),
            columns,
            rows,
        };
        (items, geometry)
    }

    pub(crate) fn grid_intrinsic_width(
        &self,
        style: &StyledNode,
        mode: Intrinsic,
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> f32 {
        let column_gap = style.px("column-gap", base);
        let row_gap = style.px("row-gap", base);
        let (items, geometry) = self.grid_setup(style, base, None, column_gap, row_gap);
        let contributions = self.column_contributions(&items, fonts, images, base);
        let space = match mode {
            Intrinsic::Min => Space::MinContent,
            Intrinsic::Max => Space::MaxContent,
        };
        let sizes = size_tracks(
            &geometry.columns,
            &geometry.column_collapsed,
            &contributions,
            column_gap,
            space,
            false,
            style.font_size(),
        );
        span_size(
            &sizes,
            &geometry.column_collapsed,
            column_gap,
            0.0,
            0,
            sizes.len(),
        )
    }

    fn column_contributions(
        &self,
        items: &[Item],
        fonts: &Fonts,
        images: &ImageMap,
        base: f32,
    ) -> Vec<Contribution> {
        items
            .iter()
            .map(|item| {
                let child = &self.children[item.index];
                let extras = match item.style {
                    Some(style) => crate::layout::horizontal_extras(style, base),
                    None => 0.0,
                };
                Contribution {
                    start: item.placement.column,
                    span: item.placement.columns,
                    min: child.intrinsic_width(Intrinsic::Min, fonts, images, base) + extras,
                    max: child.intrinsic_width(Intrinsic::Max, fonts, images, base) + extras,
                }
            })
            .collect()
    }

    pub(crate) fn layout_grid_children(
        &mut self,
        style: &StyledNode,
        fonts: &Fonts,
        images: &ImageMap,
    ) {
        let width = self.dimensions.content.width;
        let origin_x = self.dimensions.content.x;
        let origin_y = self.dimensions.content.y;
        let height = style.length_or_auto("height", None, None);
        let column_gap = style.px("column-gap", width);
        let row_gap = style.px("row-gap", width);
        let font_size = style.font_size();

        for child in &mut self.children {
            if let BoxKind::Block(child_style) = &child.kind {
                if child_style.is_out_of_flow() {
                    child.static_position = Some((origin_x, origin_y));
                    continue;
                }
            }
            child.formatting_root = true;
        }

        let (items, geometry) = self.grid_setup(style, width, height, column_gap, row_gap);
        if items.is_empty() {
            self.dimensions.content.height = 0.0;
            return;
        }
        let column_stretch = style.content_distribution("justify-content").is_none();
        let row_stretch = style.content_distribution("align-content").is_none();

        let contributions = self.column_contributions(&items, fonts, images, width);
        let column_sizes = size_tracks(
            &geometry.columns,
            &geometry.column_collapsed,
            &contributions,
            column_gap,
            Space::Definite(width),
            column_stretch,
            font_size,
        );

        let container_justify = style.justify_items();
        let container_align = style.align_items();
        let mut row_contributions = Vec::with_capacity(items.len());
        for item in &items {
            let area_width = span_size(
                &column_sizes,
                &geometry.column_collapsed,
                column_gap,
                0.0,
                item.placement.column,
                item.placement.columns,
            );
            let justify = item
                .style
                .and_then(|s| s.justify_self())
                .unwrap_or(container_justify);
            let child = &mut self.children[item.index];
            let item_width =
                item_content_width(child, item.style, area_width, justify, fonts, images);
            child.forced_width = Some(item_width);
            child.forced_height = None;
            child.reset();
            let mut initial = Dimensions::default();
            initial.content.width = area_width;
            child.layout(&initial, fonts, images, &mut FloatContext::default());
            let margins = margins_of(item.style, area_width);
            let outer = child.dimensions.border_box().height + margins.top + margins.bottom;
            row_contributions.push(Contribution {
                start: item.placement.row,
                span: item.placement.rows,
                min: outer,
                max: outer,
            });
        }
        let row_space = match height {
            Some(height) => Space::Definite(height),
            None => Space::MaxContent,
        };
        let row_sizes = size_tracks(
            &geometry.rows,
            &geometry.row_collapsed,
            &row_contributions,
            row_gap,
            row_space,
            row_stretch,
            font_size,
        );

        let column_total = span_size(
            &column_sizes,
            &geometry.column_collapsed,
            column_gap,
            0.0,
            0,
            column_sizes.len(),
        );
        let row_total = span_size(
            &row_sizes,
            &geometry.row_collapsed,
            row_gap,
            0.0,
            0,
            row_sizes.len(),
        );
        let live_columns = geometry.column_collapsed.iter().filter(|c| !**c).count();
        let live_rows = geometry.row_collapsed.iter().filter(|c| !**c).count();
        let (column_offset, column_extra) = match style.content_distribution("justify-content") {
            Some(kind) => distribute(kind, width - column_total, live_columns),
            None => (0.0, 0.0),
        };
        let (row_offset, row_extra) = match (style.content_distribution("align-content"), height) {
            (Some(kind), Some(height)) => distribute(kind, height - row_total, live_rows),
            _ => (0.0, 0.0),
        };
        let column_positions = positions(
            &column_sizes,
            &geometry.column_collapsed,
            column_gap,
            origin_x + column_offset,
            column_extra,
        );
        let row_positions = positions(
            &row_sizes,
            &geometry.row_collapsed,
            row_gap,
            origin_y + row_offset,
            row_extra,
        );

        for item in &items {
            let placement = item.placement;
            let area_width = span_size(
                &column_sizes,
                &geometry.column_collapsed,
                column_gap,
                column_extra,
                placement.column,
                placement.columns,
            );
            let area_height = span_size(
                &row_sizes,
                &geometry.row_collapsed,
                row_gap,
                row_extra,
                placement.row,
                placement.rows,
            );
            let area_x = column_positions[placement.column];
            let area_y = row_positions[placement.row];
            let justify = item
                .style
                .and_then(|s| s.justify_self())
                .unwrap_or(container_justify);
            let align = item
                .style
                .and_then(|s| s.align_self())
                .unwrap_or(container_align);
            let child = &mut self.children[item.index];
            place_item(
                child,
                item.style,
                (area_x, area_y, area_width, area_height),
                (justify, align),
                width,
                fonts,
                images,
            );
        }

        let rows_height = row_total + row_extra * live_rows.saturating_sub(1) as f32;
        self.dimensions.content.height = match height {
            Some(height) => height.max(rows_height),
            None => rows_height,
        }
        .max(rows_height + row_offset.min(0.0));
    }
}

fn margins_of(style: Option<&StyledNode>, base: f32) -> EdgeSizes {
    match style {
        Some(style) => EdgeSizes {
            top: style.px("margin-top", base),
            right: style.px("margin-right", base),
            bottom: style.px("margin-bottom", base),
            left: style.px("margin-left", base),
        },
        None => EdgeSizes::default(),
    }
}

fn is_auto_margin(style: Option<&StyledNode>, name: &str) -> bool {
    match style {
        Some(style) => matches!(
            style.value(name),
            Some(crate::css::Value::Keyword(keyword)) if keyword == "auto"
        ),
        None => false,
    }
}

fn item_content_width(
    child: &LayoutBox,
    style: Option<&StyledNode>,
    area_width: f32,
    justify: Align,
    fonts: &Fonts,
    images: &ImageMap,
) -> f32 {
    let Some(style) = style else {
        return area_width;
    };
    let margins = margins_of(Some(style), area_width);
    let extras = style.px("padding-left", area_width)
        + style.px("padding-right", area_width)
        + style.px("border-left-width", area_width)
        + style.px("border-right-width", area_width)
        + margins.left
        + margins.right;
    let replaced = block_replaced(style, images, area_width);
    let specified = match &replaced {
        Some(replaced) => Some(replaced.width),
        None => style.length_or_auto("width", None, Some(area_width)),
    };
    let auto_margin =
        is_auto_margin(Some(style), "margin-left") || is_auto_margin(Some(style), "margin-right");
    let mut width = match specified {
        Some(width) => width,
        None if justify == Align::Stretch && !auto_margin => (area_width - extras).max(0.0),
        None => child
            .preferred_width(fonts, images, area_width)
            .min((area_width - extras).max(0.0)),
    };
    if let Some(max) = style.length_or_auto("max-width", None, Some(area_width)) {
        width = width.min(max);
    }
    if let Some(min) = style.length_or_auto("min-width", None, Some(area_width)) {
        width = width.max(min);
    }
    width
}

fn place_item(
    child: &mut LayoutBox,
    style: Option<&StyledNode>,
    area: (f32, f32, f32, f32),
    alignment: (Align, Align),
    container_width: f32,
    fonts: &Fonts,
    images: &ImageMap,
) {
    let (area_x, area_y, area_width, area_height) = area;
    let (justify, align) = alignment;
    let margins = margins_of(style, area_width);
    let width = item_content_width(child, style, area_width, justify, fonts, images);
    let mut forced_height = None;
    if let Some(style) = style {
        let vertical = style.px("padding-top", area_width)
            + style.px("padding-bottom", area_width)
            + style.px("border-top-width", area_width)
            + style.px("border-bottom-width", area_width)
            + margins.top
            + margins.bottom;
        let replaced = block_replaced(style, images, area_width);
        let specified = replaced.is_some()
            || style
                .length_or_auto("height", None, Some(area_height))
                .is_some();
        let auto_margin = is_auto_margin(Some(style), "margin-top")
            || is_auto_margin(Some(style), "margin-bottom");
        if !specified && align == Align::Stretch && !auto_margin {
            let mut height = (area_height - vertical).max(0.0);
            if let Some(max) = style.length_or_auto("max-height", None, Some(area_height)) {
                height = height.min(max);
            }
            if let Some(min) = style.length_or_auto("min-height", None, Some(area_height)) {
                height = height.max(min);
            }
            forced_height = Some(height);
        }
    } else if align == Align::Stretch {
        forced_height = None;
    }
    child.forced_width = Some(width);
    child.forced_height = forced_height;
    child.reset();
    let mut initial = Dimensions::default();
    initial.content.width = area_width;
    child.layout(&initial, fonts, images, &mut FloatContext::default());
    child.dimensions.margin = margins;

    let border_box = child.dimensions.border_box();
    let free_x = area_width - border_box.width - margins.left - margins.right;
    let free_y = area_height - border_box.height - margins.top - margins.bottom;
    let (auto_left, auto_right) = (
        is_auto_margin(style, "margin-left"),
        is_auto_margin(style, "margin-right"),
    );
    let (auto_top, auto_bottom) = (
        is_auto_margin(style, "margin-top"),
        is_auto_margin(style, "margin-bottom"),
    );
    let offset_x = match (auto_left, auto_right) {
        (true, true) => free_x.max(0.0) / 2.0,
        (true, false) => free_x.max(0.0),
        (false, true) => 0.0,
        _ => match justify {
            Align::End => free_x,
            Align::Center => free_x / 2.0,
            _ => 0.0,
        },
    };
    let offset_y = match (auto_top, auto_bottom) {
        (true, true) => free_y.max(0.0) / 2.0,
        (true, false) => free_y.max(0.0),
        (false, true) => 0.0,
        _ => match align {
            Align::End => free_y,
            Align::Center => free_y / 2.0,
            _ => 0.0,
        },
    };
    let margin_box = child.dimensions.margin_box();
    child.translate(
        area_x + offset_x - margin_box.x,
        area_y + offset_y - margin_box.y,
    );
    child.apply_relative_offset(container_width);
}
