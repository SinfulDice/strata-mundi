use std::sync::Arc;
use strata_core::content::{BlockId, Content, Face, ItemId, Orientation};
use strata_core::updates::UpdateRules;

/// What the server needs to know about blocks to judge collisions and player actions.
#[derive(Debug, Clone)]
pub struct BlockRules {
    solid: Vec<bool>,
    placeable: Vec<bool>,
    breakable: Vec<bool>,
    orientation: Vec<Option<Orientation>>,
    replaceable: Vec<bool>,
    item_count: usize,
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
