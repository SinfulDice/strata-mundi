//! How a bench's screen looks, from `benches/<name>.ron`: the arrangement of its cells
//! and list, and its colour. A bench without a file gets the plain screen.

use crate::id::Identifier;
use crate::pack::{PackError, PackSet};
use crate::texture::HexColor;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};

/// The arrangement used when a bench names none, or one the interface lacks.
pub const PLAIN_LAYOUT: &str = "plain";

#[derive(Debug, Clone, PartialEq, Default, Deserialize)]
#[serde(rename = "Bench", deny_unknown_fields)]
pub struct BenchStyle {
    /// A group `bench_layout_<layout>` of the inventory screen (`assets/ui/inventory.ron`).
    #[serde(default)]
    pub layout: Option<String>,
    /// The colour of its title band and work surface (the theme's otherwise).
    #[serde(default)]
    pub accent: Option<HexColor>,
}

impl BenchStyle {
    pub const PLAIN: Self = Self {
        layout: None,
        accent: None,
    };

    pub fn layout_name(&self) -> &str {
        self.layout.as_deref().unwrap_or(PLAIN_LAYOUT)
    }
}

/// The styles of `benches/`; one naming no bench of a block (`stations`) is an error.
pub(crate) fn load_styles(
    packs: &PackSet,
    stations: &BTreeSet<Identifier>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, BenchStyle> {
    packs
        .layered::<BenchStyle>("benches", errors)
        .into_iter()
        .filter(|(id, sourced)| {
            let known = stations.contains(id);
            if !known {
                let message = format!("no block opens the bench `{id}`");
                errors.push(PackError::invalid(&sourced.source, message));
            }
            known
        })
        .map(|(id, sourced)| (id, sourced.value))
        .collect()
}
