//! The fog takes the colour of the biomes around the camera, eased while moving.

use glam::Vec3;

/// Seconds for the fog to get about two thirds of the way to a new biome's colour.
const EASE_SECONDS: f32 = 1.5;

/// Linear factor applied to the horizon and the fog; 1 = the sky's own colour.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct BiomeFog {
    current: Option<Vec3>,
}

impl BiomeFog {
    /// Moves toward `target` (snaps on the first frame); `None` keeps the last colour.
    pub fn follow(&mut self, target: Option<Vec3>, seconds: f32) -> Vec3 {
        let next = match (self.current, target) {
            (Some(current), Some(target)) => {
                current.lerp(target, 1.0 - (-seconds.max(0.0) / EASE_SECONDS).exp())
            }
            (current, target) => target.or(current).unwrap_or(Vec3::ONE),
        };
        self.current = Some(next);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fog_eases_toward_a_new_biome() {
        let mut fog = BiomeFog::default();
        let swamp = Vec3::new(0.7, 0.9, 0.6);
        assert_eq!(fog.follow(None, 0.1), Vec3::ONE);
        let step = fog.follow(Some(swamp), 0.1);
        assert!(step.x < 1.0 && step.x > 0.95, "a small step: {step}");
        for _ in 0..200 {
            fog.follow(Some(swamp), 0.05);
        }
        assert!(
            fog.follow(None, 0.05).distance(swamp) < 1e-3,
            "kept without a biome"
        );
    }

    #[test]
    fn the_first_biome_is_taken_at_once() {
        let mut fog = BiomeFog::default();
        assert_eq!(fog.follow(Some(Vec3::X), 0.0), Vec3::X);
    }
}
