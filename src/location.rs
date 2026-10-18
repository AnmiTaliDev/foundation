// SPDX-License-Identifier: GPL-3.0-only

use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub enum Location {
    Url(String),
    File(PathBuf),
}

impl Location {
    pub fn parse(text: &str) -> Location {
        if is_http_url(text) {
            Location::Url(text.to_string())
        } else {
            Location::File(PathBuf::from(text))
        }
    }

    pub fn resolve(&self, reference: &str) -> Result<Location, String> {
        let reference = reference.trim();
        let reference = match reference.split_once('#') {
            Some((before, _)) => before,
            None => reference,
        };
        if reference.is_empty() {
            return Err("empty reference".to_string());
        }
        if is_http_url(reference) {
            return Ok(Location::Url(reference.to_string()));
        }
        if has_scheme(reference) {
            return Err(format!("unsupported scheme in {}", reference));
        }
        match self {
            Location::Url(base) => Ok(Location::Url(resolve_url(base, reference))),
            Location::File(base) => {
                let path = match reference.split_once('?') {
                    Some((before, _)) => before,
                    None => reference,
                };
                if path.starts_with('/') {
                    Ok(Location::File(PathBuf::from(path)))
                } else {
                    let is_directory = base.to_string_lossy().ends_with('/');
                    let directory = if is_directory {
                        base.as_path()
                    } else {
                        base.parent().unwrap_or(Path::new(""))
                    };
                    Ok(Location::File(directory.join(path)))
                }
            }
        }
    }
}

impl std::fmt::Display for Location {
    fn fmt(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Location::Url(url) => write!(formatter, "{}", url),
            Location::File(path) => write!(formatter, "{}", path.display()),
        }
    }
}

fn is_http_url(text: &str) -> bool {
    let lower = text.get(..8).unwrap_or(text).to_ascii_lowercase();
    lower.starts_with("http://") || lower.starts_with("https://")
}

fn has_scheme(text: &str) -> bool {
    let Some((scheme, _)) = text.split_once(':') else {
        return false;
    };
    let mut chars = scheme.chars();
    match chars.next() {
        Some(first) if first.is_ascii_alphabetic() => {
            chars.all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        }
        _ => false,
    }
}

fn split_origin(url: &str) -> (&str, &str) {
    let after_scheme = url.find("://").map_or(0, |index| index + 3);
    match url[after_scheme..].find(['/', '?']) {
        Some(index) => url.split_at(after_scheme + index),
        None => (url, ""),
    }
}

fn resolve_url(base: &str, reference: &str) -> String {
    let base = match base.split_once('#') {
        Some((before, _)) => before,
        None => base,
    };
    let (origin, rest) = split_origin(base);
    let base_path = match rest.split_once('?') {
        Some((path, _)) => path,
        None => rest,
    };

    if let Some(network_path) = reference.strip_prefix("//") {
        let scheme_end = origin.find("://").unwrap_or(0);
        return format!("{}//{}", &origin[..scheme_end + 1], network_path);
    }
    if reference.starts_with('?') {
        return format!("{}{}{}", origin, base_path, reference);
    }

    let (reference_path, query) = match reference.split_once('?') {
        Some((path, query)) => (path, format!("?{}", query)),
        None => (reference, String::new()),
    };
    let merged = if reference_path.starts_with('/') {
        reference_path.to_string()
    } else {
        let directory_end = base_path.rfind('/').map_or(0, |index| index + 1);
        format!(
            "/{}{}",
            base_path[..directory_end].trim_start_matches('/'),
            reference_path
        )
    };
    format!("{}{}{}", origin, remove_dot_segments(&merged), query)
}

fn remove_dot_segments(path: &str) -> String {
    let mut output: Vec<&str> = Vec::new();
    let segments: Vec<&str> = path.split('/').collect();
    let last = segments.len() - 1;
    for (index, segment) in segments.iter().enumerate().skip(1) {
        match *segment {
            "." => {
                if index == last {
                    output.push("");
                }
            }
            ".." => {
                output.pop();
                if index == last {
                    output.push("");
                }
            }
            other => output.push(other),
        }
    }
    format!("/{}", output.join("/"))
}
