mod features;

pub(crate) use features::bounded;
pub use features::{Feature, MAX_CROWN, MAX_PATCH_RADIUS, Plants};

use crate::defs::{
    BandsDef, BiomeColors, BiomeDef, CavesDef, Climate, DimensionDef, Relief, TerrainDef,
};
use features::load_features;
use std::path::Path;
use strata_core::chunk::BiomeId;
use strata_core::content::{BiomeTints, Block, BlockId, Registry, Tint};
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
    /// `None` = no caves.
    pub caves: Option<Caves>,
    /// Indices into `WorldgenContent::features`.
    pub ores: Vec<usize>,
    /// Indices of `Feature::Deposit`s.
    pub deposits: Vec<usize>,
    pub relief: Relief,
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
    pub ravine_depth: (i32, i32),
    pub aquifers: f64,
    pub aquifer_cell: i32,
    pub aquifer_top: i32,
    pub aquifer_depth: i32,
    pub lava_level: i32,
    pub water: BlockId,
    pub lava: BlockId,
}

#[derive(Debug, Clone)]
pub struct Biome {
    pub id: Identifier,
    /// Position in `WorldgenContent::biomes`, as sent to clients.
    pub index: BiomeId,
    pub climate: Climate,
    pub surface: BlockId,
    pub subsurface: BlockId,
    pub subsurface_depth: u32,
    pub underwater: BlockId,
    /// Highest ground, above the sea level, still covered with `underwater`.
    pub shore_height: i32,
    /// Sorted by height.
    pub peaks: Vec<(i32, BlockId)>,
    /// Surface of steep slopes; `None` = the dimension's stone.
    pub steep: Option<BlockId>,
    pub frozen: bool,
    pub min_height: Option<i32>,
    pub max_height: Option<i32>,
    pub river: bool,
    pub bands: Option<Bands>,
    pub map_color: [u8; 3],
    pub spawn: bool,
    /// `(feature index, attempts per column)`.
    pub features: Vec<(usize, f64)>,
    pub tints: BiomeTints,
}

/// Layers by height: `blocks[y % len]`, shifted by a little noise.
#[derive(Debug, Clone)]
pub struct Bands {
    pub blocks: Vec<BlockId>,
    pub depth: i32,
    pub waviness: f64,
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
    let caves = resolve_caves(def.caves.as_ref(), def.sea_level, water, &mut r);
    Terrain {
        sea_level: def.sea_level,
        deep_level: def.deep_level,
        stone: r.block(&def.stone),
        deep_stone: r.block(&def.deep_stone),
        bedrock: r.block(&def.bedrock),
        water,
        ice: r.block(&def.ice),
        biome_scale: def.biome_scale.clamp(0.25, 16.0),
        caves,
        ores,
        deposits,
        relief: bounded_relief(&def.relief),
    }
}

fn load_biomes(
    packs: &PackSet,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
    index_of: &dyn Fn(&Identifier) -> Option<usize>,
) -> Vec<Biome> {
    let defs = packs.layered::<BiomeDef>("biomes", errors);
    defs.into_iter()
        .enumerate()
        .filter_map(|(index, (id, Sourced { value, source }))| {
            let Ok(index) = u16::try_from(index) else {
                let message = "too many biomes (65 536 at most): this one is ignored";
                errors.push(PackError::invalid(&source, message));
                return None;
            };
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
            Some(Biome {
                id,
                index: BiomeId(index),
                climate: value.climate,
                surface: r.block(&value.surface),
                subsurface: r.block(&value.subsurface),
                subsurface_depth: value.subsurface_depth.min(16),
                underwater: r.block(&value.underwater),
                shore_height: value.shore_height.clamp(-8, 8),
                peaks: sorted_peaks(&value.peaks, &mut r),
                steep: value.steep.as_ref().map(|id| r.block(id)),
                frozen: value.frozen,
                min_height: value.min_height,
                max_height: value.max_height,
                river: value.river,
                bands: value.bands.as_ref().and_then(|def| bands(def, &mut r)),
                map_color: {
                    let [r, g, b, _] = value.map_color.0.0;
                    [r, g, b]
                },
                spawn: value.spawn,
                features,
                tints: biome_tints(&value.colors),
            })
        })
        .collect()
}

fn biome_tints(colors: &BiomeColors) -> BiomeTints {
    BiomeTints {
        kinds: [colors.grass, colors.foliage, colors.water].map(Tint::from_factors),
        fog: Tint::from_factors(colors.fog),
    }
}

fn bands(def: &BandsDef, r: &mut Resolver<'_>) -> Option<Bands> {
    let blocks = r.blocks(&def.blocks);
    (!blocks.is_empty()).then(|| Bands {
        blocks,
        depth: def.depth.min(64) as i32,
        waviness: def.waviness.clamp(0.0, 16.0),
    })
}

/// Pack data must never make generation absurdly slow (huge overhangs scan many blocks).
fn bounded_relief(relief: &Relief) -> Relief {
    let mut relief = relief.clone();
    relief.overhang_blocks = relief.overhang_blocks.clamp(0.0, 64.0);
    relief.terrace_step = relief.terrace_step.clamp(1.0, 64.0);
    relief.plateau_lift = relief.plateau_lift.clamp(-64.0, 128.0);
    relief.overhang_scale = relief.overhang_scale.clamp(4.0, 4096.0);
    relief.weirdness_scale = relief.weirdness_scale.clamp(16.0, 65536.0);
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

/// Bounded so that pack data can't flood the world or make generation absurdly slow.
fn resolve_caves(
    def: Option<&CavesDef>,
    sea_level: i32,
    water: BlockId,
    r: &mut Resolver<'_>,
) -> Option<Caves> {
    let def = def.filter(|def| def.enabled)?;
    let density = def.density.clamp(0.0, 4.0);
    let width = |value: f64| (value * density).clamp(0.0, 0.5);
    // Density 0 lifts the threshold out of the noise's range: no cavern at all.
    let cheese = 1.0 - (1.0 - def.cheese.clamp(-1.0, 1.0)) * density;
    let (shallow, deep) = def.ravine_depth;
    let shallow = shallow.clamp(1, 256);
    Some(Caves {
        spaghetti: width(def.spaghetti),
        noodles: width(def.noodles),
        noodle_top: def.noodle_top.clamp(0, WORLD_HEIGHT),
        cheese: cheese.clamp(MIN_CHEESE, 1.0),
        cavern_top: def.cavern_top.clamp(1, WORLD_HEIGHT),
        ravines: (def.ravines * density).clamp(0.0, 0.2),
        ravine_depth: (shallow, deep.clamp(shallow, 256)),
        aquifers: def.aquifers.clamp(0.0, 1.0),
        aquifer_cell: def.aquifer_cell.clamp(16, 1024),
        aquifer_top: def.aquifer_top.min(sea_level - 4).max(0),
        aquifer_depth: def.aquifer_depth.clamp(0, 256),
        lava_level: def.lava_level.clamp(-1, 64),
        water,
        lava: r.block(&def.lava),
    })
}

/// Lower thresholds would hollow out most of the underground.
const MIN_CHEESE: f64 = 0.15;

fn sorted_peaks(peaks: &[(i32, Identifier)], r: &mut Resolver<'_>) -> Vec<(i32, BlockId)> {
    let mut resolved: Vec<_> = peaks.iter().map(|(y, id)| (*y, r.block(id))).collect();
    resolved.sort_by_key(|(y, _)| *y);
    resolved
}
