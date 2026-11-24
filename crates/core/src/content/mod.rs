//! Game content loaded from packs: blocks, items, textures, translations and cinematics.

mod attached;
mod block;
mod block_load;
mod container;
mod cracks;
mod describe;
mod entity;
mod facing;
mod fire;
mod fluid;
mod hit_box;
mod item;
mod keys;
mod liquid;
mod loot;
mod particle;
mod placement;
mod png_file;
mod registry;
mod tags;
mod textures;
mod tint;
mod tool;
mod turned;
mod worked;

pub use crate::cooking::FurnaceDef;
pub use attached::{AttachedSides, Attachment};
pub use block::{
    Axis, Block, BlockDef, BlockTextures, Face, LitDef, Orientation, Placement, RenderKind,
};
pub use container::{ContainerDef, StationDef};
pub use cracks::{CracksDef, MAX_CRACK_STAGES};
pub use entity::{DroppedItemDef, EntityDef, EntityType, ModelBox};
pub use facing::{FRONTS, Facing};
pub use fire::{FireDef, Flammability, MAX_FIRE_AGE};
pub use fluid::{
    Fluid, FluidDef, FluidLevel, FluidState, Hardening, HardeningDef, MAX_SLOPE_SEARCH, MAX_SPREAD,
};
pub use hit_box::{HitBox, Support};
pub use item::{EquipSlot, Item, ItemAction, ItemActionDef, ItemDef, ItemKind};
pub use liquid::{LiquidMotion, LiquidView, SurfaceAnimation, surface_sixteenths};
pub use loot::{
    EntryDef, LootCondition, LootContext, LootDrop, LootEntry, LootPool, LootTable, LootTableDef,
    PoolDef,
};
pub use particle::{
    BlockParticles, BreakingDef, ImpactDef, MAX_BURST, Particle, ParticleDef, ParticleRole,
    ParticleTriggersDef, Particles,
};
pub use png_file::{ACCEPTED_SIZES, missing_texture};
pub use registry::{
    BlockId, EntityTypeId, ItemId, ParticleId, RecipeId, Registry, RegistryId, SoundId, TextureId,
};
pub use tags::{ItemTags, TagDef};
pub use textures::{MAX_LAYERS, TextureLayout, TextureLibrary, missing_id, position_hash};
pub use tint::{BiomeTints, BlockTint, Tint, TintFaces, TintKind, UNTINTED_ALPHA};
pub use tool::{BlockTool, TICKS_PER_SECOND, ToolDef, crack_progress, crack_stage};
pub use worked::{Worked, WorkedDef};

use crate::audio::{self, Sounds};
use crate::cinematic::{self, Cinematic};
use crate::cooking::{self, Cooking};
use crate::crafting::{self, Benches, Recipes};
use crate::dimension::DimensionId;
use crate::gear::Gear;
use crate::id::Identifier;
use crate::lang::Translations;
use crate::pack::{PackError, PackSet};
use crate::physics::LiquidCell;
use crate::sky::{self, DayCycle, Sky};
use crate::storage::{Storage, load_storage};
use crate::upgrade::{Upgrades, load_upgrades};
use crate::worn::{Worn, bound_worn_effects, load_worn};
use block_load::{BlockRefs, Presented, load_blocks};
use std::collections::BTreeMap;

/// Everything the game knows about, built once from the active packs.
#[derive(Debug)]
pub struct Content {
    pub blocks: Registry<BlockId, Block>,
    pub items: Registry<ItemId, Item>,
    pub entities: Registry<EntityTypeId, EntityType>,
    /// Liquids by source block.
    pub fluids: BTreeMap<BlockId, Fluid>,
    pub textures: TextureLibrary,
    pub translations: Translations,
    pub cinematics: BTreeMap<Identifier, Cinematic>,
    /// Day cycles and skies by dimension.
    pub days: BTreeMap<Identifier, DayCycle>,
    pub skies: BTreeMap<Identifier, Sky>,
    pub particles: Registry<ParticleId, Particle>,
    pub sounds: Sounds,
    /// What broken blocks give, by table identifier.
    pub loot_tables: BTreeMap<Identifier, LootTable>,
    /// Texture of each crack stage over a block being broken (`cracks.ron`).
    pub crack_stages: Vec<TextureId>,
    /// Named groups of items (« any planks »).
    pub tags: ItemTags,
    pub recipes: Recipes,
    /// Furnaces (fuels, cooking recipes, tiers) and the stonecutter.
    pub cooking: Cooking,
    /// Tool tiers and armour weights.
    pub gear: Gear,
    /// Upgrades put in items at the anvil, and the repair rules.
    pub upgrades: Upgrades,
    /// Sets and the movement abilities equipment gives.
    pub worn: Worn,
    /// Chest and barrel tiers, compression recipes.
    pub storage: Storage,
    /// Backpacks and pouches.
    pub bags: crate::bag::Bags,
    /// Recipes of the benches (jeweller…).
    pub benches: Benches,
    /// Item tooltips by language, see `item_tooltip`.
    pub tooltips: describe::TooltipCache,
}

/// The benches blocks open.
fn bench_stations(blocks: &Registry<BlockId, Block>) -> std::collections::BTreeSet<Identifier> {
    (blocks.iter())
        .filter_map(|(_, _, block)| match &block.station {
            Some(StationDef::Bench(bench)) => Some(bench.clone()),
            _ => None,
        })
        .collect()
}

/// Identifier of the empty block, always `BlockId::AIR`.
pub fn air_id() -> Identifier {
    Identifier::builtin("strata", "air")
}

impl Content {
    /// `packs` define content; `resource_packs` may only restyle it (textures, palette,
    /// translations). Errors are collected, never fatal: broken entries are skipped or
    /// shown with the "missing" texture.
    pub fn load(packs: &PackSet, resource_packs: &PackSet) -> (Self, Vec<PackError>) {
        let mut errors = Vec::new();
        let presentation = PackSet::joined(packs, resource_packs);
        let textures = TextureLibrary::load(&presentation, &mut errors);
        let particles = particle::load_particles(&presentation, &textures, &mut errors);
        let sounds = audio::load_sounds(&presentation, &mut errors);
        let presented = Presented {
            textures: &textures,
            particles: &particles,
            sounds: &sounds,
        };
        let mut refs = BlockRefs::default();
        let (mut blocks, fluids) = load_blocks(packs, &presented, &mut refs, &mut errors);
        let entities = entity::load_entities(packs, &textures, &mut errors);
        let gear = Gear::load(packs, &mut errors);
        let known = item::Known {
            blocks: &blocks,
            entities: &entities,
            gear: &gear,
        };
        let mut items = item::load_items(packs, &known, &textures, &mut errors);
        let loot_tables = loot::load_loot_tables(packs, &items, &mut errors);
        let tags = tags::load_tags(packs, &items, &mut errors);
        let names = crafting::Names {
            items: &items,
            tags: &tags,
        };
        let recipes = Recipes::new(crafting::load_recipes(packs, &names, &mut errors));
        let benches = crafting::load_benches(packs, &names, &bench_stations(&blocks), &mut errors);
        let cooking = cooking::load_cooking(packs, &names, &blocks, &mut errors);
        let upgrades = load_upgrades(packs, &names, &gear.slot_families, &mut errors);
        let storage_context = (&cooking, &gear.slot_families);
        let storage = load_storage(packs, &items, &mut blocks, storage_context, &mut errors);
        let bags = crate::bag::load_bags(&items, &blocks, &tags, &storage, &mut errors);
        bound_worn_effects(&mut items, &upgrades.stats, &mut errors);
        let worn = load_worn(packs, &items, &upgrades.stats, &mut errors);
        loot::check_loot_names(&refs.loot, &loot_tables, &mut errors);
        worked::resolve_worked(&mut blocks, refs.worked, &items, &mut errors);
        let translations = Translations::load(&presentation, &mut errors);
        let crack_stages = cracks::load_cracks(&presentation, &textures, &mut errors);
        let cinematics = cinematic::load(packs, &mut errors);
        let days = sky::load_days(packs, &mut errors);
        let skies = sky::load_skies(&presentation, &textures, &mut errors);
        let mut content = Self {
            blocks,
            items,
            entities,
            fluids,
            textures,
            translations,
            cinematics,
            days,
            skies,
            particles: particles.registry,
            sounds,
            loot_tables,
            crack_stages,
            tags,
            recipes,
            cooking,
            gear,
            upgrades,
            worn,
            storage,
            bags,
            benches,
            tooltips: Default::default(),
        };
        content.cache_tooltips();
        (content, errors)
    }

    /// Translated name of an item (a block's item has the block's name).
    pub fn item_name(&self, item: ItemId, language: &str) -> String {
        let Some(id) = self.items.identifier(item) else {
            return String::new();
        };
        let kind = match self.items.get(item).map(|i| i.kind) {
            Some(ItemKind::Block(_)) => "block",
            _ => "item",
        };
        self.translations.text(language, &id.translation_key(kind))
    }

    /// What breaking `block` rolls: its table, or the block itself.
    pub fn block_loot(&self, block: BlockId) -> Option<&LootTable> {
        let name = self.blocks.get(block)?.loot.as_ref()?;
        self.loot_tables.get(name)
    }

    /// Tool kinds named by blocks, items and loot tables (each has a translated name).
    pub fn tool_kinds(&self) -> Vec<&Identifier> {
        let blocks = self.blocks.iter().filter_map(|(_, _, b)| b.tool.as_ref());
        let items = self.items.iter().filter_map(|(_, _, i)| i.tool.as_ref());
        let mut kinds: Vec<&Identifier> = blocks
            .map(|tool| &tool.kind)
            .chain(items.flat_map(|tool| &tool.kinds))
            .chain(self.loot_tables.values().flat_map(LootTable::tool_kinds))
            .collect();
        kinds.sort();
        kinds.dedup();
        kinds
    }

    /// The day cycle of `dimension` (the default one if its pack defines none).
    pub fn day_cycle(&self, dimension: &DimensionId) -> DayCycle {
        self.days.get(&dimension.0).copied().unwrap_or_default()
    }

    pub fn sky(&self, dimension: &DimensionId) -> Sky {
        self.skies
            .get(&dimension.0)
            .cloned()
            .unwrap_or_else(Sky::fallback)
    }

    /// Whether each block (by `BlockId`) stops movement, for fast collision lookups.
    pub fn solidity(&self) -> Vec<bool> {
        self.blocks
            .iter()
            .map(|(_, _, block)| block.solid)
            .collect()
    }

    /// The liquid of each block (by `BlockId`), for swimming.
    pub fn liquids(&self) -> Vec<Option<LiquidCell>> {
        self.blocks
            .iter()
            .map(|(_, _, block)| {
                let state = block.fluid?;
                let fluid = self.fluids.get(&state.source)?;
                Some(LiquidCell {
                    motion: fluid.motion,
                    surface: f64::from(surface_sixteenths(state.level)) / 16.0,
                })
            })
            .collect()
    }

    /// Identifier naming `block` in the interface: lying logs are called like upright ones.
    pub fn block_name(&self, block: BlockId) -> Option<&Identifier> {
        let family = self.blocks.get(block)?.family(block);
        self.blocks.identifier(family)
    }
}
