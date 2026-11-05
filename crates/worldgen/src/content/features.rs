//! Features (trees, pillars, plants, ores, deposits) resolved from `features/`.

use super::Resolver;
use crate::defs::FeatureDef;
use crate::deposits::Deposit;
use crate::fnv1a;
use crate::trees::{Tree, TreeDef};
use std::collections::BTreeMap;
use strata_core::content::{Block, BlockId, Registry};
use strata_core::id::Identifier;
use strata_core::pack::{PackError, PackSet, Sourced};

#[derive(Debug, Clone)]
pub enum Feature {
    Tree(Tree),
    Pillar {
        block: BlockId,
        height: (u32, u32),
        on: Vec<BlockId>,
    },
    Plants(Plants),
    Ore {
        block: BlockId,
        deep_block: Option<BlockId>,
        vein_size: u32,
        per_column: f64,
        min_y: i32,
        max_y: i32,
    },
    Deposit(Deposit),
}

#[derive(Debug, Clone)]
pub struct Plants {
    /// `(block, weight)`, weights above zero.
    pub blocks: Vec<(BlockId, u32)>,
    pub on: Vec<BlockId>,
    pub radius: u32,
    pub count: (u32, u32),
    /// Hash of the feature's identifier: patches do not move when other features change.
    pub salt: u64,
}

impl Feature {
    /// Blocks it adds above the ground (trees, cactus, plants).
    pub fn surface_blocks(&self) -> Vec<BlockId> {
        match self {
            Self::Tree(tree) => [tree.log, tree.leaves]
                .into_iter()
                .chain(tree.vines)
                .collect(),
            Self::Pillar { block, .. } => vec![*block],
            Self::Plants(plants) => plants.blocks.iter().map(|(block, _)| *block).collect(),
            Self::Ore { .. } | Self::Deposit(_) => Vec::new(),
        }
    }
}

pub(super) fn load_features(
    packs: &PackSet,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, Feature> {
    let defs = packs.layered::<FeatureDef>("features", errors);
    defs.into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            let mut r = Resolver {
                blocks,
                errors,
                source: &source,
            };
            let feature = match value {
                FeatureDef::Tree {
                    log,
                    leaves,
                    height,
                    crown,
                    trunk,
                    vines,
                    on,
                } => {
                    let def = TreeDef {
                        log: &log,
                        leaves: &leaves,
                        height,
                        crown,
                        trunk,
                        vines: vines.as_ref(),
                        on: &on,
                    };
                    Feature::Tree(Tree::resolve(def, &mut r))
                }
                FeatureDef::Pillar { block, height, on } => Feature::Pillar {
                    block: r.block(&block),
                    height: bounded(height, MAX_PILLAR_HEIGHT),
                    on: r.blocks(&on),
                },
                FeatureDef::Plants {
                    blocks,
                    on,
                    radius,
                    count,
                } => Feature::Plants(Plants {
                    blocks: blocks
                        .iter()
                        .filter(|(_, weight)| *weight > 0)
                        .map(|(block, weight)| (r.block(block), *weight))
                        .collect(),
                    on: r.blocks(&on),
                    radius: radius.min(MAX_PATCH_RADIUS),
                    count: bounded(count, MAX_PATCH_PLANTS),
                    salt: fnv1a(id.to_string().as_bytes()),
                }),
                FeatureDef::Ore {
                    block,
                    deep_block,
                    vein_size,
                    per_column,
                    min_y,
                    max_y,
                } => Feature::Ore {
                    block: r.block(&block),
                    deep_block: deep_block.map(|id| r.block(&id)),
                    vein_size: vein_size.min(64),
                    per_column: per_column.clamp(0.0, 256.0),
                    min_y,
                    max_y,
                },
                def @ FeatureDef::Deposit { .. } => {
                    let deposit = Deposit::resolve(&id, &def, &mut r);
                    return deposit.map(|d| (id, Feature::Deposit(d)));
                }
            };
            Some((id, feature))
        })
        .collect()
}

/// Features are written from the 3×3 neighbouring columns: anything wider than 16 blocks
/// from its root would be cut, so sizes are bounded.
pub const MAX_CROWN: u32 = 8;
const MAX_PILLAR_HEIGHT: u32 = 32;
/// Patches are written from the 3×3 neighbouring columns too.
pub const MAX_PATCH_RADIUS: u32 = MAX_CROWN;
const MAX_PATCH_PLANTS: u32 = 64;

pub(crate) fn bounded((a, b): (u32, u32), max: u32) -> (u32, u32) {
    (a.min(b).min(max), a.max(b).min(max))
}
