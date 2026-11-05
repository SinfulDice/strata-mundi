//! Game content loaded from packs: blocks, items, textures, translations and cinematics.

mod block;
mod entity;
mod fluid;
mod item;
mod liquid;
mod png_file;
mod registry;
mod textures;

pub use block::{
    Axis, Block, BlockDef, BlockTextures, Face, HitBox, Orientation, Placement, RenderKind,
};
pub use entity::{EntityDef, EntityType, ModelBox};
pub use fluid::{
    Fluid, FluidDef, FluidLevel, FluidState, Hardening, HardeningDef, MAX_SLOPE_SEARCH, MAX_SPREAD,
};
pub use item::{Item, ItemAction, ItemActionDef, ItemDef, ItemKind};
pub use liquid::{LiquidMotion, LiquidView, SurfaceAnimation, surface_sixteenths};
pub use png_file::{ACCEPTED_SIZES, missing_texture};
pub use registry::{BlockId, EntityTypeId, ItemId, Registry, RegistryId, TextureId};
pub use textures::{TextureLibrary, missing_id};

use crate::cinematic::{self, Cinematic};
use crate::id::Identifier;
use crate::lang::Translations;
use crate::pack::{PackError, PackSet, Sourced};
use crate::physics::LiquidCell;
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
        let (blocks, fluids) = load_blocks(packs, &textures, &mut errors);
        let entities = entity::load_entities(packs, &textures, &mut errors);
        let known = Known {
            blocks: &blocks,
            entities: &entities,
        };
        let items = load_items(packs, &known, &textures, &mut errors);
        let translations = Translations::load(&presentation, &mut errors);
        let cinematics = cinematic::load(packs, &mut errors);
        let content = Self {
            blocks,
            items,
            entities,
            fluids,
            textures,
            translations,
            cinematics,
        };
        (content, errors)
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

    /// Fluent keys every language must define for this content.
    pub fn translation_keys(&self) -> Vec<String> {
        let blocks = self
            .blocks
            .iter()
            .filter(|(key, _, block)| block.family(*key) == *key)
            .map(|(_, id, _)| id.translation_key("block"));
        let items = self
            .items
            .iter()
            .filter_map(|(_, id, item)| match item.kind {
                ItemKind::Flat(_) => Some(id.translation_key("item")),
                ItemKind::Block(_) => None,
            });
        let categories = self
            .blocks
            .iter()
            .filter_map(|(_, _, block)| block.category.as_ref())
            .chain(
                self.items
                    .iter()
                    .filter_map(|(_, _, item)| item.category.as_ref()),
            )
            .map(|category| category.translation_key("category"));
        let subtitles = self
            .cinematics
            .values()
            .flat_map(|shot| &shot.keyframes)
            .filter_map(|key| key.subtitle.clone());
        let mut keys: Vec<String> = blocks
            .chain(items)
            .chain(categories)
            .chain(subtitles)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }
}

fn load_blocks(
    packs: &PackSet,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> (Registry<BlockId, Block>, BTreeMap<BlockId, Fluid>) {
    let mut blocks = Registry::default();
    let mut fluids = Vec::new();
    blocks.insert(air_id(), Block::air());
    for (id, Sourced { value, source }) in packs.layered::<BlockDef>("blocks", errors) {
        if id == air_id() {
            errors.push(PackError::invalid(
                &source,
                "`strata:air` is built into the engine",
            ));
            continue;
        }
        if value.light_opacity > 15 {
            errors.push(PackError::invalid(
                &source,
                "light_opacity must be between 0 and 15",
            ));
        }
        let faces = Face::ALL.map(|face| match value.textures.face(face) {
            None => TextureId::MISSING,
            Some(texture) => textures.layer(texture).unwrap_or_else(|| {
                errors.push(PackError::invalid(
                    &source,
                    format!("unknown texture `{texture}`"),
                ));
                TextureId::MISSING
            }),
        });
        let placement = value.placement;
        let fluid_def = value.fluid;
        let block = Block {
            solid: value.solid,
            render: value.render,
            light_opacity: value.light_opacity.min(15),
            hardness: value.hardness,
            faces,
            sound: value.sound,
            category: value.category,
            has_item: value.has_item,
            max_stack: value.max_stack.max(1),
            cull_same: value.cull_same,
            orientation: None,
            replaceable: value.replaceable,
            rooted: value.rooted,
            hanging: value.hanging,
            fluid: None,
            slipperiness: value.slipperiness.clamp(0.0, 0.99),
        };
        let inserted = match (fluid_def, placement) {
            (Some(def), Placement::Fixed) => {
                let pending = fluid::insert_fluid(&mut blocks, &id, block, def, &source, errors);
                let inserted = pending.is_some();
                fluids.extend(pending);
                inserted
            }
            (Some(_), Placement::Axis) => false,
            (None, Placement::Fixed) => blocks.insert(id, block).is_some(),
            (None, Placement::Axis) => insert_turned(&mut blocks, &id, block),
        };
        if !inserted {
            errors.push(PackError::invalid(
                &source,
                "too many blocks (65 536 at most), a variant name (`__x`, `__1`, `__falling`…) \
                 is taken, or a fluid has `placement: Axis`",
            ));
        }
    }
    let fluids = fluid::resolve(&blocks, fluids, errors);
    (blocks, fluids)
}

/// Registers the upright block and its two lying variants `<name>__x` and `<name>__z`,
/// which have no item and share the upright block's name.
fn insert_turned(blocks: &mut Registry<BlockId, Block>, id: &Identifier, upright: Block) -> bool {
    let variant_id = |axis: Axis| match axis {
        Axis::Y => Some(id.clone()),
        Axis::X => Identifier::new(id.namespace(), &format!("{}__x", id.name())).ok(),
        Axis::Z => Identifier::new(id.namespace(), &format!("{}__z", id.name())).ok(),
    };
    let Some(ids) = Axis::ALL
        .map(variant_id)
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .filter(|ids| ids.iter().all(|id| blocks.key(id).is_none()))
        .filter(|_| blocks.len() + 3 <= usize::from(u16::MAX) + 1)
    else {
        return false;
    };
    let first = blocks.len() as u16;
    let variants = [BlockId(first), BlockId(first + 1), BlockId(first + 2)];
    for (axis, id) in Axis::ALL.into_iter().zip(ids) {
        let block = Block {
            faces: Face::ALL.map(|face| upright.texture(axis.source_face(face))),
            has_item: upright.has_item && axis == Axis::Y,
            orientation: Some(Orientation { axis, variants }),
            ..upright.clone()
        };
        blocks.insert(id, block);
    }
    true
}

/// What items may refer to.
struct Known<'a> {
    blocks: &'a Registry<BlockId, Block>,
    entities: &'a Registry<EntityTypeId, EntityType>,
}

fn load_items(
    packs: &PackSet,
    known: &Known<'_>,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> Registry<ItemId, Item> {
    let mut items = Registry::default();
    for (block_id, id, block) in known.blocks.iter().filter(|(_, _, b)| b.has_item) {
        items.insert(
            id.clone(),
            Item {
                max_stack: block.max_stack,
                category: block.category.clone(),
                kind: ItemKind::Block(block_id),
                action: None,
            },
        );
    }
    for (id, Sourced { value, source }) in packs.layered::<ItemDef>("items", errors) {
        let texture = textures.layer(&value.texture).unwrap_or_else(|| {
            errors.push(PackError::invalid(
                &source,
                format!("unknown texture `{}`", value.texture),
            ));
            TextureId::MISSING
        });
        let action = value
            .action
            .and_then(|action| item_action(known, action, &source, errors));
        let item = Item {
            max_stack: value.max_stack.max(1),
            category: value.category,
            kind: ItemKind::Flat(texture),
            action,
        };
        if items.insert(id.clone(), item).is_none() {
            errors.push(PackError::invalid(
                &source,
                format!("`{id}` is already a block item"),
            ));
        }
    }
    items
}

fn item_action(
    known: &Known<'_>,
    action: ItemActionDef,
    source: &std::path::Path,
    errors: &mut Vec<PackError>,
) -> Option<ItemAction> {
    let blocks = known.blocks;
    match action {
        ItemActionDef::TakeFluid => Some(ItemAction::TakeFluid),
        ItemActionDef::PlaceEntity(id) => {
            let entity = known.entities.key(&id);
            if entity.is_none() {
                errors.push(PackError::invalid(
                    source,
                    format!("PlaceEntity: unknown entity `{id}`"),
                ));
            }
            entity.map(ItemAction::PlaceEntity)
        }
        ItemActionDef::PlaceFluid(id) => {
            let fluid = blocks.key(&id).filter(|&key| {
                blocks
                    .get(key)
                    .and_then(|block| block.fluid)
                    .is_some_and(|state| state.level == FluidLevel::Source)
            });
            if fluid.is_none() {
                errors.push(PackError::invalid(
                    source,
                    format!("PlaceFluid: `{id}` is not a liquid source"),
                ));
            }
            fluid.map(ItemAction::PlaceFluid)
        }
    }
}
