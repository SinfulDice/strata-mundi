use serde::Deserialize;
use strata_core::texture::HexColor;

/// Interface colors from `assets/ui/theme.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Theme")]
pub struct Theme {
    pub text: HexColor,
    pub text_shadow: HexColor,
    pub text_dim: HexColor,
    pub accent: HexColor,
    pub panel: HexColor,
    pub panel_light: HexColor,
    pub panel_dark: HexColor,
    pub slot: HexColor,
    pub slot_border: HexColor,
    pub focus: HexColor,
    pub field: HexColor,
    /// Filled part of an options gauge.
    pub gauge: HexColor,
    pub shade: HexColor,
    /// Fill of the HUD bars, by gauge id (health, stamina…); the accent otherwise.
    #[serde(default)]
    pub bars: std::collections::BTreeMap<String, HexColor>,
    /// The flash over the screen when hurt.
    #[serde(default = "hurt_red")]
    pub hurt: HexColor,
    /// The edges of the screen in the cold and in the heat.
    #[serde(default = "frost_white")]
    pub frost: HexColor,
    #[serde(default = "heat_orange")]
    pub heat: HexColor,
}

fn frost_white() -> HexColor {
    HexColor(strata_core::texture::Rgba([220, 239, 255, 144]))
}

fn heat_orange() -> HexColor {
    HexColor(strata_core::texture::Rgba([255, 140, 48, 112]))
}

/// A translucent red, for themes written before the survival gauges.
fn hurt_red() -> HexColor {
    HexColor(strata_core::texture::Rgba([192, 32, 24, 112]))
}
