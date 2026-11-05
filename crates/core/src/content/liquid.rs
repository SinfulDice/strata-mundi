//! How a liquid feels and looks: movement inside it, the view from inside, its surface.

use super::fluid::{FluidLevel, MAX_SPREAD};
use crate::texture::{HexColor, Rgba};
use serde::Deserialize;

/// Movement of bodies inside the liquid (shared physics, client and server).
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidMotion {
    /// Swimming speed, as a fraction of the walking speed.
    pub speed: f64,
    /// Fraction of the velocity kept each tick (lower is thicker).
    pub drag: f64,
    /// Fraction of the normal gravity: how fast a body sinks.
    pub gravity: f64,
}

impl Default for LiquidMotion {
    fn default() -> Self {
        Self {
            speed: 0.5,
            drag: 0.8,
            gravity: 0.15,
        }
    }
}

impl LiquidMotion {
    pub(super) fn clamped(self) -> Self {
        Self {
            speed: self.speed.clamp(0.0, 1.0),
            drag: self.drag.clamp(0.0, 0.99),
            gravity: self.gravity.clamp(0.0, 1.0),
        }
    }
}

/// How the surface moves (a light shader effect, which a setting turns off).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum SurfaceAnimation {
    Still,
    /// Quick ripples (water).
    #[default]
    Ripple,
    /// Slow heavy waves (lava).
    Ooze,
}

impl SurfaceAnimation {
    /// Value sent to the shader in the vertex.
    pub const fn code(self) -> u8 {
        match self {
            Self::Still => 0,
            Self::Ripple => 1,
            Self::Ooze => 2,
        }
    }
}

/// What the camera sees with its eyes inside the liquid.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LiquidView {
    #[serde(default = "default_fog_color")]
    pub fog_color: HexColor,
    /// Blocks until the fog is complete.
    #[serde(default = "default_fog_distance")]
    pub fog_distance: f32,
    #[serde(default)]
    pub animation: SurfaceAnimation,
}

impl Default for LiquidView {
    fn default() -> Self {
        Self {
            fog_color: default_fog_color(),
            fog_distance: default_fog_distance(),
            animation: SurfaceAnimation::default(),
        }
    }
}

fn default_fog_color() -> HexColor {
    HexColor(Rgba::rgb(0x1e, 0x4a, 0x8c))
}

fn default_fog_distance() -> f32 {
    16.0
}

/// Height of the surface of each liquid cell in sixteenths of a block, the same for the
/// mesher and the physics. A source fills 14/16, like Minecraft; falling liquid is full.
pub const fn surface_sixteenths(level: FluidLevel) -> u8 {
    match level {
        FluidLevel::Source => 14,
        FluidLevel::Falling => 16,
        FluidLevel::Flowing(n) => {
            let n = if n > MAX_SPREAD { MAX_SPREAD } else { n };
            14 - (12 * n).div_ceil(MAX_SPREAD)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_go_down_with_the_flow_and_never_vanish() {
        let heights: Vec<u8> = (1..=MAX_SPREAD)
            .map(|n| surface_sixteenths(FluidLevel::Flowing(n)))
            .collect();
        assert!(heights.windows(2).all(|w| w[0] > w[1]), "{heights:?}");
        assert!(surface_sixteenths(FluidLevel::Source) > heights[0]);
        assert_eq!(heights.last(), Some(&2));
        assert_eq!(surface_sixteenths(FluidLevel::Falling), 16);
    }
}
