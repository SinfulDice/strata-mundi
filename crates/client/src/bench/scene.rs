use crate::camera::Camera;
use glam::DVec3;
use serde::Deserialize;
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SceneError {
    #[error("could not read bench scene {path}: {source}")]
    Io {
        path: String,
        source: std::io::Error,
    },
    #[error("invalid bench scene {path}: {source}")]
    Parse {
        path: String,
        source: Box<ron::error::SpannedError>,
    },
    #[error("bench scene {0} needs at least one keyframe and a positive duration")]
    Empty(String),
}

/// Scripted camera path, described in `assets/bench-scenes/<name>.ron`. Positions are
/// relative to the world spawn point.
#[derive(Debug, Clone, Deserialize)]
pub struct BenchScene {
    /// Seconds rendered before measuring (shader warm-up, initial loading).
    pub warmup: f32,
    /// Measured seconds.
    pub duration: f32,
    pub keyframes: Vec<Keyframe>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct Keyframe {
    /// Seconds since the start of the measured part.
    pub time: f32,
    pub pos: (f64, f64, f64),
    /// Degrees, like `--look`.
    pub yaw: f32,
    pub pitch: f32,
}

impl BenchScene {
    pub fn load(path: &Path) -> Result<Self, SceneError> {
        let display = path.display().to_string();
        let text = std::fs::read_to_string(path).map_err(|source| SceneError::Io {
            path: display.clone(),
            source,
        })?;
        Self::parse(&text, &display)
    }

    pub fn parse(text: &str, name: &str) -> Result<Self, SceneError> {
        let scene: Self = ron::from_str(text).map_err(|source| SceneError::Parse {
            path: name.to_owned(),
            source: Box::new(source),
        })?;
        if scene.keyframes.is_empty() || scene.duration <= 0.0 {
            return Err(SceneError::Empty(name.to_owned()));
        }
        Ok(scene)
    }

    /// Camera pose at `time` seconds, linearly interpolated between keyframes.
    pub fn pose(&self, time: f32, camera: &mut Camera) {
        let keys = &self.keyframes;
        let next = keys
            .iter()
            .position(|k| k.time > time)
            .unwrap_or(keys.len());
        let a = keys[next.saturating_sub(1)];
        let b = keys[next.min(keys.len() - 1)];
        let span = (b.time - a.time).max(f32::EPSILON);
        let t = ((time - a.time) / span).clamp(0.0, 1.0);
        let pos = |k: Keyframe| DVec3::new(k.pos.0, k.pos.1, k.pos.2);
        camera.position = pos(a).lerp(pos(b), f64::from(t));
        camera.yaw = (a.yaw + (b.yaw - a.yaw) * t).to_radians();
        camera.pitch = (a.pitch + (b.pitch - a.pitch) * t).to_radians();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENE: &str = "(warmup: 1.0, duration: 10.0, keyframes: [
        (time: 0.0, pos: (0.0, 100.0, 0.0), yaw: 0.0, pitch: 0.0),
        (time: 10.0, pos: (100.0, 100.0, 0.0), yaw: 90.0, pitch: -20.0),
    ])";

    #[test]
    fn pose_interpolates_and_clamps() {
        let scene = BenchScene::parse(SCENE, "test").unwrap();
        let mut camera = Camera::new(DVec3::ZERO, 0.0, 0.0, 70.0);
        scene.pose(5.0, &mut camera);
        assert!((camera.position.x - 50.0).abs() < 1e-6);
        assert!((camera.yaw - 45f32.to_radians()).abs() < 1e-6);
        scene.pose(99.0, &mut camera);
        assert!((camera.position.x - 100.0).abs() < 1e-6);
        scene.pose(-1.0, &mut camera);
        assert!(camera.position.x.abs() < 1e-6);
    }

    #[test]
    fn empty_scene_is_rejected() {
        let error = BenchScene::parse("(warmup: 0.0, duration: 1.0, keyframes: [])", "x");
        assert!(matches!(error, Err(SceneError::Empty(_))));
    }
}
