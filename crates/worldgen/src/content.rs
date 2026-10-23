use crate::defs::{BiomeDef, Climate, Crown, DimensionDef, FeatureDef, TerrainDef};
use std::collections::BTreeMap;
use std::path::Path;
use strata_core::content::{Block, BlockId, Registry};
use strata_core::id::Identifier;
use strata_core::pack::{PackError, PackSet, Sourced};

/// Generation content with block identifiers resolved to numeric ids. Only built by
/// `load`, which guarantees at least one biome.
#[derive(Debug, Clone)]
pub struct WorldgenContent {
    pub terrain: Terrain,
    biomes: Vec<Biome>,
    pub features: Vec<Feature>,
}

#[derive(Debug, Clone)]
pub struct Terrain {
    pub sea_level: i32,
    pub deep_level: i32,
    pub stone: BlockId,
    pub deep_stone: BlockId,
    pub bedrock: BlockId,
    pub water: BlockId,
    pub ice: BlockId,
    pub biome_scale: f64,
    pub caves: bool,
    /// Indices into `WorldgenContent::features`.
    pub ores: Vec<usize>,
}

#[derive(Debug, Clone)]
pub struct Biome {
    pub id: Identifier,
    pub climate: Climate,
    pub surface: BlockId,
    pub subsurface: BlockId,
    pub subsurface_depth: u32,
    pub underwater: BlockId,
    /// Sorted by height.
    pub peaks: Vec<(i32, BlockId)>,
    pub frozen: bool,
    pub spawn: bool,
    /// `(feature index, attempts per column)`.
    pub features: Vec<(usize, f64)>,
}

#[derive(Debug, Clone)]
pub enum Feature {
    Tree {
        log: BlockId,
        leaves: BlockId,
        height: (u32, u32),
        crown: Crown,
        on: Vec<BlockId>,
    },
    Pillar {
        block: BlockId,
        height: (u32, u32),
        on: Vec<BlockId>,
    },
    Ore {
        block: BlockId,
        deep_block: Option<BlockId>,
        vein_size: u32,
        per_column: f64,
        min_y: i32,
        max_y: i32,
    },
}

/// Looks up identifiers, recording an error (with the file) for each unknown one.
struct Resolver<'a> {
    blocks: &'a Registry<BlockId, Block>,
    errors: &'a mut Vec<PackError>,
    source: &'a Path,
}

impl Resolver<'_> {
    fn block(&mut self, id: &Identifier) -> BlockId {
        self.blocks.key(id).unwrap_or_else(|| {
            self.errors.push(PackError::invalid(
                self.source,
                format!("unknown block `{id}`"),
            ));
            BlockId::AIR
        })
    }

    fn blocks(&mut self, ids: &[Identifier]) -> Vec<BlockId> {
        ids.iter().map(|id| self.block(id)).collect()
    }
}

impl WorldgenContent {
    pub fn biomes(&self) -> &[Biome] {
        &self.biomes
    }

    /// `None` if the dimension does not exist or has no biome.
    pub fn load(
        packs: &PackSet,
        blocks: &Registry<BlockId, Block>,
        dimension: &Identifier,
        errors: &mut Vec<PackError>,
    ) -> Option<Self> {
        let dimensions = packs.layered::<DimensionDef>("dimensions", errors);
        let Some(Sourced { value, source }) = dimensions.get(dimension) else {
            tracing::error!(%dimension, "dimension not found in packs");
            return None;
        };
        let features_by_id = load_features(packs, blocks, errors);
        let index_of = |id: &Identifier| features_by_id.keys().position(|k| k == id);
        let features: Vec<Feature> = features_by_id.values().cloned().collect();
        let terrain = resolve_terrain(&value.terrain, blocks, errors, source, &index_of);
        let biomes = load_biomes(packs, blocks, errors, &index_of);
        if biomes.is_empty() {
            errors.push(PackError::invalid(source, "no biome defined"));
            return None;
        }
        Some(Self {
            terrain,
            biomes,
            features,
        })
    }
}

fn resolve_terrain(
    def: &TerrainDef,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
    source: &Path,
    index_of: &dyn Fn(&Identifier) -> Option<usize>,
) -> Terrain {
    let ores = def
        .ores
        .iter()
        .filter_map(|id| {
            index_of(id).or_else(|| {
                errors.push(PackError::invalid(
                    source,
                    format!("unknown feature `{id}`"),
                ));
                None
            })
        })
        .collect();
    let mut r = Resolver {
        blocks,
        errors,
        source,
    };
    Terrain {
        sea_level: def.sea_level,
        deep_level: def.deep_level,
        stone: r.block(&def.stone),
        deep_stone: r.block(&def.deep_stone),
        bedrock: r.block(&def.bedrock),
        water: r.block(&def.water),
        ice: r.block(&def.ice),
        biome_scale: def.biome_scale.clamp(0.25, 16.0),
        caves: def.caves,
        ores,
    }
}

fn load_features(
    packs: &PackSet,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, Feature> {
    let defs = packs.layered::<FeatureDef>("features", errors);
    defs.into_iter()
        .map(|(id, Sourced { value, source })| {
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
                    on,
                } => Feature::Tree {
                    log: r.block(&log),
                    leaves: r.block(&leaves),
                    height: bounded(height, MAX_TREE_HEIGHT),
                    crown: match crown {
                        Crown::Round { radius } => Crown::Round {
                            radius: radius.min(MAX_CROWN),
                        },
                        Crown::Cone { radius } => Crown::Cone {
                            radius: radius.min(MAX_CROWN),
                        },
                    },
                    on: r.blocks(&on),
                },
                FeatureDef::Pillar { block, height, on } => Feature::Pillar {
                    block: r.block(&block),
                    height: bounded(height, MAX_TREE_HEIGHT),
                    on: r.blocks(&on),
                },
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
            };
            (id, feature)
        })
        .collect()
}

fn load_biomes(
    packs: &PackSet,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
    index_of: &dyn Fn(&Identifier) -> Option<usize>,
) -> Vec<Biome> {
    let defs = packs.layered::<BiomeDef>("biomes", errors);
    defs.into_iter()
        .map(|(id, Sourced { value, source })| {
            let features = value
                .features
                .iter()
                .filter_map(|placement| match index_of(&placement.feature) {
                    Some(index) => Some((index, placement.per_column.clamp(0.0, 64.0))),
                    None => {
                        let message = format!("unknown feature `{}`", placement.feature);
                        errors.push(PackError::invalid(&source, message));
                        None
                    }
                })
                .collect();
            let mut r = Resolver {
                blocks,
                errors,
                source: &source,
            };
            Biome {
                id,
                climate: value.climate,
                surface: r.block(&value.surface),
                subsurface: r.block(&value.subsurface),
                subsurface_depth: value.subsurface_depth.min(16),
                underwater: r.block(&value.underwater),
                peaks: sorted_peaks(&value.peaks, &mut r),
                frozen: value.frozen,
                spawn: value.spawn,
                features,
            }
        })
        .collect()
}

fn sorted_peaks(peaks: &[(i32, Identifier)], r: &mut Resolver<'_>) -> Vec<(i32, BlockId)> {
    let mut resolved: Vec<_> = peaks.iter().map(|(y, id)| (*y, r.block(id))).collect();
    resolved.sort_by_key(|(y, _)| *y);
    resolved
}

/// Features are written from the 3×3 neighbouring columns: anything wider than 16 blocks
/// from its root would be cut, so sizes are bounded.
const MAX_CROWN: u32 = 8;
const MAX_TREE_HEIGHT: u32 = 32;

fn bounded((a, b): (u32, u32), max: u32) -> (u32, u32) {
    (a.min(b).min(max), a.max(b).min(max))
}
