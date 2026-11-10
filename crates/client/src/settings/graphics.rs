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

/// How many plants (crossed planes) are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Vegetation {
    #[default]
    Full,
    /// One plant in `REDUCED_SHARE`, chosen by position so it never flickers.
    Reduced,
    Off,
}

impl Vegetation {
    pub const ALL: [Vegetation; 3] = [Self::Full, Self::Reduced, Self::Off];
    pub const REDUCED_SHARE: u32 = 3;

    pub const fn id(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Reduced => "reduced",
            Self::Off => "off",
        }
    }
}

/// How the clouds are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloudMode {
    Off,
    /// One flat layer: a single face per cloud cell.
    Flat,
    /// Volumes with sides, top and underside.
    #[default]
    Full,
}

impl CloudMode {
    pub const ALL: [CloudMode; 3] = [Self::Off, Self::Flat, Self::Full];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Flat => "flat",
            Self::Full => "full",
        }
    }
}

/// How many particles (block fragments, drips, bubbles) are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParticleMode {
    Off,
    /// A quarter of each burst, fewer drips, a smaller pool.
    Reduced,
    #[default]
    Full,
}

impl ParticleMode {
    pub const ALL: [ParticleMode; 3] = [Self::Off, Self::Reduced, Self::Full];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Reduced => "reduced",
            Self::Full => "full",
        }
    }
}

/// How light is computed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Lighting {
    /// Sky and lamps, on a background thread.
    #[default]
    Full,
    /// Nothing computed: everything is lit like the open sky.
    Flat,
}

impl Lighting {
    pub const ALL: [Lighting; 2] = [Self::Full, Self::Flat];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Full => "full",
            Self::Flat => "flat",
        }
    }
}

/// Every graphics option. Threads at 0 mean "automatic".
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
    pub lighting: Lighting,
    /// Light blended across each face from the cells around its corners.
    pub smooth_lighting: bool,
    /// Lifts dark areas, in percent (0 = moody, 100 = bright); the same in every preset.
    pub brightness: u32,
    /// Fades the far terrain into the sky, hiding where the world stops.
    pub fog: bool,
    /// Moving textures (water, lava, fire, portals) and liquid ripples, in the shader.
    #[serde(alias = "liquid_animation")]
    pub animated_textures: bool,
    /// Random variants by position and connected textures (glass): more faces to draw,
    /// since only identical neighbouring faces merge.
    pub texture_variants: bool,
    /// Blocks over which grass, leaf and water colours blend at biome borders (0: sharp).
    pub biome_blend: u32,
    /// Twinkling stars in the night sky.
    pub stars: bool,
    pub clouds: CloudMode,
    /// Smaller copies of textures for distant blocks (less shimmering).
    pub mipmaps: bool,
    /// Anisotropic filtering: 1 (off), 2, 4, 8 or 16.
    pub anisotropy: u32,
    pub vegetation: Vegetation,
    pub particles: ParticleMode,
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
            lighting: Lighting::Full,
            smooth_lighting: preset != Preset::VeryLow,
            brightness: DEFAULT_BRIGHTNESS,
            fog: true,
            animated_textures: preset != Preset::VeryLow,
            texture_variants: preset != Preset::VeryLow,
            biome_blend: match preset {
                Preset::VeryLow => 0,
                Preset::Low | Preset::Ds => 1,
                Preset::Medium => 2,
                Preset::High => 3,
                Preset::Ultra => 4,
            },
            stars: preset != Preset::VeryLow,
            clouds: match preset {
                Preset::VeryLow => CloudMode::Off,
                Preset::Low | Preset::Ds => CloudMode::Flat,
                _ => CloudMode::Full,
            },
            mipmaps: !matches!(preset, Preset::VeryLow | Preset::Ds),
            // Off in every preset: it smooths the pixel art up close (left to taste).
            anisotropy: 1,
            vegetation: match preset {
                Preset::VeryLow => Vegetation::Off,
                Preset::Ds => Vegetation::Reduced,
                _ => Vegetation::Full,
            },
            particles: match preset {
                Preset::VeryLow => ParticleMode::Off,
                Preset::Low | Preset::Ds => ParticleMode::Reduced,
                _ => ParticleMode::Full,
            },
            worker_threads: threads,
            generation_threads: threads,
        }
    }

    /// Threads to start for meshing, `AUTO_THREADS` resolved for this machine. The
    /// lighting thread takes the place of one of them when there are at least two.
    pub fn meshing_threads(&self) -> usize {
        let threads = resolve_threads(self.worker_threads);
        if self.lighting == Lighting::Full && threads >= 2 {
            threads - 1
        } else {
            threads
        }
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
pub const BRIGHTNESS: (u32, u32) = (0, 100);
pub const BIOME_BLEND: (u32, u32) = (0, crate::mesher::MAX_BLEND_RADIUS);
pub const DEFAULT_BRIGHTNESS: u32 = 50;
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
        clamp(&mut self.brightness, BRIGHTNESS);
        clamp(&mut self.biome_blend, BIOME_BLEND);
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
#[allow(clippy::unwrap_used)]
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
        assert_eq!(ds.vegetation, Vegetation::Reduced);
        assert_eq!(ds.clouds, CloudMode::Flat);
        assert_eq!(ds.particles, ParticleMode::Reduced);
        let very_low = GraphicsSettings::for_preset(Preset::VeryLow);
        assert_eq!(very_low.particles, ParticleMode::Off);
    }

    #[test]
    fn cloud_presets_go_from_none_to_volumes() {
        let clouds = |preset| GraphicsSettings::for_preset(preset).clouds;
        assert_eq!(clouds(Preset::VeryLow), CloudMode::Off);
        assert_eq!(clouds(Preset::Low), CloudMode::Flat);
        assert_eq!(clouds(Preset::High), CloudMode::Full);
        assert_eq!(clouds(Preset::Ultra), CloudMode::Full);
        let file: GraphicsSettings = toml::from_str("clouds = \"flat\"").unwrap();
        assert_eq!(file.clouds, CloudMode::Flat);
    }

    #[test]
    fn the_lighting_thread_counts_among_the_worker_threads() {
        let mut graphics = GraphicsSettings {
            worker_threads: 3,
            ..GraphicsSettings::default()
        };
        assert_eq!(graphics.meshing_threads(), 2);
        graphics.lighting = Lighting::Flat;
        assert_eq!(graphics.meshing_threads(), 3);
        graphics.worker_threads = 1;
        graphics.lighting = Lighting::Full;
        assert_eq!(graphics.meshing_threads(), 1);
    }
}
