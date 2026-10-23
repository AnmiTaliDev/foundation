// SPDX-License-Identifier: GPL-3.0-only

pub mod calc;
pub mod collapse;
pub mod color;
mod composite;
pub mod control;
pub mod counters;
pub mod css;
pub mod decoration;
pub mod dom;
pub mod entities;
pub mod flex;
pub mod float;
pub mod generated;
pub mod gradient;
mod grid;
pub mod gridspec;
pub mod html;
pub mod inline;
pub mod layout;
pub mod list;
pub mod loader;
pub mod location;
pub mod logical;
pub mod media;
pub mod page;
pub mod paint;
pub mod picture;
pub mod radius;
pub mod scroll;
pub mod selector;
pub mod shadow;
pub mod snap;
pub mod sticky;
pub mod style;
pub mod supports;
mod table;
pub mod text;
pub mod transform;
mod transform_text;

pub use page::{Frame, Page};

pub fn render(source: &str, width: usize, height: usize) -> paint::Canvas {
    Page::new(source).render(width, height, 0.0).canvas
}
