//! `heat_sources/<name>.ron`: blocks that warm the air around them (fire, lava, torches,
//! lit furnaces, the campfire), with their strength and reach. Of the sources around a
//! body, only the strongest counts (a hundred torches do not make a furnace).

use crate::content::{Block, BlockId, ItemKind, ItemTags, Registry};
use crate::content::{Item, ItemId};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;

/// Farthest reach a pack may give a source, in blocks (the search is a cube of it).
pub const MAX_REACH: f32 = 8.0;

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename = "HeatSource", deny_unknown_fields)]
pub struct HeatSourceDef {
    #[serde(default)]
    pub blocks: Vec<Identifier>,
    /// The blocks of the items of an item tag (`strata:furnaces`).
    #[serde(default)]
    pub tag: Option<Identifier>,
    /// Degrees right next to it, fading to nothing at `range` blocks.
    pub warmth: f32,
    pub range: f32,
    /// Only while lit (a furnace burning).
    #[serde(default)]
    pub lit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatSource {
    pub warmth: f32,
    pub range: f32,
}

impl HeatSource {
    /// Degrees it gives at `distance` blocks from its centre.
    pub fn at(&self, distance: f32) -> f32 {
        (self.warmth * (1.0 - distance / self.range)).max(0.0)
    }
}

/// Every source by `BlockId`, and how far the farthest reaches (whole blocks).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeatSources {
    pub by_block: Vec<Option<HeatSource>>,
    pub reach: u8,
}

impl HeatSources {
    pub fn get(&self, block: BlockId) -> Option<HeatSource> {
        *self.by_block.get(usize::from(block.0))?
    }
}

pub(crate) struct HeatKnown<'a> {
    pub blocks: &'a Registry<BlockId, Block>,
    pub items: &'a Registry<ItemId, Item>,
    pub tags: &'a ItemTags,
}

pub(crate) fn load_heat_sources(
    packs: &PackSet,
    known: &HeatKnown<'_>,
    errors: &mut Vec<PackError>,
) -> HeatSources {
    let mut sources = HeatSources {
        by_block: vec![None; known.blocks.len()],
        reach: 0,
    };
    for (_, Sourced { value, source }) in packs.layered::<HeatSourceDef>("heat_sources", errors) {
        let valid = |v: f32| v.is_finite() && v > 0.0;
        if !valid(value.warmth) || !valid(value.range) {
            errors.push(PackError::invalid(
                &source,
                "warmth and range must be above 0",
            ));
            continue;
        }
        let heat = HeatSource {
            warmth: value.warmth.min(100.0),
            range: value.range.min(MAX_REACH),
        };
        for family in families(&value, known, &source, errors) {
            sources.mark(family, value.lit, heat, known.blocks);
        }
        sources.reach = sources.reach.max(heat.range.ceil() as u8);
    }
    sources
}

/// The blocks a definition names, directly or through its tag.
fn families(
    def: &HeatSourceDef,
    known: &HeatKnown<'_>,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> Vec<BlockId> {
    let mut found = Vec::new();
    for id in &def.blocks {
        match known.blocks.key(id) {
            Some(block) => found.push(block),
            None => errors.push(PackError::invalid(source, format!("unknown block `{id}`"))),
        }
    }
    if let Some(tag) = &def.tag {
        let Some(items) = known.tags.get(tag) else {
            errors.push(PackError::invalid(source, format!("unknown tag `{tag}`")));
            return found;
        };
        found.extend(
            items
                .iter()
                .filter_map(|&item| match known.items.get(item)?.kind {
                    ItemKind::Block(block) => Some(block),
                    _ => None,
                }),
        );
    }
    found
}

impl HeatSources {
    /// Every block of `family` (turned, attached or lit variants) gives `heat`; only the
    /// lit ones when `lit`.
    fn mark(
        &mut self,
        family: BlockId,
        lit: bool,
        heat: HeatSource,
        blocks: &Registry<BlockId, Block>,
    ) {
        for (key, _, block) in blocks.iter() {
            let member = key == family || block.family(key) == family;
            let burning = !lit || block.facing.is_some_and(|f| f.lit);
            if member
                && burning
                && let Some(slot) = self.by_block.get_mut(usize::from(key.0))
            {
                *slot = Some(heat);
            }
        }
    }
}
