//! How a bench's screen looks, from `benches/<name>.ron`: the arrangement of its cells
//! and list, and its colour. A bench without a file gets the plain screen.

use crate::id::Identifier;
use crate::pack::{PackError, PackSet};
use crate::texture::HexColor;
use serde::Deserialize;
use std::collections::BTreeMap;

/// The arrangements the inventory screen has for benches (`assets/ui/inventory.ron`,
/// groups `bench_layout_<name>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum BenchLayout {
    #[default]
    Plain,
    Jeweller,
    Forge,
    Tannery,
    Loom,
}

impl BenchLayout {
    pub const ALL: [Self; 5] = [
        Self::Plain,
        Self::Jeweller,
        Self::Forge,
        Self::Tannery,
        Self::Loom,
    ];

    /// The name in the interface's group ids.
    pub fn name(self) -> &'static str {
        match self {
            Self::Plain => "plain",
            Self::Jeweller => "jeweller",
            Self::Forge => "forge",
            Self::Tannery => "tannery",
            Self::Loom => "loom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(rename = "Bench", deny_unknown_fields)]
pub struct BenchStyle {
    #[serde(default)]
    pub layout: BenchLayout,
    /// The colour of its title band and work surface (the theme's otherwise).
    #[serde(default)]
    pub accent: Option<HexColor>,
}

pub(crate) fn load_styles(
    packs: &PackSet,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, BenchStyle> {
    packs
        .layered::<BenchStyle>("benches", errors)
        .into_iter()
        .map(|(id, sourced)| (id, sourced.value))
        .collect()
}
