//! Graphics settings and the quality presets that fill them.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Quality presets. Changing any single graphics setting switches to "custom".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Preset {
    VeryLow,
    Low,
    Medium,
    High,
    Ultra,
    /// The player's handheld console (Intel N100, 30 FPS target).
    Ds,
}

impl Preset {
    pub const ALL: [Preset; 6] = [
        Self::VeryLow,
        Self::Low,
        Self::Medium,
        Self::High,
        Self::Ultra,
        Self::Ds,
    ];

    pub const fn id(self) -> &'static str {
        match self {
            Self::VeryLow => "very-low",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
            Self::Ultra => "ultra",
            Self::Ds => "ds",
        }
    }
}

impl fmt::Display for Preset {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.id())
    }
}

impl FromStr for Preset {
    type Err = String;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|preset| preset.id() == text)
            .ok_or_else(|| {
                let names: Vec<_> = Self::ALL.iter().map(|p| p.id()).collect();
                format!(
                    "unknown preset `{text}` (expected one of: {})",
                    names.join(", ")
                )
            })
    }
}

/// Every graphics option of v0.1. Threads at 0 mean "automatic".
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GraphicsSettings {
    /// Columns loaded around the player, in each direction; can change while playing.
    pub render_distance: u32,
    /// Sections meshed above and below the player's section.
    pub vertical_distance: u32,
    /// 3D image resolution, in percent of the window (the interface stays sharp).
    pub render_scale: u32,
    /// Frames per second at most; 0 = no limit.
    pub fps_limit: u32,
    pub vsync: bool,
    /// Vertical field of view, in degrees.
    pub fov: u32,
    pub ambient_occlusion: bool,
    /// Fades the far terrain into the sky, hiding where the world stops.
    pub fog: bool,
    /// Smaller copies of textures for distant blocks (less shimmering).
    pub mipmaps: bool,
    /// Anisotropic filtering: 1 (off), 2, 4, 8 or 16.
    pub anisotropy: u32,
    /// Background threads for meshing.
    pub worker_threads: usize,
    /// Background threads for world generation (integrated server).
    pub generation_threads: usize,
}

impl Default for GraphicsSettings {
    fn default() -> Self {
        Self::for_preset(Preset::High)
    }
}

impl GraphicsSettings {
    pub fn for_preset(preset: Preset) -> Self {
        let (render_distance, vertical_distance, render_scale, ambient_occlusion) = match preset {
            Preset::VeryLow => (4, 3, 75, false),
            Preset::Low => (6, 4, 100, false),
            Preset::Medium => (8, 6, 100, true),
            Preset::High => (12, 8, 100, true),
            Preset::Ultra => (16, 12, 100, true),
            Preset::Ds => (6, 4, 75, true),
        };
        let threads = if preset == Preset::Ds {
            1
        } else {
            AUTO_THREADS
        };
        Self {
            render_distance,
            vertical_distance,
            render_scale,
            fps_limit: if preset == Preset::Ds { 30 } else { 0 },
            vsync: true,
            fov: 70,
            ambient_occlusion,
            fog: true,
            mipmaps: !matches!(preset, Preset::VeryLow | Preset::Ds),
            anisotropy: match preset {
                Preset::High => 4,
                Preset::Ultra => 16,
                _ => 1,
            },
            worker_threads: threads,
            generation_threads: threads,
        }
    }

    /// Threads to start for meshing, `AUTO_THREADS` resolved for this machine.
    pub fn meshing_threads(&self) -> usize {
        resolve_threads(self.worker_threads)
    }

    pub fn world_threads(&self) -> usize {
        resolve_threads(self.generation_threads)
    }
}

/// Stored value meaning "as many as the machine allows".
pub const AUTO_THREADS: usize = 0;

/// Every core but the render and server threads, at least one (two on a 4-core N100).
/// Bounds of the range settings, shared by the options screen and file checks.
pub const RENDER_DISTANCE: (u32, u32) = (2, 32);
pub const VERTICAL_DISTANCE: (u32, u32) = (2, 16);
pub const RENDER_SCALE: (u32, u32) = (50, 200);
pub const FOV: (u32, u32) = (50, 110);
pub const ANISOTROPY: [u32; 5] = [1, 2, 4, 8, 16];
/// More threads than this is never useful, whatever the file says.
const MAX_THREADS: usize = 64;

impl GraphicsSettings {
    /// Values from a hand-edited file brought back into their ranges.
    pub fn sanitize(&mut self) {
        let clamp = |value: &mut u32, (min, max): (u32, u32)| *value = (*value).clamp(min, max);
        clamp(&mut self.render_distance, RENDER_DISTANCE);
        clamp(&mut self.vertical_distance, VERTICAL_DISTANCE);
        clamp(&mut self.render_scale, RENDER_SCALE);
        clamp(&mut self.fov, FOV);
        if !ANISOTROPY.contains(&self.anisotropy) {
            self.anisotropy = 1;
        }
        self.worker_threads = self.worker_threads.min(MAX_THREADS);
        self.generation_threads = self.generation_threads.min(MAX_THREADS);
    }
}

pub fn default_worker_threads() -> usize {
    std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(2).max(1))
}

fn resolve_threads(chosen: usize) -> usize {
    if chosen == AUTO_THREADS {
        default_worker_threads()
    } else {
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preset_ids_round_trip() {
        for preset in Preset::ALL {
            assert_eq!(preset.id().parse::<Preset>(), Ok(preset));
        }
        assert!("potato".parse::<Preset>().is_err());
    }

    #[test]
    fn the_ds_preset_is_light() {
        let ds = GraphicsSettings::for_preset(Preset::Ds);
        assert_eq!((ds.fps_limit, ds.meshing_threads()), (30, 1));
        assert!(ds.render_distance < GraphicsSettings::for_preset(Preset::Ultra).render_distance);
    }
}
