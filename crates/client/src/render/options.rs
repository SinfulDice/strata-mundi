//! What the image looks like without a rebuild: fog, sky color, liquid tint, scale and
//! filter.

use crate::settings::{CloudMode, ColorFilter, DEFAULT_BRIGHTNESS, GraphicsSettings, Settings};
use glam::Vec3;
use strata_core::coords::SECTION_SIZE;

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

pub(crate) fn linear_rgb([r, g, b]: [u8; 3]) -> Vec3 {
    Vec3::from_array([r, g, b].map(|c| srgb_to_linear(c) as f32))
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

/// Light level of the darkest places, so that nothing is ever pitch black.
pub(super) const MIN_AMBIENT: f32 = 0.2;

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
    pub animated_textures: bool,
    /// Random texture variants, picked per pixel by the shader.
    pub texture_variants: bool,
    /// 0 (moody) to 1 (bright): how much dark areas are lifted.
    pub brightness: f32,
    pub clouds: CloudMode,
}

impl Default for RenderOptions {
    fn default() -> Self {
        Self {
            fog: false,
            fog_end: f32::MAX,
            render_scale: 1.0,
            color_filter: ColorFilter::None,
            animated_textures: true,
            texture_variants: true,
            brightness: DEFAULT_BRIGHTNESS as f32 / 100.0,
            clouds: CloudMode::default(),
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
            fog_end: fog_end(graphics.render_distance),
            render_scale: 1.0,
            color_filter: ColorFilter::None,
            animated_textures: graphics.animated_textures,
            texture_variants: graphics.texture_variants,
            brightness: graphics.brightness as f32 / 100.0,
            clouds: graphics.clouds,
        }
    }

    /// Whether the image goes through the final pass (scaling, colour filter).
    pub fn needs_post(&self) -> bool {
        (self.render_scale - 1.0).abs() > f32::EPSILON || self.color_filter != ColorFilter::None
    }
}

/// Columns at the edge of the loaded disc that are never drawn: a section is meshed only
/// once the 5 × 5 columns around it are loaded and lit (D046).
const UNMESHED_RING: f32 = 2.0;

/// Where the fog is complete: the nearest the drawn world can end, whatever the camera's
/// place in its column.
fn fog_end(render_distance: u32) -> f32 {
    (render_distance as f32 - UNMESHED_RING).max(1.0) * SECTION_SIZE as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fog_ends_before_the_first_undrawn_column() {
        for distance in [2, 6, 12, 32] {
            let end = fog_end(distance);
            let drawn = (distance as f32 - UNMESHED_RING) * SECTION_SIZE as f32;
            assert!(end <= drawn.max(SECTION_SIZE as f32), "{distance}: {end}");
            assert!(end > 0.0);
        }
        assert_eq!(fog_end(12), 160.0);
    }
}
