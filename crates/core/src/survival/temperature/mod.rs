//! Temperature: felt by any body, from named contributions of the surroundings and of
//! clothing, followed slowly; its zone strains (cold: slower, hungrier; heat: thirstier,
//! less stamina) then hurts (`strata:freezing`, `strata:overheating`).

mod body;
mod felt;
mod heat;
mod rules;
mod scan;

pub use body::BodyTemperature;
pub use felt::{
    ALTITUDE, BIOME, CLOTHING, Contribution, DIMENSION, HEAT, Insulation, NIGHT, PartName, SHELTER,
    Surroundings, WATER, contributions, target, total,
};
pub use heat::{HeatSource, HeatSourceDef, HeatSources, MAX_REACH};
pub use rules::{
    Altitude, BiomeTemperature, Clothing, Shelter, Strain, TemperatureRules, Water, ZoneHarm, Zones,
};
pub use scan::{HeatFound, strongest_heat};

pub(crate) use heat::{HeatKnown, load_heat_sources};

use serde::{Deserialize, Serialize};

/// Where a felt temperature falls, from the coldest.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TemperatureZone {
    Freezing,
    Cold,
    #[default]
    Comfortable,
    Hot,
    Scorching,
}

impl TemperatureZone {
    pub fn key(self) -> &'static str {
        match self {
            Self::Freezing => "freezing",
            Self::Cold => "cold",
            Self::Comfortable => "comfortable",
            Self::Hot => "hot",
            Self::Scorching => "scorching",
        }
    }

    /// How far from comfort, 0 to 2.
    pub fn severity(self) -> u8 {
        match self {
            Self::Comfortable => 0,
            Self::Cold | Self::Hot => 1,
            Self::Freezing | Self::Scorching => 2,
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
