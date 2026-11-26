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
    #[serde(default)]
    pub temperature: TemperatureTheme,
}

/// How strong the frost or heat at the edges is (0 to 1, times the colour's opacity) in a
/// straining and in a harmful zone, and how long the hint stays.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct TemperatureTheme {
    pub edges_straining: f32,
    pub edges_harmful: f32,
    pub hint_seconds: f32,
}

impl Default for TemperatureTheme {
    fn default() -> Self {
        Self {
            edges_straining: 0.45,
            edges_harmful: 1.0,
            hint_seconds: 8.0,
        }
    }
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
