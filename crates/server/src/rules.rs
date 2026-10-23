use strata_core::content::{BlockId, Content, Face, Orientation};

/// What the server needs to know about blocks to judge collisions and player actions.
#[derive(Debug, Clone)]
pub struct BlockRules {
    solid: Vec<bool>,
    placeable: Vec<bool>,
    breakable: Vec<bool>,
    orientation: Vec<Option<Orientation>>,
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
            orientation: content
                .blocks
                .iter()
                .map(|(_, _, block)| block.orientation)
                .collect(),
        }
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        self.solid
            .get(usize::from(block.0))
            .copied()
            .unwrap_or(false)
    }

    /// Creative mode: any block that has an item can be placed.
    pub fn is_placeable(&self, block: BlockId) -> bool {
        self.placeable
            .get(usize::from(block.0))
            .copied()
            .unwrap_or(false)
    }

    /// Outside creative, blocks with a negative hardness (bedrock) cannot be broken.
    pub fn is_breakable(&self, block: BlockId) -> bool {
        self.breakable
            .get(usize::from(block.0))
            .copied()
            .unwrap_or(false)
    }

    /// The variant actually set when `block` is placed against `face` (logs lie down).
    pub fn placed_against(&self, block: BlockId, face: Face) -> BlockId {
        match self.orientation.get(usize::from(block.0)) {
            Some(Some(o)) => o.along(face.axis()),
            _ => block,
        }
    }
}
