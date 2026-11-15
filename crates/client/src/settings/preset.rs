//! Quality presets: names, parsing, the console profile.

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
