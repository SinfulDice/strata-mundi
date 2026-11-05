//! The cloud layer of a sky (`clouds` of `skies/<name>.ron`): where it floats, how much
//! of the sky it covers and how it drifts with world time.

use crate::texture::{HexColor, Rgba};
use serde::Deserialize;

/// World time runs at 20 ticks per second.
const TICKS_PER_SECOND: f64 = 20.0;
/// The world's build height: a layer above it would never be seen near the ground.
const MAX_HEIGHT: f32 = 512.0;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CloudsDef {
    /// Height of the layer's underside, in blocks.
    pub height: f32,
    /// Thickness of the volume (complete clouds), in blocks.
    pub thickness: f32,
    /// Side of one square cloud cell, in blocks.
    pub cell_size: f32,
    /// Share of the sky covered, 0 (clear) to 1 (overcast).
    pub coverage: f32,
    /// 0 (invisible) to 1 (opaque).
    pub opacity: f32,
    /// Drift, in blocks per second of world time.
    pub speed: f32,
    /// Where the clouds drift to, in degrees: 0 toward east (+X), 90 toward south (+Z).
    pub direction: f32,
    pub day: HexColor,
    /// At sunrise and sunset.
    pub twilight: HexColor,
    pub night: HexColor,
}

/// A cloud layer with its values brought into their ranges.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clouds {
    pub height: f32,
    pub thickness: f32,
    pub cell_size: f32,
    pub coverage: f32,
    pub opacity: f32,
    /// Blocks per tick, along X and Z.
    pub velocity: [f64; 2],
    pub day: Rgba,
    pub twilight: Rgba,
    pub night: Rgba,
}

impl Clouds {
    pub fn resolve(def: &CloudsDef) -> Self {
        let speed = f64::from(bounded(def.speed, 0.0, 50.0)) / TICKS_PER_SECOND;
        let direction = f64::from(bounded(def.direction, -360.0, 360.0)).to_radians();
        Self {
            height: bounded(def.height, 0.0, MAX_HEIGHT),
            thickness: bounded(def.thickness, 1.0, 32.0),
            cell_size: bounded(def.cell_size, 2.0, 64.0),
            coverage: bounded(def.coverage, 0.0, 1.0),
            opacity: bounded(def.opacity, 0.0, 1.0),
            velocity: [speed * direction.cos(), speed * direction.sin()],
            day: def.day.0,
            twilight: def.twilight.0,
            night: def.night.0,
        }
    }

    /// How far the clouds have moved at world time `time` (ticks): the same time always
    /// gives the same place, so they move on smoothly and match after a reload.
    pub fn drift(&self, time: f64) -> [f64; 2] {
        let time = if time.is_finite() { time } else { 0.0 };
        self.velocity.map(|v| v * time)
    }
}

pub(super) fn bounded(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def() -> CloudsDef {
        let white = HexColor(Rgba([255; 4]));
        CloudsDef {
            height: 300.0,
            thickness: 4.0,
            cell_size: 12.0,
            coverage: 0.4,
            opacity: 0.9,
            speed: 2.0,
            direction: 90.0,
            day: white,
            twilight: white,
            night: white,
        }
    }

    #[test]
    fn pack_values_are_bounded() {
        let wild = CloudsDef {
            height: 9000.0,
            thickness: -3.0,
            cell_size: f32::NAN,
            coverage: 4.0,
            opacity: -1.0,
            speed: f32::INFINITY,
            ..def()
        };
        let clouds = Clouds::resolve(&wild);
        assert_eq!(clouds.height, MAX_HEIGHT);
        assert_eq!((clouds.thickness, clouds.cell_size), (1.0, 2.0));
        assert_eq!((clouds.coverage, clouds.opacity), (1.0, 0.0));
        assert_eq!(clouds.velocity, [0.0, 0.0]);
        let low = Clouds::resolve(&CloudsDef {
            coverage: -0.5,
            ..def()
        });
        assert_eq!(low.coverage, 0.0);
    }

    #[test]
    fn the_drift_is_continuous_and_follows_the_direction() {
        let clouds = Clouds::resolve(&def());
        assert_eq!(clouds.drift(0.0), [0.0, 0.0]);
        let second = clouds.drift(20.0);
        assert!(second[0].abs() < 1e-9 && (second[1] - 2.0).abs() < 1e-9);
        let mut previous = clouds.drift(1_000_000.0);
        for step in 1..=200 {
            let now = clouds.drift(1_000_000.0 + f64::from(step) * 0.25);
            let moved = (now[1] - previous[1]).abs() + (now[0] - previous[0]).abs();
            assert!(
                (moved - 0.025).abs() < 1e-6,
                "smooth at step {step}: {moved}"
            );
            previous = now;
        }
        assert_eq!(clouds.drift(f64::NAN), [0.0, 0.0]);
    }
}
