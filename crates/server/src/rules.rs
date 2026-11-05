use std::sync::Arc;
use strata_core::content::{
    BlockId, Content, EntityType, EntityTypeId, Face, ItemAction, ItemId, Orientation,
};
use strata_core::physics::LiquidCell;
use strata_core::updates::UpdateRules;

/// What the server needs to know about blocks, items and entity types to judge
/// collisions and player actions.
#[derive(Debug, Clone)]
pub struct BlockRules {
    solid: Vec<bool>,
    placeable: Vec<bool>,
    breakable: Vec<bool>,
    orientation: Vec<Option<Orientation>>,
    replaceable: Vec<bool>,
    item_count: usize,
    actions: Vec<Option<ItemAction>>,
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
            item_count: content.items.len(),
            actions: content
                .items
                .iter()
                .map(|(_, _, item)| item.action)
                .collect(),
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
            orientation: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.orientation)
                .collect(),
            replaceable: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.replaceable)
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
    pub fn is_placeable(&self, block: BlockId) -> bool {
        flag(&self.placeable, block)
    }

    pub fn is_item(&self, item: ItemId) -> bool {
        usize::from(item.0) < self.item_count
    }

    pub fn item_action(&self, item: ItemId) -> Option<ItemAction> {
        self.actions.get(usize::from(item.0)).copied().flatten()
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

    /// The variant actually set when `block` is placed against `face` (logs lie down).
    pub fn placed_against(&self, block: BlockId, face: Face) -> BlockId {
        match self.orientation.get(usize::from(block.0)) {
            Some(Some(o)) => o.along(face.axis()),
            _ => block,
        }
    }
}

fn flag(flags: &[bool], block: BlockId) -> bool {
    flags.get(usize::from(block.0)).copied().unwrap_or(false)
}
