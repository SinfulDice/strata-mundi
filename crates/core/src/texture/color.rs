use super::Rgba;
use serde::Deserialize;
use std::collections::HashMap;

/// Colors from dark to light for one material, e.g. `"stone": ["#5c5c63", …]`.
pub type Ramp = Vec<Rgba>;

/// Every ramp of the game, by name, merged from all packs (`palette.ron`).
/// Restyling the whole game = replacing this file in a resource pack.
#[derive(Debug, Clone, Default)]
pub struct Palette {
    ramps: HashMap<String, Ramp>,
}

#[derive(Debug, Deserialize)]
#[serde(transparent)]
pub struct PaletteFile(HashMap<String, Vec<HexColor>>);

/// `"#rrggbb"` or `"#rrggbbaa"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HexColor(pub Rgba);

impl<'de> Deserialize<'de> for HexColor {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        parse_hex(&text).map(HexColor).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "invalid color `{text}`, expected #rrggbb or #rrggbbaa"
            ))
        })
    }
}

pub fn parse_hex(text: &str) -> Option<Rgba> {
    let digits = text.strip_prefix('#')?;
    if !matches!(digits.len(), 6 | 8) || !digits.is_ascii() {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(&digits[i..i + 2], 16).ok();
    let alpha = if digits.len() == 8 { byte(6)? } else { 255 };
    Some(Rgba([byte(0)?, byte(2)?, byte(4)?, alpha]))
}

impl Palette {
    /// Ramps of `file` replace ramps of the same name.
    pub fn merge(&mut self, file: PaletteFile) {
        for (name, colors) in file.0 {
            self.ramps
                .insert(name, colors.into_iter().map(|c| c.0).collect());
        }
    }

    pub fn ramp(&self, name: &str) -> Option<&Ramp> {
        self.ramps.get(name).filter(|ramp| !ramp.is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_colors() {
        assert_eq!(parse_hex("#ff8000"), Some(Rgba([255, 128, 0, 255])));
        assert_eq!(parse_hex("#ff800080"), Some(Rgba([255, 128, 0, 128])));
        assert_eq!(parse_hex("ff8000"), None);
        assert_eq!(parse_hex("#ff80"), None);
        assert_eq!(parse_hex("#gg8000"), None);
    }

    #[test]
    fn later_palette_overrides_ramps() {
        let mut palette = Palette::default();
        palette.merge(ron::from_str(r##"{"stone": ["#000000"], "dirt": ["#111111"]}"##).unwrap());
        palette.merge(ron::from_str(r##"{"stone": ["#ffffff"]}"##).unwrap());
        assert_eq!(
            palette.ramp("stone"),
            Some(&vec![Rgba([255, 255, 255, 255])])
        );
        assert_eq!(palette.ramp("dirt"), Some(&vec![Rgba([17, 17, 17, 255])]));
    }
}
