mod biomes;
mod features;
mod world_types;

pub use biomes::{Bands, Biome};
pub(crate) use features::bounded;
pub use features::{Feature, MAX_CROWN, MAX_PATCH_RADIUS, Plants};
pub use world_types::WorldType;

use crate::defs::{CavesDef, ClimateSizes, DimensionDef, Relief, TerrainDef};
use biomes::{check_heat, load_biomes};
use features::load_features;
use std::collections::HashMap;
use std::path::Path;
use strata_core::content::{Block, BlockId, Registry};
use strata_core::coords::WORLD_HEIGHT;
use strata_core::id::Identifier;
use strata_core::pack::{PackError, PackSet, Sourced};

/// Generation content with block identifiers resolved to numeric ids. Only built by
/// `load`, which guarantees at least one biome.
#[derive(Debug, Clone)]
pub struct WorldgenContent {
    pub terrain: Terrain,
    biomes: Vec<Biome>,
    pub features: Vec<Feature>,
    /// Sorted by their `order`.
    world_types: Vec<WorldType>,
    /// Every block, for the layers of flat worlds.
    pub(crate) blocks: HashMap<Identifier, BlockId>,
    /// Leaves of every tree: a trunk grows through the crown of a neighbouring tree.
    pub(crate) tree_leaves: Vec<BlockId>,
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
    pub climate_sizes: ClimateSizes,
    /// Sorted temperatures between heat bands.
    pub heat_bands: Vec<f64>,
    /// `None` = no caves.
    pub caves: Option<Caves>,
    /// What `caves` was resolved from (with its lava), to scale it again.
    pub(crate) cave_def: Option<(CavesDef, BlockId)>,
    /// Indices into `WorldgenContent::features`.
    pub ores: Vec<usize>,
    /// Indices of `Feature::Deposit`s.
    pub deposits: Vec<usize>,
    pub relief: Relief,
    /// Index of the only biome, for world types that force one.
    pub forced_biome: Option<usize>,
    /// Layers of flat worlds, instead of the relief.
    pub flat: Option<Flat>,
}

/// `(block, thickness)` from the bottom of the world up.
#[derive(Debug, Clone)]
pub struct Flat {
    pub layers: Vec<(BlockId, i32)>,
    /// Trees and plants.
    pub features: bool,
}

impl Flat {
    /// Height of the highest block of the layers.
    pub fn top(&self) -> i32 {
        self.layers
            .iter()
            .map(|(_, thickness)| thickness)
            .sum::<i32>()
            - 1
    }
}

/// Cave settings with blocks resolved, bounded, and `density` already applied.
#[derive(Debug, Clone)]
pub struct Caves {
    pub spaghetti: f64,
    pub noodles: f64,
    pub noodle_top: i32,
    pub cheese: f64,
    pub cavern_top: i32,
    pub ravines: f64,
    pub ravine_spacing: i32,
    pub ravine_depth: (i32, i32),
    pub aquifers: f64,
    pub aquifer_cell: i32,
    pub aquifer_top: i32,
    pub aquifer_depth: i32,
    pub lava_level: i32,
    pub water: BlockId,
    pub lava: BlockId,
}

/// Looks up identifiers, recording an error (with the file) for each unknown one.
pub(crate) struct Resolver<'a> {
    pub(crate) blocks: &'a Registry<BlockId, Block>,
    pub(crate) errors: &'a mut Vec<PackError>,
    pub(crate) source: &'a Path,
}

impl Resolver<'_> {
    pub(crate) fn block(&mut self, id: &Identifier) -> BlockId {
        self.blocks.key(id).unwrap_or_else(|| {
            self.errors.push(PackError::invalid(
                self.source,
                format!("unknown block `{id}`"),
            ));
            BlockId::AIR
        })
    }

    pub(crate) fn blocks(&mut self, ids: &[Identifier]) -> Vec<BlockId> {
        ids.iter().map(|id| self.block(id)).collect()
    }
}

impl WorldgenContent {
    pub fn biomes(&self) -> &[Biome] {
        &self.biomes
    }

    pub fn world_types(&self) -> &[WorldType] {
        &self.world_types
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
        check_heat(&terrain.heat_bands, &biomes, source, errors);
        let blocks_by_id = blocks
            .iter()
            .map(|(key, id, _)| (id.clone(), key))
            .collect();
        let world_types = world_types::load(packs, &blocks_by_id, &biomes, errors);
        Some(Self {
            terrain,
            biomes,
            world_types,
            blocks: blocks_by_id,
            tree_leaves: tree_leaves(&features),
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
    let mut indices = |ids: &[Identifier]| -> Vec<usize> {
        ids.iter()
            .filter_map(|id| {
                index_of(id).or_else(|| {
                    let message = format!("unknown feature `{id}`");
                    errors.push(PackError::invalid(source, message));
                    None
                })
            })
            .collect()
    };
    let ores = indices(&def.ores);
    let deposits = indices(&def.deposits);
    let mut r = Resolver {
        blocks,
        errors,
        source,
    };
    let water = r.block(&def.water);
    let cave_def = def
        .caves
        .clone()
        .filter(|caves| caves.enabled)
        .map(|caves| {
            warn_old_ravine_width(&caves, source);
            let lava = r.block(&caves.lava);
            (caves, lava)
        });
    let mut terrain = Terrain {
        sea_level: def.sea_level,
        deep_level: def.deep_level,
        stone: r.block(&def.stone),
        deep_stone: r.block(&def.deep_stone),
        bedrock: r.block(&def.bedrock),
        water,
        ice: r.block(&def.ice),
        biome_scale: def.biome_scale.clamp(0.25, 16.0),
        climate_sizes: def.climate_sizes.bounded(),
        heat_bands: sorted_bands(&def.heat_bands),
        caves: None,
        cave_def,
        ores,
        deposits,
        relief: bounded_relief(&def.relief),
        forced_biome: None,
        flat: None,
    };
    terrain.scale_caves(1.0);
    terrain
}

impl Terrain {
    /// Heat band of a temperature: 0 below the first threshold.
    pub fn heat_band(&self, temperature: f64) -> u8 {
        let warmer = self.heat_bands.partition_point(|&t| t <= temperature);
        u8::try_from(warmer).unwrap_or(u8::MAX)
    }

    /// Resolves the caves again with their density multiplied by `factor`; only a factor
    /// of 0 (the player's slider) removes them entirely, a pack density of 0 keeps its
    /// aquifers and lava as before.
    pub(crate) fn scale_caves(&mut self, factor: f64) {
        self.caves = self
            .cave_def
            .as_ref()
            .filter(|_| factor > 0.0)
            .map(|(def, lava)| {
                let density = def.density.clamp(0.0, 4.0) * factor;
                bounded_caves(def, density, self.sea_level, self.water, *lava)
            });
    }
}

fn tree_leaves(features: &[Feature]) -> Vec<BlockId> {
    features
        .iter()
        .filter_map(|feature| match feature {
            Feature::Tree(tree) => Some(tree.leaves),
            _ => None,
        })
        .collect()
}

fn sorted_bands(thresholds: &[f64]) -> Vec<f64> {
    let mut bands: Vec<f64> = thresholds
        .iter()
        .copied()
        .filter(|t| t.is_finite())
        .collect();
    bands.sort_by(f64::total_cmp);
    bands.truncate(usize::from(u8::MAX));
    bands
}

/// Pack data must never make generation absurdly slow (huge overhangs scan many blocks).
pub(crate) fn bounded_relief(relief: &Relief) -> Relief {
    let mut relief = relief.clone();
    relief.overhang_blocks = relief.overhang_blocks.clamp(0.0, 64.0);
    relief.terrace_step = relief.terrace_step.clamp(1.0, 64.0);
    relief.plateau_lift = relief.plateau_lift.clamp(-64.0, 128.0);
    relief.overhang_scale = relief.overhang_scale.clamp(4.0, 4096.0);
    relief.weirdness_scale = relief.weirdness_scale.clamp(16.0, 65536.0);
    relief.continent_scale = relief.continent_scale.clamp(0.05, 16.0);
    // Overhangs rise above the plain height: the highest block stays inside the world.
    let ceiling = f64::from(WORLD_HEIGHT - 1) - relief.overhang_blocks.ceil();
    relief.max_height = relief.max_height.clamp(1.0, ceiling);
    relief.river_scale = relief.river_scale.clamp(16.0, 65536.0);
    relief.steep_slope = relief.steep_slope.clamp(1, 64);
    if let Some(rivers) = &mut relief.rivers {
        rivers.width = rivers.width.clamp(0.0, 0.5);
        rivers.banks = rivers.banks.clamp(0.0, 0.5);
        rivers.depth = rivers.depth.clamp(0.0, 64.0);
        rivers.highest = rivers.highest.clamp(0.0, 400.0);
    }
    if let Some(wetlands) = &mut relief.wetlands {
        wetlands.level = wetlands.level.clamp(-16.0, 16.0);
        wetlands.bumps = wetlands.bumps.clamp(0.0, 16.0);
    }
    relief
}

/// `ravines` became a half-width in blocks in v0.2.1; older packs gave a noise width
/// (0.03), which now makes ravines invisible.
fn warn_old_ravine_width(caves: &CavesDef, source: &Path) {
    if caves.ravines > 0.0 && caves.ravines < 1.0 {
        tracing::warn!(
            ravines = caves.ravines,
            pack = %source.display(),
            "`ravines` is a half-width in blocks since v0.2.1 (3.5 by default): values below 1 hide ravines"
        );
    }
}

/// Bounded so that pack data can't flood the world or make generation absurdly slow.
fn bounded_caves(
    def: &CavesDef,
    density: f64,
    sea_level: i32,
    water: BlockId,
    lava: BlockId,
) -> Caves {
    let density = density.clamp(0.0, 16.0);
    // Density 0 lifts the threshold out of the noise's range: no cavern at all.
    let width = |value: f64| (value * density).clamp(0.0, 0.5);
    let cheese = 1.0 - (1.0 - def.cheese.clamp(-1.0, 1.0)) * density;
    let (shallow, deep) = def.ravine_depth;
    let shallow = shallow.clamp(1, 256);
    Caves {
        spaghetti: width(def.spaghetti),
        noodles: width(def.noodles),
        noodle_top: def.noodle_top.clamp(0, WORLD_HEIGHT),
        cheese: cheese.clamp(MIN_CHEESE, 1.0),
        cavern_top: def.cavern_top.clamp(1, WORLD_HEIGHT),
        ravines: (def.ravines * density).clamp(0.0, 16.0),
        ravine_spacing: def.ravine_spacing.clamp(64, 4096),
        ravine_depth: (shallow, deep.clamp(shallow, 256)),
        aquifers: def.aquifers.clamp(0.0, 1.0),
        aquifer_cell: def.aquifer_cell.clamp(16, 1024),
        aquifer_top: def.aquifer_top.min(sea_level - 4).max(0),
        aquifer_depth: def.aquifer_depth.clamp(0, 256),
        lava_level: def.lava_level.clamp(-1, 64),
        water,
        lava,
    }
}

/// Lower thresholds would hollow out most of the underground.
const MIN_CHEESE: f64 = 0.15;
