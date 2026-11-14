use super::container::Container;
use crate::content::ItemId;
use serde::{Deserialize, Serialize};

/// The crafting grid in front of a player: 2×2 in the inventory, 3×3 at a crafting
/// table. Its stacks belong to the player until the screen closes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CraftGrid {
    pub side: u8,
    pub slots: Container,
}

impl Default for CraftGrid {
    fn default() -> Self {
        Self::new(Self::INVENTORY)
    }
}

impl CraftGrid {
    /// Side of the grid of the inventory screen.
    pub const INVENTORY: u8 = 2;

    pub fn new(side: u8) -> Self {
        let side = side.clamp(1, 3);
        Self {
            side,
            slots: Container::new(usize::from(side) * usize::from(side)),
        }
    }

    /// The item of each cell, row by row.
    pub fn items(&self) -> Vec<Option<ItemId>> {
        self.slots
            .slots
            .iter()
            .map(|slot| slot.as_ref().map(|stack| stack.item))
            .collect()
    }
}
