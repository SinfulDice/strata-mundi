//! TrueType fonts (`fonts/<name>.ttf` described by `fonts/<name>.ron`), rasterized into
//! the same kind of atlas as pixel fonts, at the screen resolution: the readable
//! accessibility font.

use super::{Font, Glyph, MAX_FONT_ATLAS};
use crate::texture::{Rgba, RgbaImage};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::sync::Arc;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "TrueType")]
pub struct TrueTypeDef {
    /// Height of the em square, in interface pixels.
    pub size: f32,
    pub line_height: u32,
}

/// Characters rasterized: everything the interface writes in French, English and LOLcat.
const CHARACTERS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`\
abcdefghijklmnopqrstuvwxyz{|}~«»…’°×·àâäèéêëìíîïòóôöùúûüýÿÀÂÄÈÉÊËÌÍÎÏÒÓÔÖÙÚÛÜŸçÇœŒæÆ";
/// Atlas pixels left between glyphs, so filtering never bleeds into a neighbour.
const GAP: u32 = 1;

#[derive(Clone)]
pub struct TrueTypeSource {
    font: Arc<fontdue::Font>,
    def: TrueTypeDef,
    /// Drawn at density 1 when loaded: always available.
    base: Font,
}

impl std::fmt::Debug for TrueTypeSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TrueTypeSource")
            .field("def", &self.def)
            .finish_non_exhaustive()
    }
}

impl TrueTypeSource {
    /// Fails on an unreadable file or a size the atlas cannot hold even at density 1.
    pub fn new(bytes: Vec<u8>, def: TrueTypeDef) -> Result<Self, String> {
        if !(4.0..=64.0).contains(&def.size) || def.line_height == 0 {
            return Err("size must be between 4 and 64 and line_height above 0".to_owned());
        }
        let font = fontdue::Font::from_bytes(bytes, fontdue::FontSettings::default())
            .map_err(|e| format!("unreadable TrueType file: {e}"))?;
        let font = Arc::new(font);
        let base = build(&font, &def, 1)?;
        Ok(Self { font, def, base })
    }

    /// At `density`, or the largest density below it whose atlas fits.
    pub fn font(&self, density: u32) -> Font {
        (2..=density)
            .rev()
            .find_map(|density| build(&self.font, &self.def, density).ok())
            .unwrap_or_else(|| self.base.clone())
    }
}

/// Glyphs packed in rows (shelves) as wide as the largest font atlas allowed.
fn build(font: &fontdue::Font, def: &TrueTypeDef, density: u32) -> Result<Font, String> {
    let size = def.size * density as f32;
    let cell_h = def.line_height * density;
    let baseline = font
        .horizontal_line_metrics(size)
        .map_or(size, |line| line.ascent)
        .round() as i32;
    let mut atlas = RgbaImage::new(MAX_FONT_ATLAS, MAX_FONT_ATLAS, Rgba([0; 4]));
    let fallback = draw_fallback(&mut atlas, density, cell_h);
    let mut pen = (fallback.width + GAP, 0);
    let mut glyphs = BTreeMap::new();
    for ch in CHARACTERS.chars() {
        let (metrics, coverage) = font.rasterize(ch, size);
        let advance = (metrics.advance_width.round() as u32).max(1);
        let cell_w = advance.max(metrics.xmin.max(0) as u32 + metrics.width as u32);
        if pen.0 + cell_w > MAX_FONT_ATLAS {
            pen = (0, pen.1 + cell_h + GAP);
        }
        if pen.1 + cell_h > MAX_FONT_ATLAS {
            return Err(format!("font too large for a {MAX_FONT_ATLAS} pixel atlas"));
        }
        let top = baseline - (metrics.ymin + metrics.height as i32);
        let left = metrics.xmin.max(0);
        for (i, &alpha) in coverage.iter().enumerate() {
            let (gx, gy) = ((i % metrics.width) as i32, (i / metrics.width) as i32);
            let inside = (0..cell_h as i32).contains(&(top + gy)) && alpha > 0;
            if inside {
                let (px, py) = (pen.0 as i32 + left + gx, pen.1 as i32 + top + gy);
                atlas.set(px as u32, py as u32, Rgba([255, 255, 255, alpha]));
            }
        }
        glyphs.insert(
            ch,
            Glyph {
                x: pen.0,
                y: pen.1,
                width: cell_w,
                advance,
            },
        );
        pen.0 += cell_w + GAP;
    }
    let used = pen.1 + cell_h;
    Ok(Font {
        atlas: crop_height(&atlas, used),
        line_height: def.line_height,
        density,
        glyphs,
        fallback,
    })
}

/// An outlined box at the atlas origin, for characters the font lacks.
fn draw_fallback(atlas: &mut RgbaImage, density: u32, cell_h: u32) -> Glyph {
    let (w, h) = (3 * density, (7 * density).min(cell_h));
    let top = cell_h - h - density;
    for y in 0..h {
        for x in 0..w {
            let edge = x < density || x >= w - density || y < density || y >= h - density;
            if edge {
                atlas.set(x, top + y, Rgba([255; 4]));
            }
        }
    }
    Glyph {
        x: 0,
        y: 0,
        width: w,
        advance: w + density,
    }
}

fn crop_height(image: &RgbaImage, height: u32) -> RgbaImage {
    let mut cropped = RgbaImage::new(image.width(), height, Rgba([0; 4]));
    for y in 0..height {
        for x in 0..image.width() {
            cropped.set(x, y, image.get(x, y));
        }
    }
    cropped
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn readable() -> TrueTypeSource {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/packs/strata/fonts/readable.ttf");
        let def = TrueTypeDef {
            size: 10.0,
            line_height: 12,
        };
        TrueTypeSource::new(std::fs::read(path).unwrap(), def).unwrap()
    }

    #[test]
    fn the_readable_font_covers_french() {
        let font = readable().font(1);
        assert!(font.width("Réglages : Échap") > 40);
        assert_ne!(font.glyph('é'), font.glyph('☃'), "accents are real glyphs");
        let broken = TrueTypeDef {
            size: 10.0,
            line_height: 12,
        };
        assert!(TrueTypeSource::new(b"not a font".to_vec(), broken).is_err());
    }

    #[test]
    fn denser_fonts_keep_the_same_interface_size() {
        let source = readable();
        let (one, three) = (source.font(1), source.font(3));
        assert_eq!(three.density, 3);
        assert_eq!(one.line_height, three.line_height);
        let (w1, w3) = (one.width("Options"), three.width("Options"));
        assert!(w1.abs_diff(w3) <= 2, "{w1} vs {w3}");
    }

    #[test]
    fn too_dense_falls_back_to_what_fits() {
        assert!(readable().font(40).density < 40);
    }
}
