use crate::pack::PackError;
use serde::Deserialize;

/// Slots of a container block, in rows of nine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename = "Container", deny_unknown_fields)]
pub struct ContainerDef {
    pub rows: u8,
    /// Two of them side by side open as one (double chest).
    #[serde(default)]
    pub joins: bool,
}

impl ContainerDef {
    pub const COLUMNS: usize = 9;
    pub const MAX_ROWS: u8 = 6;

    pub fn slots(&self) -> usize {
        usize::from(self.rows) * Self::COLUMNS
    }
}

/// What a work station block opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
pub enum StationDef {
    /// A crafting grid of this side (3 for the crafting table).
    Crafting(u8),
    /// One input cut into the variant chosen in a list (`stonecutting/` recipes).
    Stonecutter,
}

/// A crafting grid of 1 to 3 cells a side; other stations need nothing.
pub(super) fn valid_station(
    station: StationDef,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> bool {
    let StationDef::Crafting(side) = station else {
        return true;
    };
    let valid = (1..=crate::crafting::MAX_GRID as u8).contains(&side);
    if !valid {
        errors.push(PackError::invalid(source, "crafting grid: 1 to 3 cells"));
    }
    valid
}
