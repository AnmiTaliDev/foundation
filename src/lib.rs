// SPDX-License-Identifier: GPL-3.0-only

pub mod calc;
mod composite;
pub mod css;
pub mod dom;
pub mod entities;
pub mod flex;
pub mod float;
pub mod html;
pub mod inline;
pub mod layout;
pub mod loader;
pub mod location;
pub mod media;
pub mod page;
pub mod paint;
pub mod picture;
pub mod style;
pub mod supports;
pub mod text;
pub mod transform;

pub use page::{Frame, Page};

pub fn render(source: &str, width: usize, height: usize) -> paint::Canvas {
    Page::new(source).render(width, height, 0.0).canvas
}
