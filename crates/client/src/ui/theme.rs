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
    pub shade: HexColor,
}
