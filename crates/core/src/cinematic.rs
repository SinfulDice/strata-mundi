//! Cinematics: scripted camera shots described in packs (`cinematics/<name>.ron`), played
//! by the client over the world (boss entrances, discoveries…). Positions are relative to
//! the point the shot is played from, so one shot fits anywhere.

use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use glam::DVec3;
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Deserialize)]
pub struct Cinematic {
    /// At least two keyframes, in time order.
    pub keyframes: Vec<CameraKey>,
    /// The player may skip it (Escape / B); on by default.
    #[serde(default = "yes")]
    pub skippable: bool,
    /// Black bands above and below, like a film.
    #[serde(default = "yes")]
    pub letterbox: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CameraKey {
    /// Seconds from the start.
    pub time: f32,
    pub pos: (f64, f64, f64),
    /// Degrees, like `--look`.
    pub yaw: f32,
    pub pitch: f32,
    /// Translation key shown from this keyframe until the next one with a subtitle.
    #[serde(default)]
    pub subtitle: Option<String>,
}

/// Where the camera is at some moment of a shot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pose {
    pub position: DVec3,
    pub yaw_degrees: f32,
    pub pitch_degrees: f32,
}

fn yes() -> bool {
    true
}

impl Cinematic {
    pub fn duration(&self) -> f32 {
        self.keyframes.last().map_or(0.0, |key| key.time)
    }

    /// Smooth path through the keyframes (Catmull-Rom), angles turning the short way.
    pub fn pose(&self, time: f32, origin: DVec3) -> Pose {
        let keys = &self.keyframes;
        let next = keys
            .iter()
            .position(|k| k.time > time)
            .unwrap_or(keys.len())
            .clamp(1, keys.len().saturating_sub(1).max(1));
        let at = |i: usize| &keys[i.min(keys.len() - 1)];
        let (a, b) = (at(next - 1), at(next));
        let span = (b.time - a.time).max(f32::EPSILON);
        let t = f64::from(((time - a.time) / span).clamp(0.0, 1.0));
        let point = |k: &CameraKey| DVec3::new(k.pos.0, k.pos.1, k.pos.2);
        let before = point(at(next.saturating_sub(2)));
        let after = point(at(next + 1));
        let position = catmull_rom(before, point(a), point(b), after, t);
        let eased = smoothstep(t as f32);
        Pose {
            position: origin + position,
            yaw_degrees: a.yaw + shortest_turn(a.yaw, b.yaw) * eased,
            pitch_degrees: a.pitch + (b.pitch - a.pitch) * eased,
        }
    }

    /// Translation key of the subtitle showing at `time`.
    pub fn subtitle(&self, time: f32) -> Option<&str> {
        self.keyframes
            .iter()
            .take_while(|key| key.time <= time)
            .filter_map(|key| key.subtitle.as_deref())
            .last()
    }

    fn check(&self) -> Result<(), &'static str> {
        if self.keyframes.len() < 2 {
            return Err("a cinematic needs at least two keyframes");
        }
        let ordered = self.keyframes.windows(2).all(|w| w[0].time < w[1].time);
        let finite = self.keyframes.iter().all(|k| {
            k.time.is_finite() && k.pos.0.is_finite() && k.pos.1.is_finite() && k.pos.2.is_finite()
        });
        if !ordered || !finite {
            return Err("keyframe times must increase and values be finite");
        }
        Ok(())
    }
}

/// Every cinematic of the packs; broken ones are reported and left out.
pub fn load(packs: &PackSet, errors: &mut Vec<PackError>) -> BTreeMap<Identifier, Cinematic> {
    packs
        .layered::<Cinematic>("cinematics", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| match value.check() {
            Ok(()) => Some((id, value)),
            Err(message) => {
                errors.push(PackError::invalid(&source, message));
                None
            }
        })
        .collect()
}

fn catmull_rom(p0: DVec3, p1: DVec3, p2: DVec3, p3: DVec3, t: f64) -> DVec3 {
    let (t2, t3) = (t * t, t * t * t);
    0.5 * ((2.0 * p1)
        + (p2 - p0) * t
        + (2.0 * p0 - 5.0 * p1 + 4.0 * p2 - p3) * t2
        + (3.0 * p1 - p0 - 3.0 * p2 + p3) * t3)
}

fn smoothstep(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

/// From `from` to `to` degrees, at most half a turn either way.
fn shortest_turn(from: f32, to: f32) -> f32 {
    (to - from + 180.0).rem_euclid(360.0) - 180.0
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn shot() -> Cinematic {
        ron::from_str(
            r#"(keyframes: [
                (time: 0.0, pos: (0.0, 10.0, 0.0), yaw: 350.0, pitch: 0.0, subtitle: Some("a")),
                (time: 2.0, pos: (10.0, 10.0, 0.0), yaw: 10.0, pitch: -20.0),
                (time: 4.0, pos: (20.0, 10.0, 0.0), yaw: 20.0, pitch: 0.0, subtitle: Some("b")),
            ])"#,
        )
        .unwrap()
    }

    #[test]
    fn the_path_goes_through_every_keyframe() {
        let shot = shot();
        let origin = DVec3::new(100.0, 0.0, 0.0);
        assert_eq!(
            shot.pose(0.0, origin).position,
            DVec3::new(100.0, 10.0, 0.0)
        );
        let middle = shot.pose(2.0, origin);
        assert!((middle.position - DVec3::new(110.0, 10.0, 0.0)).length() < 1e-9);
        assert!((shot.pose(9.0, origin).position.x - 120.0).abs() < 1e-9);
        assert!(shot.check().is_ok() && (shot.duration() - 4.0).abs() < f32::EPSILON);
    }

    #[test]
    fn yaw_turns_the_short_way_and_subtitles_follow() {
        let shot = shot();
        let yaw = shot.pose(1.0, DVec3::ZERO).yaw_degrees;
        assert!(
            (yaw - 360.0).abs() < 1.0,
            "350° → 10° passes by 360°, got {yaw}"
        );
        assert_eq!(shot.subtitle(1.0), Some("a"));
        assert_eq!(shot.subtitle(4.0), Some("b"));
    }

    #[test]
    fn unordered_keyframes_are_refused() {
        let mut shot = shot();
        shot.keyframes.swap(0, 1);
        assert!(shot.check().is_err());
    }
}
