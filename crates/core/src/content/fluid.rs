//! Liquids: a source block described in the pack, and flowing variants generated from it.

use super::block::Block;
use super::liquid::{LiquidMotion, LiquidView};
use super::registry::{BlockId, Registry};
use crate::id::Identifier;
use crate::pack::PackError;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Farthest a liquid flows on flat ground: flowing levels are 1 to 7, as in Minecraft.
pub const MAX_SPREAD: u8 = 7;
pub const MAX_SLOPE_SEARCH: u8 = 8;

/// The `fluid` field of a block: makes it the source of a flowing liquid.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FluidDef {
    /// Ticks between two steps of the flow (20 ticks per second).
    #[serde(default = "default_tick_rate")]
    pub tick_rate: u32,
    /// How far it flows on flat ground, 1 to 7 blocks.
    #[serde(default = "default_spread")]
    pub spread: u8,
    /// How far it looks for a way down before choosing where to flow.
    #[serde(default = "default_slope_search")]
    pub slope_search: u8,
    /// A new source forms between two sources standing on something (water).
    #[serde(default)]
    pub renewable: bool,
    #[serde(default)]
    pub hardens: Option<HardeningDef>,
    /// Fire cannot stay next to it (water).
    #[serde(default)]
    pub puts_out_fire: bool,
    #[serde(default)]
    pub motion: LiquidMotion,
    #[serde(default)]
    pub view: LiquidView,
}

/// What this liquid turns into when it touches another one (lava meeting water).
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HardeningDef {
    /// Source block of the other liquid.
    pub touching: Identifier,
    /// What a source becomes.
    pub source: Identifier,
    /// What a flowing or falling cell becomes.
    pub flowing: Identifier,
}

fn default_tick_rate() -> u32 {
    5
}

fn default_spread() -> u8 {
    MAX_SPREAD
}

fn default_slope_search() -> u8 {
    4
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FluidLevel {
    Source,
    /// Distance to the nearest source, 1 (fullest) to the liquid's spread.
    Flowing(u8),
    /// Pouring down from above.
    Falling,
}

/// Which liquid a block is, and how full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FluidState {
    pub source: BlockId,
    pub level: FluidLevel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Hardening {
    pub touching: BlockId,
    pub source: BlockId,
    pub flowing: BlockId,
}

/// A loaded liquid: every block it can take, and how it moves.
#[derive(Debug, Clone, PartialEq)]
pub struct Fluid {
    pub source: BlockId,
    /// Indexed by level − 1.
    pub flowing: Vec<BlockId>,
    pub falling: BlockId,
    pub tick_rate: u32,
    pub spread: u8,
    pub slope_search: u8,
    pub renewable: bool,
    pub hardens: Option<Hardening>,
    pub puts_out_fire: bool,
    pub motion: LiquidMotion,
    pub view: LiquidView,
}

impl Fluid {
    pub fn block(&self, level: FluidLevel) -> Option<BlockId> {
        match level {
            FluidLevel::Source => Some(self.source),
            FluidLevel::Falling => Some(self.falling),
            FluidLevel::Flowing(level) => {
                let index = usize::from(level).checked_sub(1)?;
                self.flowing.get(index).copied()
            }
        }
    }
}

/// A liquid whose hardening still names blocks by identifier.
pub(super) struct PendingFluid {
    fluid: Fluid,
    hardens: Option<HardeningDef>,
    source: PathBuf,
}

/// Registers the source and its variants `<name>__1` … `<name>__<spread>` and
/// `<name>__falling`, which have no item and share the source's name.
pub(super) fn insert_fluid(
    blocks: &mut Registry<BlockId, Block>,
    id: &Identifier,
    source: Block,
    def: FluidDef,
    path: &Path,
    errors: &mut Vec<PackError>,
) -> Option<PendingFluid> {
    let spread = def.spread.clamp(1, MAX_SPREAD);
    if spread != def.spread || def.slope_search > MAX_SLOPE_SEARCH || def.tick_rate == 0 {
        errors.push(PackError::invalid(
            path,
            "fluid: spread must be 1-7, slope_search 0-8 and tick_rate at least 1",
        ));
    }
    let levels = (1..=spread)
        .map(FluidLevel::Flowing)
        .chain([FluidLevel::Falling]);
    let names: Option<Vec<Identifier>> =
        levels.clone().map(|level| variant_id(id, level)).collect();
    let names = names.filter(|names| {
        names.iter().all(|name| blocks.key(name).is_none())
            && blocks.len() + names.len() < usize::from(u16::MAX)
    })?;
    let source_state = FluidState {
        source: BlockId(blocks.len() as u16),
        level: FluidLevel::Source,
    };
    let variant = |level| Block {
        fluid: Some(FluidState {
            level,
            ..source_state
        }),
        has_item: false,
        category: None,
        ..source.clone()
    };
    let source_id = blocks.insert(
        id.clone(),
        Block {
            fluid: Some(source_state),
            ..source.clone()
        },
    )?;
    let mut flowing = Vec::new();
    let mut falling = source_id;
    for (level, name) in levels.zip(names) {
        let key = blocks.insert(name, variant(level))?;
        match level {
            FluidLevel::Falling => falling = key,
            _ => flowing.push(key),
        }
    }
    Some(PendingFluid {
        fluid: Fluid {
            source: source_id,
            flowing,
            falling,
            tick_rate: def.tick_rate.max(1),
            spread,
            slope_search: def.slope_search.min(MAX_SLOPE_SEARCH),
            renewable: def.renewable,
            hardens: None,
            puts_out_fire: def.puts_out_fire,
            motion: def.motion.clamped(),
            view: def.view,
        },
        hardens: def.hardens,
        source: path.to_owned(),
    })
}

fn variant_id(id: &Identifier, level: FluidLevel) -> Option<Identifier> {
    let suffix = match level {
        FluidLevel::Source => return Some(id.clone()),
        FluidLevel::Falling => "falling".to_owned(),
        FluidLevel::Flowing(level) => level.to_string(),
    };
    Identifier::new(id.namespace(), &format!("{}__{suffix}", id.name())).ok()
}

/// Names in `hardens` are resolved once every block is known.
pub(super) fn resolve(
    blocks: &Registry<BlockId, Block>,
    pending: Vec<PendingFluid>,
    errors: &mut Vec<PackError>,
) -> BTreeMap<BlockId, Fluid> {
    let mut fluids = BTreeMap::new();
    for PendingFluid {
        mut fluid,
        hardens,
        source,
    } in pending
    {
        if let Some(def) = hardens {
            fluid.hardens = hardening(blocks, &def);
            if fluid.hardens.is_none() {
                errors.push(PackError::invalid(
                    &source,
                    "fluid.hardens: `touching` must be a fluid source, `source` and `flowing` known blocks",
                ));
            }
        }
        fluids.insert(fluid.source, fluid);
    }
    fluids
}

fn hardening(blocks: &Registry<BlockId, Block>, def: &HardeningDef) -> Option<Hardening> {
    let touching = blocks.key(&def.touching)?;
    let is_source = blocks
        .get(touching)?
        .fluid
        .is_some_and(|state| state.level == FluidLevel::Source);
    is_source.then_some(Hardening {
        touching,
        source: blocks.key(&def.source)?,
        flowing: blocks.key(&def.flowing)?,
    })
}
