// SPDX-License-Identifier: GPL-3.0-only

use std::collections::HashSet;
use std::rc::Rc;

use fontdue::Font;

use crate::css::{self, Rule, Stylesheet, Value};
use crate::dom::{Node, NodeType};
use crate::layout::{self, ImageMap};
use crate::loader;
use crate::location::Location;
use crate::paint::{self, Canvas};
use crate::picture;
use crate::style::{self, StyleSource};
use crate::{html, text};

const MAX_IMPORT_DEPTH: usize = 8;
const MAX_STYLESHEETS: usize = 64;
const MAX_IMAGES: usize = 256;

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

    pub fn with_location(source: &str, location: &Location) -> Page {
        let document = html::parse(source);
        let mut warnings = Vec::new();
        let base = document_base(&document, location, &mut warnings);

        let mut sheets = Sheets {
            visited: HashSet::new(),
            loaded: 0,
            rules: Vec::new(),
            warnings: Vec::new(),
        };
        for style_source in style::style_sources(&document) {
            match style_source {
                StyleSource::Inline(text) => add_stylesheet(&text, &base, 0, &mut sheets),
                StyleSource::Link(href) => add_linked_stylesheet(&href, &base, &mut sheets),
            }
        }
        warnings.append(&mut sheets.warnings);

        let mut images = ImageMap::new();
        let mut sources = Vec::new();
        collect_image_sources(&document, &mut sources);
        for rule in &sheets.rules {
            for declaration in &rule.declarations {
                if let Value::Url(url) = &declaration.value {
                    sources.push(url.clone());
                }
            }
        }
        for src in sources {
            if images.contains_key(&src) {
                continue;
            }
            if images.len() >= MAX_IMAGES {
                warnings.push(format!(
                    "image {} skipped: more than {} images",
                    src, MAX_IMAGES
                ));
                continue;
            }
            match load_picture(&base, &src) {
                Ok(loaded) => {
                    images.insert(src, Rc::new(loaded));
                }
                Err(message) => warnings.push(message),
            }
        }

        Page {
            stylesheet: style::with_author_rules(sheets.rules),
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
                    layout::layout_tree(&styled_root, viewport, &self.font, &self.images);
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

struct Sheets {
    visited: HashSet<String>,
    loaded: usize,
    rules: Vec<Rule>,
    warnings: Vec<String>,
}

fn find_base_href(node: &Node) -> Option<String> {
    if let NodeType::Element(element) = &node.node_type {
        if element.tag_name == "base" {
            if let Some(href) = element.attributes.get("href") {
                return Some(href.clone());
            }
        }
    }
    for child in &node.children {
        if let Some(href) = find_base_href(child) {
            return Some(href);
        }
    }
    None
}

fn document_base(document: &Node, location: &Location, warnings: &mut Vec<String>) -> Location {
    let Some(href) = find_base_href(document) else {
        return location.clone();
    };
    if href.trim().is_empty() {
        return location.clone();
    }
    match location.resolve(&href) {
        Ok(base) => base,
        Err(error) => {
            warnings.push(format!("base {}: {}", href, error));
            location.clone()
        }
    }
}

fn add_stylesheet(text: &str, base: &Location, depth: usize, sheets: &mut Sheets) {
    for import in css::parse_imports(text) {
        if !style::media_applies(&import.media) {
            continue;
        }
        if depth >= MAX_IMPORT_DEPTH {
            sheets.warnings.push(format!(
                "@import {} is nested deeper than {} levels",
                import.url, MAX_IMPORT_DEPTH
            ));
            continue;
        }
        let location = match base.resolve(&import.url) {
            Ok(location) => location,
            Err(error) => {
                sheets
                    .warnings
                    .push(format!("@import {}: {}", import.url, error));
                continue;
            }
        };
        if !sheets.visited.insert(location.to_string()) {
            continue;
        }
        if sheets.loaded >= MAX_STYLESHEETS {
            sheets.warnings.push(format!(
                "@import {} skipped: more than {} stylesheets",
                import.url, MAX_STYLESHEETS
            ));
            continue;
        }
        sheets.loaded += 1;
        match loader::fetch(&location, loader::MAX_DOCUMENT_SIZE) {
            Ok(bytes) => {
                let imported = String::from_utf8_lossy(&bytes).into_owned();
                add_stylesheet(&imported, &location, depth + 1, sheets);
            }
            Err(message) => sheets.warnings.push(message),
        }
    }
    let mut parsed = css::parse(text);
    for rule in &mut parsed.rules {
        for declaration in &mut rule.declarations {
            if let Value::Url(url) = &mut declaration.value {
                if let Ok(location) = base.resolve(url) {
                    *url = location.to_string();
                }
            }
        }
    }
    sheets.rules.extend(parsed.rules);
}

fn add_linked_stylesheet(href: &str, base: &Location, sheets: &mut Sheets) {
    let location = match base.resolve(href) {
        Ok(location) => location,
        Err(error) => {
            sheets
                .warnings
                .push(format!("stylesheet {}: {}", href, error));
            return;
        }
    };
    if sheets.loaded >= MAX_STYLESHEETS {
        sheets.warnings.push(format!(
            "stylesheet {} skipped: more than {} stylesheets",
            href, MAX_STYLESHEETS
        ));
        return;
    }
    sheets.visited.insert(location.to_string());
    sheets.loaded += 1;
    match loader::fetch(&location, loader::MAX_DOCUMENT_SIZE) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).into_owned();
            add_stylesheet(&text, &location, 0, sheets);
        }
        Err(message) => sheets.warnings.push(message),
    }
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
        if let Some(inline) = element.attributes.get("style") {
            for declaration in css::parse_declarations(inline) {
                if let Value::Url(url) = declaration.value {
                    sources.push(url);
                }
            }
        }
    }
    for child in &node.children {
        collect_image_sources(child, sources);
    }
}
