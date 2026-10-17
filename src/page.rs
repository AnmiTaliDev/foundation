// SPDX-License-Identifier: GPL-3.0-only

use std::rc::Rc;

use fontdue::Font;

use crate::css::Stylesheet;
use crate::dom::{Node, NodeType};
use crate::layout::{self, ImageMap};
use crate::loader;
use crate::location::Location;
use crate::paint::{self, Canvas};
use crate::picture;
use crate::style::{self, StyleSource};
use crate::{html, text};

pub struct Page {
    document: Node,
    stylesheet: Stylesheet,
    font: Font,
    images: ImageMap,
    pub warnings: Vec<String>,
}

pub struct Frame {
    pub canvas: Canvas,
    pub content_height: f32,
    pub scroll_y: f32,
}

impl Page {
    pub fn new(source: &str) -> Page {
        let document = html::parse(source);
        let stylesheet = style::stylesheet_for_document(&document);
        Page {
            document,
            stylesheet,
            font: text::default_font(),
            images: ImageMap::new(),
            warnings: Vec::new(),
        }
    }

    pub fn open(address: &str) -> Result<Page, String> {
        let location = Location::parse(address);
        let bytes = loader::fetch(&location, loader::MAX_DOCUMENT_SIZE)?;
        let source = String::from_utf8_lossy(&bytes);
        Ok(Page::with_location(&source, &location))
    }

    pub fn with_location(source: &str, base: &Location) -> Page {
        let document = html::parse(source);
        let mut warnings = Vec::new();

        let mut author_css = String::new();
        for style_source in style::style_sources(&document) {
            match style_source {
                StyleSource::Inline(text) => author_css.push_str(&text),
                StyleSource::Link(href) => match load_stylesheet(base, &href) {
                    Ok(text) => author_css.push_str(&text),
                    Err(message) => warnings.push(message),
                },
            }
        }

        let mut images = ImageMap::new();
        let mut sources = Vec::new();
        collect_image_sources(&document, &mut sources);
        for src in sources {
            if images.contains_key(&src) {
                continue;
            }
            match load_picture(base, &src) {
                Ok(loaded) => {
                    images.insert(src, Rc::new(loaded));
                }
                Err(message) => warnings.push(message),
            }
        }

        Page {
            stylesheet: style::build_stylesheet(&author_css),
            document,
            font: text::default_font(),
            images,
            warnings,
        }
    }

    pub fn render(&self, width: usize, height: usize, scroll_y: f32) -> Frame {
        let viewport = style::Viewport::new(width as f32, height as f32);
        match style::style_tree(&self.document, &self.stylesheet, viewport) {
            Some(styled_root) => {
                let layout_root =
                    layout::layout_tree(&styled_root, width as f32, &self.font, &self.images);
                let content_height = layout_root.dimensions.margin_box().height;
                let max_scroll = (content_height - height as f32).max(0.0);
                let scroll_y = scroll_y.clamp(0.0, max_scroll);
                Frame {
                    canvas: paint::paint(&layout_root, width, height, scroll_y, &self.font),
                    content_height,
                    scroll_y,
                }
            }
            None => Frame {
                canvas: Canvas::new(width, height),
                content_height: 0.0,
                scroll_y: 0.0,
            },
        }
    }
}

fn load_stylesheet(base: &Location, href: &str) -> Result<String, String> {
    let location = base
        .resolve(href)
        .map_err(|error| format!("stylesheet {}: {}", href, error))?;
    let bytes = loader::fetch(&location, loader::MAX_DOCUMENT_SIZE)?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn load_picture(base: &Location, src: &str) -> Result<picture::Picture, String> {
    let location = base
        .resolve(src)
        .map_err(|error| format!("image {}: {}", src, error))?;
    let bytes = loader::fetch(&location, loader::MAX_IMAGE_SIZE)?;
    picture::decode(&bytes).map_err(|error| format!("cannot decode {}: {}", location, error))
}

fn collect_image_sources(node: &Node, sources: &mut Vec<String>) {
    if let NodeType::Element(element) = &node.node_type {
        if element.tag_name == "img" {
            if let Some(src) = element.attributes.get("src") {
                sources.push(src.clone());
            }
        }
    }
    for child in &node.children {
        collect_image_sources(child, sources);
    }
}
