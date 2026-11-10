//! World types read from `world_types/`, checked against the blocks and biomes.

use super::Biome;
use crate::defs::{ReliefChanges, WorldTypeDef};
use std::collections::HashMap;
use strata_core::content::BlockId;
use strata_core::generation::{FlatLayer, FlatSettings};
use strata_core::id::Identifier;
use strata_core::pack::{PackError, PackSet, Sourced};

#[derive(Debug, Clone)]
pub struct WorldType {
    pub id: Identifier,
    pub order: i32,
    pub relief: ReliefChanges,
    /// The biome forced everywhere by default.
    pub biome: Option<Identifier>,
    /// Default layers and options of flat types.
    pub flat: Option<FlatSettings>,
}

impl WorldType {
    /// Fluent message of its name.
    pub fn name_key(&self) -> String {
        self.id.translation_key("world-type")
    }
}

pub(super) fn load(
    packs: &PackSet,
    blocks: &HashMap<Identifier, BlockId>,
    biomes: &[Biome],
    errors: &mut Vec<PackError>,
) -> Vec<WorldType> {
    let defs = packs.layered::<WorldTypeDef>("world_types", errors);
    let mut types: Vec<WorldType> = defs
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            let mut invalid = |message: String| errors.push(PackError::invalid(&source, message));
            if let Some(biome) = value
                .biome
                .as_ref()
                .filter(|b| !biomes.iter().any(|x| &x.id == *b))
            {
                invalid(format!("unknown biome `{biome}`"));
                return None;
            }
            let flat = value.flat.map(|flat| FlatSettings {
                layers: flat
                    .layers
                    .into_iter()
                    .map(|(block, thickness)| FlatLayer { block, thickness })
                    .collect(),
                caves: flat.caves,
                features: flat.features,
            });
            if let Some(layer) = flat
                .iter()
                .flat_map(|flat| &flat.layers)
                .find(|layer| !blocks.contains_key(&layer.block))
            {
                invalid(format!("unknown block `{}`", layer.block));
                return None;
            }
            if flat.is_some() && value.biome.is_none() {
                invalid("a flat world type needs a `biome`".to_owned());
                return None;
            }
            Some(WorldType {
                id,
                order: value.order,
                relief: value.relief,
                biome: value.biome,
                flat: flat.map(FlatSettings::bounded),
            })
        })
        .collect();
    types.sort_by(|a, b| a.order.cmp(&b.order).then_with(|| a.id.cmp(&b.id)));
    types
}
