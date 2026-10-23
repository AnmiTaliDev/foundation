// SPDX-License-Identifier: GPL-3.0-only

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Strictness {
    Mandatory,
    Proximity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapAlign {
    None,
    Start,
    Center,
    End,
}

impl SnapAlign {
    pub fn parse(keyword: &str) -> SnapAlign {
        match keyword {
            "start" => SnapAlign::Start,
            "center" => SnapAlign::Center,
            "end" => SnapAlign::End,
            _ => SnapAlign::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SnapPoint {
    pub position: f32,
    pub always: bool,
}

const PROXIMITY: f32 = 0.25;

fn nearest(points: &[SnapPoint], target: f32) -> Option<f32> {
    points
        .iter()
        .map(|point| point.position)
        .min_by(|a, b| (a - target).abs().total_cmp(&(b - target).abs()))
}

fn accept(strictness: Strictness, chosen: f32, target: f32, port: f32) -> f32 {
    match strictness {
        Strictness::Mandatory => chosen,
        Strictness::Proximity if (chosen - target).abs() <= port * PROXIMITY => chosen,
        Strictness::Proximity => target,
    }
}

pub fn snap_nearest(points: &[SnapPoint], strictness: Strictness, target: f32, port: f32) -> f32 {
    match nearest(points, target) {
        Some(chosen) => accept(strictness, chosen, target, port),
        None => target,
    }
}

pub fn snap_directed(
    points: &[SnapPoint],
    strictness: Strictness,
    current: f32,
    target: f32,
    port: f32,
) -> f32 {
    if points.is_empty() || target == current {
        return target;
    }
    let forward = target > current;
    let ahead: Vec<SnapPoint> = points
        .iter()
        .copied()
        .filter(|point| {
            if forward {
                point.position > current
            } else {
                point.position < current
            }
        })
        .collect();
    let stop = if forward {
        ahead
            .iter()
            .find(|point| point.always && point.position <= target)
    } else {
        ahead
            .iter()
            .rev()
            .find(|point| point.always && point.position >= target)
    };
    if let Some(stop) = stop {
        return stop.position;
    }
    match nearest(&ahead, target).or_else(|| nearest(points, target)) {
        Some(chosen) => accept(strictness, chosen, target, port),
        None => target,
    }
}

pub fn advance(
    offset: f32,
    delta: f32,
    max: f32,
    snap: Option<Strictness>,
    points: &[SnapPoint],
    port: f32,
) -> (f32, f32) {
    let target = (offset + delta).clamp(0.0, max);
    let Some(strictness) = snap else {
        return (target, delta - (target - offset));
    };
    let next = snap_directed(points, strictness, offset, target, port).clamp(0.0, max);
    (next, if next == offset { delta } else { 0.0 })
}
