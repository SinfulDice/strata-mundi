use std::fmt;
use std::str::FromStr;

/// Quality presets. Changing any single setting switches to "custom" (milestone 8).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
}

/// Graphics settings that exist so far; the full options screen arrives at milestone 8.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphicsSettings {
    /// Sections meshed above and below the player's section.
    pub vertical_distance: u32,
    pub ambient_occlusion: bool,
    /// Background threads for meshing (and generation, from milestone 4).
    pub worker_threads: usize,
}

impl GraphicsSettings {
    pub fn for_preset(preset: Preset) -> Self {
        let (vertical_distance, ambient_occlusion) = match preset {
            Preset::VeryLow => (2, false),
            Preset::Low => (3, false),
            Preset::Medium => (4, true),
            Preset::High => (6, true),
            Preset::Ultra => (8, true),
            Preset::Ds => (3, true),
        };
        Self {
            vertical_distance,
            ambient_occlusion,
            worker_threads: default_worker_threads(),
        }
    }
}

/// Every core but the render and server threads, at least one (two on a 4-core N100).
pub fn default_worker_threads() -> usize {
    std::thread::available_parallelism().map_or(2, |n| n.get().saturating_sub(2).max(1))
}
