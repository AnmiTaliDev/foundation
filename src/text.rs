// SPDX-License-Identifier: GPL-3.0-only

use std::cell::OnceCell;

use fontdue::{Font, FontSettings};

struct Face {
    family: String,
    bold: bool,
    italic: bool,
    data: &'static [u8],
    font: OnceCell<Font>,
}

pub struct Fonts {
    faces: Vec<Face>,
}

const BUILTIN: [(&str, bool, bool, &[u8]); 12] = [
    (
        "dejavu sans",
        false,
        false,
        include_bytes!("../assets/DejaVuSans.ttf"),
    ),
    (
        "dejavu sans",
        true,
        false,
        include_bytes!("../assets/DejaVuSans-Bold.ttf"),
    ),
    (
        "dejavu sans",
        false,
        true,
        include_bytes!("../assets/DejaVuSans-Oblique.ttf"),
    ),
    (
        "dejavu sans",
        true,
        true,
        include_bytes!("../assets/DejaVuSans-BoldOblique.ttf"),
    ),
    (
        "dejavu serif",
        false,
        false,
        include_bytes!("../assets/DejaVuSerif.ttf"),
    ),
    (
        "dejavu serif",
        true,
        false,
        include_bytes!("../assets/DejaVuSerif-Bold.ttf"),
    ),
    (
        "dejavu serif",
        false,
        true,
        include_bytes!("../assets/DejaVuSerif-Italic.ttf"),
    ),
    (
        "dejavu serif",
        true,
        true,
        include_bytes!("../assets/DejaVuSerif-BoldItalic.ttf"),
    ),
    (
        "dejavu sans mono",
        false,
        false,
        include_bytes!("../assets/DejaVuSansMono.ttf"),
    ),
    (
        "dejavu sans mono",
        true,
        false,
        include_bytes!("../assets/DejaVuSansMono-Bold.ttf"),
    ),
    (
        "dejavu sans mono",
        false,
        true,
        include_bytes!("../assets/DejaVuSansMono-Oblique.ttf"),
    ),
    (
        "dejavu sans mono",
        true,
        true,
        include_bytes!("../assets/DejaVuSansMono-BoldOblique.ttf"),
    ),
];

const ALIASES: [(&str, &str); 19] = [
    ("sans-serif", "dejavu sans"),
    ("system-ui", "dejavu sans"),
    ("arial", "dejavu sans"),
    ("helvetica", "dejavu sans"),
    ("helvetica neue", "dejavu sans"),
    ("verdana", "dejavu sans"),
    ("tahoma", "dejavu sans"),
    ("segoe ui", "dejavu sans"),
    ("serif", "dejavu serif"),
    ("times", "dejavu serif"),
    ("times new roman", "dejavu serif"),
    ("georgia", "dejavu serif"),
    ("monospace", "dejavu sans mono"),
    ("ui-monospace", "dejavu sans mono"),
    ("courier", "dejavu sans mono"),
    ("courier new", "dejavu sans mono"),
    ("consolas", "dejavu sans mono"),
    ("menlo", "dejavu sans mono"),
    ("monaco", "dejavu sans mono"),
];

impl Fonts {
    pub fn builtin() -> Fonts {
        let mut faces = Vec::new();
        for (family, bold, italic, data) in BUILTIN {
            faces.push(Face {
                family: family.to_string(),
                bold,
                italic,
                data,
                font: OnceCell::new(),
            });
        }
        Fonts { faces }
    }

    pub fn add(
        &mut self,
        family: &str,
        bold: bool,
        italic: bool,
        data: &[u8],
    ) -> Result<(), String> {
        let font =
            Font::from_bytes(data, FontSettings::default()).map_err(|error| error.to_string())?;
        self.faces.push(Face {
            family: family.trim().to_ascii_lowercase(),
            bold,
            italic,
            data: &[],
            font: OnceCell::from(font),
        });
        Ok(())
    }

    pub fn select(&self, families: &[String], bold: bool, italic: bool) -> usize {
        for family in families {
            let name = family.trim().to_ascii_lowercase();
            let mut candidates: Vec<usize> = Vec::new();
            for (index, face) in self.faces.iter().enumerate() {
                if face.family == name {
                    candidates.push(index);
                }
            }
            if candidates.is_empty() {
                if let Some((_, target)) = ALIASES.iter().find(|(alias, _)| *alias == name) {
                    candidates = (0..self.faces.len())
                        .filter(|index| self.faces[*index].family == *target)
                        .collect();
                }
            }
            if let Some(best) = candidates.into_iter().max_by_key(|index| {
                let face = &self.faces[*index];
                (face.bold == bold, face.italic == italic, usize::MAX - index)
            }) {
                return best;
            }
        }
        self.select(&["dejavu sans".to_string()], bold, italic)
    }

    pub fn font(&self, face: usize) -> &Font {
        let face = &self.faces[face];
        face.font.get_or_init(|| {
            Font::from_bytes(face.data, FontSettings::default())
                .expect("embedded font must be valid")
        })
    }

    pub fn width(&self, face: usize, text: &str, font_size: f32) -> f32 {
        let font = self.font(face);
        text.chars()
            .map(|c| font.metrics(c, font_size).advance_width)
            .sum()
    }

    pub fn line_height(&self, face: usize, font_size: f32) -> f32 {
        match self.font(face).horizontal_line_metrics(font_size) {
            Some(metrics) => metrics.new_line_size,
            None => font_size * 1.2,
        }
    }

    pub fn ascent(&self, face: usize, font_size: f32) -> f32 {
        match self.font(face).horizontal_line_metrics(font_size) {
            Some(metrics) => metrics.ascent,
            None => font_size,
        }
    }

    pub fn descent(&self, face: usize, font_size: f32) -> f32 {
        match self.font(face).horizontal_line_metrics(font_size) {
            Some(metrics) => -metrics.descent,
            None => font_size * 0.2,
        }
    }
}

pub fn default_font() -> Fonts {
    Fonts::builtin()
}

pub fn text_width(fonts: &Fonts, text: &str, font_size: f32) -> f32 {
    fonts.width(0, text, font_size)
}

pub fn line_height(fonts: &Fonts, font_size: f32) -> f32 {
    fonts.line_height(0, font_size)
}

pub fn ascent(fonts: &Fonts, font_size: f32) -> f32 {
    fonts.ascent(0, font_size)
}

pub fn descent(fonts: &Fonts, font_size: f32) -> f32 {
    fonts.descent(0, font_size)
}
