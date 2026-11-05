//! What the image looks like without a rebuild: fog, sky color, liquid tint, scale and
//! filter.

use crate::settings::{ColorFilter, GraphicsSettings, Settings};
use strata_core::coords::SECTION_SIZE;

/// Clear color (sRGB) until the sky arrives in v0.2.
pub(super) const SKY_SRGB: [u8; 3] = [0x78, 0xa7, 0xff];

/// Fog starts at this fraction of the render distance.
pub(super) const FOG_START: f32 = 0.6;

fn srgb_to_linear(c: u8) -> f64 {
    let c = f64::from(c) / 255.0;
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// Render targets are sRGB: the clear color must be given in linear space.
pub(super) fn linear_color([r, g, b]: [u8; 3]) -> wgpu::Color {
    wgpu::Color {
        r: srgb_to_linear(r),
        g: srgb_to_linear(g),
        b: srgb_to_linear(b),
        a: 1.0,
    }
}

pub(super) fn linear_rgba([r, g, b]: [u8; 3]) -> [f32; 4] {
    let [r, g, b] = [r, g, b].map(|c| srgb_to_linear(c) as f32);
    [r, g, b, 1.0]
}

/// The eyes are inside a liquid: a thick fog of its color hides everything farther.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Immersion {
    /// sRGB.
    pub color: [u8; 3],
    /// Blocks until the fog is complete.
    pub distance: f32,
}

/// With the eyes in a liquid, the fog starts this fraction of its distance behind the
/// camera, so even what is close takes the liquid's tint.
pub(super) const IMMERSED_FOG_LEAD: f32 = 0.5;

/// Animation time wraps after an hour, keeping `f32` precise (one tiny jump an hour).
pub(super) const TIME_WRAP: f32 = 3600.0;

/// Image settings that need no rebuild, taken from the player's settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderOptions {
    pub fog: bool,
    /// Where the fog is complete, in blocks (the edge of the loaded world).
    pub fog_end: f32,
    /// 3D resolution, as a fraction of the window.
    pub render_scale: f32,
    pub color_filter: ColorFilter,
    pub liquid_animation: bool,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            fog: false,
            fog_end: f32::MAX,
            render_scale: 1.0,
            color_filter: ColorFilter::None,
            liquid_animation: true,
        }
    }
}

impl RenderOptions {
    pub fn from_settings(settings: &Settings) -> Self {
        Self {
            render_scale: settings.graphics.render_scale as f32 / 100.0,
            color_filter: settings.interface.color_filter,
            ..Self::for_capture(&settings.graphics)
        }
    }

    /// Captures: the fog of the graphics settings, full resolution, no filter.
    pub fn for_capture(graphics: &GraphicsSettings) -> Self {
        Self {
            fog: graphics.fog,
            fog_end: (graphics.render_distance as f32 - 0.5) * SECTION_SIZE as f32,
            render_scale: 1.0,
            color_filter: ColorFilter::None,
            liquid_animation: graphics.liquid_animation,
        }
    }

    /// Whether the image goes through the final pass (scaling, colour filter).
    pub fn needs_post(&self) -> bool {
        (self.render_scale - 1.0).abs() > f32::EPSILON || self.color_filter != ColorFilter::None
    }
}
