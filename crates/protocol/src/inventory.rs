//! Inventory intents: the client says where it clicked, the server applies the rules
//! (`strata_core::inventory`) and sends the new state back.

use serde::{Deserialize, Serialize};
use strata_core::content::BlockId;
use strata_core::coords::BlockPos;
use strata_core::inventory::{Area, Click, Container, ItemStack, SlotRef, SortOrder};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum InventoryRequest {
    /// Hotbar slot in hand.
    Select(u8),
    Click {
        slot: SlotRef,
        click: Click,
    },
    /// The inventory or container screen closed: the carried stack goes back.
    Close,
    /// Sorts the main slots (`Area::Player`) or the open container. Names are in
    /// `language` (sorting by name).
    Sort {
        area: Area,
        order: SortOrder,
        language: String,
    },
    ToggleLock(u16),
    /// Unlocked main-slot stacks go into the chests around that already hold such items.
    QuickDeposit,
    /// Creative only: put any stack in a slot (or empty it).
    CreativeSet {
        index: u16,
        stack: Option<ItemStack>,
    },
    /// Throw the stack in hand: one item, or all of it.
    DropSelected {
        whole: bool,
    },
}

/// A container the player opened, as the screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenedContainer {
    /// The block clicked (its name is the screen's title).
    pub block: BlockId,
    pub pos: BlockPos,
    /// Both halves of a double chest, in order.
    pub contents: Container,
}
