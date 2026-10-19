// SPDX-License-Identifier: GPL-3.0-only

use crate::media::split_terms;

const MAX_TRACKS: usize = 1000;
const REM_PX: f32 = 16.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Size {
    Px(f32),
    Em(f32),
    Percent(f32),
    Fr(f32),
    Auto,
    MinContent,
    MaxContent,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Track {
    pub min: Size,
    pub max: Size,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AutoRepeat {
    pub at: usize,
    pub tracks: Vec<Track>,
    pub fit: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrackList {
    pub tracks: Vec<Track>,
    pub repeat: Option<AutoRepeat>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Line {
    Auto,
    Number(i32),
    Span(u32),
    Area(String),
}

fn strip_line_names(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut inside = false;
    for c in text.chars() {
        match c {
            '[' => inside = true,
            ']' => {
                inside = false;
                out.push(' ');
            }
            _ if !inside => out.push(c),
            _ => {}
        }
    }
    out
}

fn parse_number(text: &str) -> Option<f32> {
    text.parse::<f32>().ok().filter(|number| number.is_finite())
}

fn parse_size(text: &str) -> Option<Size> {
    match text {
        "auto" => return Some(Size::Auto),
        "min-content" => return Some(Size::MinContent),
        "max-content" => return Some(Size::MaxContent),
        _ => {}
    }
    if let Some(number) = text.strip_suffix("fr") {
        return parse_number(number).filter(|n| *n >= 0.0).map(Size::Fr);
    }
    if let Some(number) = text.strip_suffix("px") {
        return parse_number(number).map(Size::Px);
    }
    if let Some(number) = text.strip_suffix("rem") {
        return parse_number(number).map(|n| Size::Px(n * REM_PX));
    }
    if let Some(number) = text.strip_suffix("em") {
        return parse_number(number).map(Size::Em);
    }
    if let Some(number) = text.strip_suffix('%') {
        return parse_number(number).map(Size::Percent);
    }
    match parse_number(text) {
        Some(0.0) => Some(Size::Px(0.0)),
        _ => None,
    }
}

fn parse_track(term: &str) -> Option<Track> {
    if let Some(inner) = term
        .strip_prefix("minmax(")
        .and_then(|rest| rest.strip_suffix(')'))
    {
        let (low, high) = inner.split_once(',')?;
        let min = parse_size(low.trim())?;
        let max = parse_size(high.trim())?;
        if matches!(min, Size::Fr(_)) {
            return None;
        }
        return Some(Track { min, max });
    }
    let size = parse_size(term)?;
    Some(match size {
        Size::Fr(_) => Track {
            min: Size::Auto,
            max: size,
        },
        _ => Track {
            min: size,
            max: size,
        },
    })
}

fn parse_terms(text: &str, list: &mut TrackList, allow_repeat: bool) -> Option<()> {
    for term in split_terms(text.trim()) {
        if let Some(inner) = term
            .strip_prefix("repeat(")
            .and_then(|rest| rest.strip_suffix(')'))
        {
            if !allow_repeat {
                return None;
            }
            let (count, body) = inner.split_once(',')?;
            let count = count.trim();
            let mut tracks = TrackList {
                tracks: Vec::new(),
                repeat: None,
            };
            parse_terms(body, &mut tracks, false)?;
            if tracks.tracks.is_empty() {
                return None;
            }
            if count == "auto-fill" || count == "auto-fit" {
                if list.repeat.is_some() {
                    return None;
                }
                list.repeat = Some(AutoRepeat {
                    at: list.tracks.len(),
                    tracks: tracks.tracks,
                    fit: count == "auto-fit",
                });
            } else {
                let count: usize = count.parse().ok().filter(|count| *count >= 1)?;
                if list.tracks.len() + count * tracks.tracks.len() > MAX_TRACKS {
                    return None;
                }
                for _ in 0..count {
                    list.tracks.extend(tracks.tracks.iter().copied());
                }
            }
            continue;
        }
        list.tracks.push(parse_track(term)?);
        if list.tracks.len() > MAX_TRACKS {
            return None;
        }
    }
    Some(())
}

pub fn parse_tracks(text: &str) -> Option<TrackList> {
    let cleaned = strip_line_names(&text.to_ascii_lowercase());
    let mut list = TrackList {
        tracks: Vec::new(),
        repeat: None,
    };
    parse_terms(&cleaned, &mut list, true)?;
    if list.tracks.is_empty() && list.repeat.is_none() {
        return None;
    }
    Some(list)
}

pub fn parse_line(text: &str) -> Option<Line> {
    let lower = text.trim().to_ascii_lowercase();
    if lower == "auto" {
        return Some(Line::Auto);
    }
    if let Some(rest) = lower.strip_prefix("span") {
        let rest = rest.trim();
        if rest.is_empty() {
            return Some(Line::Span(1));
        }
        let count: u32 = rest.parse().ok().filter(|count| *count >= 1)?;
        return Some(Line::Span(count.min(MAX_TRACKS as u32)));
    }
    if let Ok(number) = lower.parse::<i32>() {
        return if number == 0 {
            None
        } else {
            Some(Line::Number(number))
        };
    }
    if !lower.is_empty()
        && lower
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Some(Line::Area(lower));
    }
    None
}

pub fn parse_areas(text: &str) -> Option<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(['"', '\'']) {
        let quote = rest[start..].chars().next()?;
        let after = &rest[start + 1..];
        let end = after.find(quote)?;
        let names: Vec<String> = after[..end]
            .split_whitespace()
            .map(|name| name.to_ascii_lowercase())
            .collect();
        if names.is_empty() {
            return None;
        }
        rows.push(names);
        rest = &after[end + 1..];
    }
    let width = rows.first()?.len();
    if rows.iter().any(|row| row.len() != width) {
        return None;
    }
    Some(rows)
}
