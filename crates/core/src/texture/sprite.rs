use super::recipe::{BASE_SIZE, ColorRef, Painter};
use super::{Rgba, RgbaImage};
use std::collections::BTreeMap;

/// Draws a text sprite: exactly `BASE_SIZE` rows of `BASE_SIZE` characters.
pub fn draw(
    colors: &BTreeMap<char, ColorRef>,
    rows: &[String],
    painter: &Painter<'_>,
) -> Result<RgbaImage, String> {
    if rows.len() != BASE_SIZE as usize {
        return Err(format!(
            "sprite has {} rows, expected {BASE_SIZE}",
            rows.len()
        ));
    }
    let mut image = RgbaImage::new(BASE_SIZE, BASE_SIZE, Rgba([0; 4]));
    for (y, row) in rows.iter().enumerate() {
        let pixels: Vec<char> = row.chars().collect();
        if pixels.len() != BASE_SIZE as usize {
            return Err(format!(
                "sprite row {} has {} pixels, expected {BASE_SIZE}",
                y + 1,
                pixels.len()
            ));
        }
        for (x, symbol) in pixels.into_iter().enumerate() {
            if symbol == '.' {
                continue;
            }
            let color = colors
                .get(&symbol)
                .ok_or_else(|| format!("sprite uses `{symbol}` which has no color"))?;
            image.set(x as u32, y as u32, painter.color(color)?);
        }
    }
    Ok(image)
}
