use crate::mining_rules::MiningRules;
use std::sync::Arc;
use strata_core::content::{
    BlockId, ContainerDef, Content, EntityType, EntityTypeId, EquipSlot, Facing, FurnaceDef, Item,
    ItemAction, ItemId, ItemKind, Registry, StationDef,
};
use strata_core::cooking::Cooking;
use strata_core::crafting::Recipes;
use strata_core::inventory::ItemRules;
use strata_core::physics::LiquidCell;
use strata_core::updates::UpdateRules;

/// What the server needs to know about blocks, items and entity types to judge
/// collisions and player actions.
#[derive(Debug, Clone)]
pub struct BlockRules {
    solid: Vec<bool>,
    placeable: Vec<bool>,
    breakable: Vec<bool>,
    replaceable: Vec<bool>,
    containers: Vec<Option<ContainerDef>>,
    stations: Vec<Option<StationDef>>,
    facings: Vec<Option<Facing>>,
    furnaces: Vec<Option<FurnaceDef>>,
    items: Arc<Registry<ItemId, Item>>,
    /// The item of each block (what breaking it gives back).
    block_items: Vec<Option<ItemId>>,
    liquids: Vec<Option<LiquidCell>>,
    slipperiness: Vec<f32>,
    entities: Vec<EntityType>,
    updates: Arc<UpdateRules>,
    /// Break times and loot.
    pub mining: Arc<MiningRules>,
    pub recipes: Arc<Recipes>,
    /// Fuels, cooking and cutting recipes, furnace tiers.
    pub cooking: Arc<Cooking>,
}

impl BlockRules {
    pub fn new(content: &Content) -> Self {
        Self {
            solid: content.solidity(),
            placeable: content
                .blocks
                .iter()
                .map(|(id, _, block)| block.has_item && id != BlockId::AIR)
                .collect(),
            breakable: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.hardness >= 0.0)
                .collect(),
            items: Arc::new(content.items.clone()),
            block_items: block_items(content),
            liquids: content.liquids(),
            slipperiness: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.slipperiness)
                .collect(),
            entities: content
                .entities
                .iter()
                .map(|(_, _, entity)| entity.clone())
                .collect(),
            replaceable: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.replaceable)
                .collect(),
            containers: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.container)
                .collect(),
            stations: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.station)
                .collect(),
            facings: content.blocks.iter().map(|(_, _, b)| b.facing).collect(),
            furnaces: content
                .blocks
                .iter()
                .map(|(_, _, b)| b.furnace.clone())
                .collect(),
            recipes: Arc::new(content.recipes.clone()),
            cooking: Arc::new(content.cooking.clone()),
            updates: Arc::new(UpdateRules::new(content)),
            mining: Arc::new(MiningRules::new(content)),
        }
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        flag(&self.solid, block)
    }

    /// Placing while aiming at it replaces it (plants).
    pub fn is_replaceable(&self, block: BlockId) -> bool {
        flag(&self.replaceable, block)
    }

    /// Neighbour reactions and liquids.
    pub fn updates(&self) -> &Arc<UpdateRules> {
        &self.updates
    }

    /// Creative mode: any block that has an item can be placed.
    pub fn container(&self, block: BlockId) -> Option<ContainerDef> {
        self.containers.get(usize::from(block.0)).copied().flatten()
    }

    /// The work station a block opens (crafting table).
    pub fn station(&self, block: BlockId) -> Option<StationDef> {
        self.stations.get(usize::from(block.0)).copied().flatten()
    }

    /// The front and lit variants of a block that has a front (furnaces).
    pub fn facing(&self, block: BlockId) -> Option<Facing> {
        self.facings.get(usize::from(block.0)).copied().flatten()
    }

    pub fn furnace(&self, block: BlockId) -> Option<&FurnaceDef> {
        self.furnaces.get(usize::from(block.0))?.as_ref()
    }

    /// Slots of the block entity of `block`: a container's, or a furnace's three and
    /// its tier's upgrade slots.
    pub fn entity_slots(&self, block: BlockId) -> Option<usize> {
        if let Some(container) = self.container(block) {
            return Some(container.slots());
        }
        let furnace = self.furnace(block)?;
        let tier = self
            .cooking
            .tier(&furnace.tier)
            .copied()
            .unwrap_or_default();
        Some(strata_core::cooking::FURNACE_MODULES + usize::from(tier.upgrade_slots))
    }

    /// The block of the same furnace family and of `tier`, with the same front and light.
    pub fn furnace_of_tier(
        &self,
        block: BlockId,
        tier: &strata_core::id::Identifier,
    ) -> Option<BlockId> {
        let family = &self.furnace(block)?.family;
        let facing = self.facing(block);
        let index = self.furnaces.iter().position(|other| {
            other
                .as_ref()
                .is_some_and(|o| o.family == *family && o.tier == *tier)
        })?;
        let found = BlockId(u16::try_from(index).ok()?);
        let Some(facing) = facing else {
            return Some(found);
        };
        let variants = self.facing(found)?;
        Some(variants.variant(facing.front, facing.lit))
    }

    pub fn items(&self) -> &Registry<ItemId, Item> {
        &self.items
    }

    /// The first entity type for items on the ground, if a pack has one.
    pub fn dropped_item_entity(
        &self,
    ) -> Option<(EntityTypeId, strata_core::content::DroppedItemDef)> {
        self.entities
            .iter()
            .enumerate()
            .find_map(|(index, entity)| {
                Some((
                    EntityTypeId(u16::try_from(index).ok()?),
                    entity.dropped_item?,
                ))
            })
    }

    /// A block that has an item can be placed (survival also needs it in hand).
    pub fn is_placeable(&self, block: BlockId) -> bool {
        flag(&self.placeable, block)
    }

    pub fn item_action(&self, item: ItemId) -> Option<ItemAction> {
        self.items.get(item)?.action
    }

    /// The first item doing `action` (the bucket full of a liquid, the empty bucket).
    pub fn item_with(&self, action: ItemAction) -> Option<ItemId> {
        self.items
            .iter()
            .find(|(_, _, item)| item.action == Some(action))
            .map(|(id, _, _)| id)
    }

    pub fn block_item(&self, block: BlockId) -> Option<ItemId> {
        self.block_items
            .get(usize::from(block.0))
            .copied()
            .flatten()
    }

    /// Category of an item (sorting by type).
    pub fn item_category(&self, item: ItemId) -> Option<&strata_core::id::Identifier> {
        self.items.get(item)?.category.as_ref()
    }

    /// The liquid a body can swim in at this block.
    pub fn liquid(&self, block: BlockId) -> Option<LiquidCell> {
        self.liquids.get(usize::from(block.0)).copied().flatten()
    }

    /// Fraction of a sliding speed kept per tick on top of the block.
    pub fn slipperiness(&self, block: BlockId) -> f64 {
        self.slipperiness
            .get(usize::from(block.0))
            .map_or(strata_core::physics::DEFAULT_SLIPPERINESS, |&s| {
                f64::from(s)
            })
    }

    pub fn entity(&self, kind: EntityTypeId) -> Option<&EntityType> {
        self.entities.get(usize::from(kind.0))
    }

    /// Whether it is a liquid source (what an empty bucket takes).
    pub fn is_fluid_source(&self, block: BlockId) -> bool {
        self.updates
            .fluid_state(block)
            .is_some_and(|state| state.source == block)
    }

    /// Outside creative, blocks with a negative hardness (bedrock) cannot be broken.
    pub fn is_breakable(&self, block: BlockId) -> bool {
        flag(&self.breakable, block)
    }
}

impl ItemRules for BlockRules {
    fn max_stack(&self, item: ItemId) -> u16 {
        self.items.max_stack(item)
    }

    fn durability(&self, item: ItemId) -> Option<u16> {
        self.items.durability(item)
    }

    fn equip(&self, item: ItemId) -> Option<EquipSlot> {
        self.items.equip(item)
    }

    fn remainder(&self, item: ItemId) -> Option<ItemId> {
        self.items.remainder(item)
    }
}

/// Turned variants (a log lying down) give back the item of the upright block.
fn block_items(content: &Content) -> Vec<Option<ItemId>> {
    let mut items = vec![None; content.blocks.len()];
    for (item, _, entry) in content.items.iter() {
        if let ItemKind::Block(block) = entry.kind {
            let mut variants = vec![block];
            if let Some(entry) = content.blocks.get(block) {
                variants.extend(entry.orientation.iter().flat_map(|o| o.variants));
                variants.extend(
                    entry
                        .attachment
                        .iter()
                        .flat_map(|a| a.variants.into_iter().flatten()),
                );
                variants.extend(entry.facing.iter().flat_map(|f| f.variants.concat()));
            }
            for variant in variants {
                if let Some(slot) = items.get_mut(usize::from(variant.0)) {
                    *slot = Some(item);
                }
            }
        }
    }
    items
}

fn flag(flags: &[bool], block: BlockId) -> bool {
    flags.get(usize::from(block.0)).copied().unwrap_or(false)
}
