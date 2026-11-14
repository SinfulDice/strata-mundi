use std::sync::Arc;
use strata_core::content::{
    BlockId, ContainerDef, Content, EntityType, EntityTypeId, EquipSlot, Item, ItemAction, ItemId,
    ItemKind, Registry,
};
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
    items: Arc<Registry<ItemId, Item>>,
    /// The item of each block (what breaking it gives back).
    block_items: Vec<Option<ItemId>>,
    liquids: Vec<Option<LiquidCell>>,
    slipperiness: Vec<f32>,
    entities: Vec<EntityType>,
    updates: Arc<UpdateRules>,
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
            updates: Arc::new(UpdateRules::new(content)),
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

    pub fn is_placeable(&self, block: BlockId) -> bool {
        flag(&self.placeable, block)
    }

    pub fn is_item(&self, item: ItemId) -> bool {
        self.items.get(item).is_some()
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

    /// The block an item places, if it is a block's item.
    pub fn item_block(&self, item: ItemId) -> Option<BlockId> {
        match self.items.get(item)?.kind {
            ItemKind::Block(block) => Some(block),
            ItemKind::Flat(_) => None,
        }
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
