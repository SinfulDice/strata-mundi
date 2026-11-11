//! Under ground, the fog fades to the dimension's cave colour as the sky light at the
//! eyes drops, eased like the biome tint so walking into a cave never snaps.

use super::biome_fog::ease;

/// How open to the sky the eyes are, from their sky light (0-15): 0 deep under ground,
/// 1 under the open sky. Squared, so a cave mouth already darkens the fog a little.
pub fn sky_openness(sky_light: u8) -> f32 {
    let open = f32::from(sky_light.min(15)) / 15.0;
    open * open
}

/// The eased openness; open sky until anything is known.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CaveFog {
    current: Option<f32>,
}

impl CaveFog {
    /// Moves toward the openness of `sky_light` (snaps on the first frame).
    pub fn follow(&mut self, sky_light: u8, seconds: f32) -> f32 {
        let target = sky_openness(sky_light);
        let next = match self.current {
            Some(current) => current + (target - current) * ease(seconds),
            None => target,
        };
        self.current = Some(next);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openness_spans_cave_to_open_sky() {
        assert_eq!(sky_openness(0), 0.0);
        assert_eq!(sky_openness(15), 1.0);
        assert!(sky_openness(8) > 0.0 && sky_openness(8) < 0.5);
    }

    #[test]
    fn the_cave_fog_eases_in() {
        let mut fog = CaveFog::default();
        assert_eq!(fog.follow(15, 0.1), 1.0, "the first frame snaps");
        let step = fog.follow(0, 0.1);
        assert!(step < 1.0 && step > 0.9, "a small step: {step}");
        for _ in 0..300 {
            fog.follow(0, 0.05);
        }
        assert!(fog.follow(0, 0.05) < 1e-3);
    }
}
