//! Fonts described in packs, drawn into an atlas image: pixel fonts written in RON
//! (`fonts/<name>.ron`), or TrueType files (`fonts/<name>.ttf` beside a `TrueType(…)`
//! description, see `truetype`).

mod truetype;

pub use truetype::TrueTypeSource;

use crate::pack::{PackError, PackSet, read_ron};
use crate::texture::{Rgba, RgbaImage};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Deserialize)]
#[serde(rename = "Font")]
struct FontFile {
    line_height: u32,
    /// Pixels from the top of the line to the cap line (room for accents above capitals).
    ascent: u32,
    glyphs: BTreeMap<char, GlyphDef>,
    accents: BTreeMap<String, Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct GlyphDef {
    accent: Option<String>,
    rows: Vec<String>,
}

/// Where a character sits in the atlas and how far the pen moves after it, in atlas
/// pixels (`Font::density` of them per interface pixel).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Glyph {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub advance: u32,
}

#[derive(Debug, Clone)]
pub struct Font {
    pub atlas: RgbaImage,
    /// In interface pixels.
    pub line_height: u32,
    /// Atlas pixels per interface pixel: 1 for pixel fonts; TrueType fonts are rasterized
    /// at the screen resolution so they stay smooth at any interface scale.
    pub density: u32,
    glyphs: BTreeMap<char, Glyph>,
    fallback: Glyph,
}

/// A font as found in the packs, before it is drawn for a given density.
#[derive(Debug, Clone)]
pub enum FontSource {
    Pixel(Font),
    TrueType(TrueTypeSource),
}

const ATLAS_COLUMNS: u32 = 16;
/// Largest font image accepted, so it always fits beside the icons in the interface atlas.
pub const MAX_FONT_ATLAS: u32 = 512;
const SPACING: u32 = 1;
const INK: Rgba = Rgba([255, 255, 255, 255]);
/// The small box drawn at the start of every atlas, for unknown characters.
const FALLBACK: Glyph = Glyph {
    x: 0,
    y: 0,
    width: 3,
    advance: 4,
};

impl FontSource {
    /// The last pack defining `fonts/<name>.ron` wins (resource packs included).
    pub fn load(packs: &PackSet, name: &str, errors: &mut Vec<PackError>) -> Option<Self> {
        let path = packs
            .packs()
            .iter()
            .rev()
            .map(|pack| pack.root.join("fonts").join(format!("{name}.ron")))
            .find(|path| path.is_file())?;
        let ttf = path.with_extension("ttf");
        let built = if ttf.is_file() {
            read_ron::<truetype::TrueTypeDef>(&path).and_then(|def| {
                let bytes =
                    std::fs::read(&ttf).map_err(|e| PackError::invalid(&ttf, e.to_string()))?;
                TrueTypeSource::new(bytes, def)
                    .map(Self::TrueType)
                    .map_err(|m| PackError::invalid(&ttf, m))
            })
        } else {
            read_ron::<FontFile>(&path).and_then(|file| {
                Font::build(&file)
                    .map(Self::Pixel)
                    .map_err(|m| PackError::invalid(&path, m))
            })
        };
        built.map_err(|error| errors.push(error)).ok()
    }

    /// Drawn again for each interface scale (pixel fonts are drawn once).
    pub fn is_scalable(&self) -> bool {
        matches!(self, Self::TrueType(_))
    }

    /// The font drawn for `density` atlas pixels per interface pixel, or the largest
    /// density below it whose atlas fits.
    pub fn font(&self, density: u32) -> Font {
        match self {
            Self::Pixel(font) => font.clone(),
            Self::TrueType(source) => source.font(density),
        }
    }
}

impl Font {
    /// Fails on glyphs taller than a line or a font too big for the interface atlas.
    fn build(file: &FontFile) -> Result<Self, String> {
        let cap = file.ascent.saturating_sub(7);
        if let Some((ch, _)) = file
            .glyphs
            .iter()
            .find(|(_, def)| cap + def.rows.len() as u32 > file.line_height)
        {
            return Err(format!("glyph `{ch}` is taller than line_height"));
        }
        let cell_w = file
            .glyphs
            .values()
            .map(|def| glyph_width(def).max(accent_width(file, def)))
            .max()
            .unwrap_or(1)
            .max(3)
            + 1;
        let cell_h = file.line_height.max(1);
        let rows = (file.glyphs.len() as u32 + 1).div_ceil(ATLAS_COLUMNS);
        if ATLAS_COLUMNS * cell_w > MAX_FONT_ATLAS || rows * cell_h > MAX_FONT_ATLAS {
            return Err(format!(
                "font too large: its glyphs need more than {MAX_FONT_ATLAS}×{MAX_FONT_ATLAS} pixels"
            ));
        }
        let mut atlas = RgbaImage::new(ATLAS_COLUMNS * cell_w, rows * cell_h, Rgba([0; 4]));
        let mut glyphs = BTreeMap::new();
        for (index, (ch, def)) in file.glyphs.iter().enumerate() {
            let (x, y) = cell(index as u32 + 1, cell_w, cell_h);
            let width = glyph_width(def).max(accent_width(file, def));
            draw_glyph(&mut atlas, file, def, x, y, width);
            glyphs.insert(
                *ch,
                Glyph {
                    x,
                    y,
                    width,
                    advance: width + SPACING,
                },
            );
        }
        draw_box(&mut atlas, file.ascent, file.line_height);
        Ok(Self {
            atlas,
            line_height: cell_h,
            density: 1,
            glyphs,
            fallback: FALLBACK,
        })
    }

    /// Unknown characters show as a small box.
    pub fn glyph(&self, ch: char) -> Glyph {
        self.glyphs.get(&ch).copied().unwrap_or(self.fallback)
    }

    /// In interface pixels, rounded up.
    pub fn width(&self, text: &str) -> u32 {
        text.chars()
            .map(|c| self.glyph(c).advance)
            .sum::<u32>()
            .saturating_sub(SPACING)
            .div_ceil(self.density)
    }
}

fn glyph_width(def: &GlyphDef) -> u32 {
    def.rows
        .iter()
        .map(|r| r.chars().count() as u32)
        .max()
        .unwrap_or(0)
}

fn accent_width(file: &FontFile, def: &GlyphDef) -> u32 {
    def.accent
        .as_ref()
        .and_then(|a| file.accents.get(a))
        .map_or(0, |rows| {
            rows.iter()
                .map(|r| r.chars().count() as u32)
                .max()
                .unwrap_or(0)
        })
}

fn cell(index: u32, w: u32, h: u32) -> (u32, u32) {
    ((index % ATLAS_COLUMNS) * w, (index / ATLAS_COLUMNS) * h)
}

/// Body rows start at the cap line (`ascent - 7`); an accent goes just above the first
/// inked row, so it lands above capitals and inside the ascender space of lowercase.
fn draw_glyph(atlas: &mut RgbaImage, file: &FontFile, def: &GlyphDef, x: u32, y: u32, width: u32) {
    let cap = file.ascent.saturating_sub(7);
    let body_width = glyph_width(def);
    let offset = (width - body_width) / 2;
    plot(atlas, &def.rows, x + offset, y + cap);
    let accent = def.accent.as_ref().and_then(|a| file.accents.get(a));
    if let Some(rows) = accent {
        let first_ink = def.rows.iter().position(|r| r.contains('#')).unwrap_or(0) as u32;
        let top = (y + cap + first_ink).saturating_sub(rows.len() as u32 + 1);
        let accent_width = rows
            .iter()
            .map(|r| r.chars().count() as u32)
            .max()
            .unwrap_or(0);
        plot(atlas, rows, x + (width - accent_width) / 2, top.max(y));
    }
}

fn plot(atlas: &mut RgbaImage, rows: &[String], x: u32, y: u32) {
    for (dy, row) in rows.iter().enumerate() {
        for (dx, pixel) in row.chars().enumerate() {
            let (px, py) = (x + dx as u32, y + dy as u32);
            if pixel == '#' && px < atlas.width() && py < atlas.height() {
                atlas.set(px, py, INK);
            }
        }
    }
}

fn draw_box(atlas: &mut RgbaImage, ascent: u32, line_height: u32) {
    let top = ascent.saturating_sub(7);
    for y in top..(top + 7).min(line_height) {
        for x in 0..3 {
            if x == 0 || x == 2 || y == top || y == top + 6 {
                atlas.set(x, y, INK);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> Font {
        let text = r######"Font(line_height: 11, ascent: 9, glyphs: {
            'a': (accent: None, rows: [".....", ".....", ".###.", "....#", ".####", "#...#", ".####"]),
            'é': (accent: Some("acute"), rows: [".....", ".....", ".###.", "#...#", "#####", "#....", ".####"]),
            'I': (accent: None, rows: ["###", ".#.", ".#.", ".#.", ".#.", ".#.", "###"]),
        }, accents: {"acute": [".#.", "#.."]})"######;
        Font::build(&ron::from_str(text).unwrap()).unwrap()
    }

    #[test]
    fn oversized_glyphs_are_refused() {
        let text = r####"Font(line_height: 4, ascent: 9, glyphs: {
            'a': (accent: None, rows: ["#", "#", "#", "#", "#"]),
        }, accents: {})"####;
        assert!(Font::build(&ron::from_str(text).unwrap()).is_err());
    }

    #[test]
    fn widths_follow_the_glyphs() {
        let font = font();
        assert_eq!(font.glyph('a').width, 5);
        assert_eq!(font.width("aI"), 5 + 1 + 3);
        assert_eq!(font.glyph('?'), font.fallback);
    }

    #[test]
    fn accents_are_drawn_above_the_letter() {
        let font = font();
        let glyph = font.glyph('é');
        let inked =
            |y: u32| (0..glyph.width).any(|x| font.atlas.get(glyph.x + x, glyph.y + y).0[3] > 0);
        assert!(inked(2) || inked(3), "accent rows");
        assert!(inked(4) && inked(8), "letter body");
    }

    #[test]
    fn the_real_font_covers_french() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/packs/strata/fonts/pixel.ron");
        let file: FontFile = read_ron(&path).unwrap();
        for ch in "AÀÂÇÉÈÊËÎÏÔÙÛÜŸŒÆaàâçéèêëîïôùûüÿœæ0123456789 .,;:!?'’«»…-()".chars()
        {
            assert!(file.glyphs.contains_key(&ch), "missing `{ch}`");
        }
    }
}
