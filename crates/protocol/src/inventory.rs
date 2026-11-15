//! Inventory intents: the client says where it clicked, the server applies the rules
//! (`strata_core::inventory`) and sends the new state back.

use serde::{Deserialize, Serialize};
use strata_core::content::{BlockId, RecipeId};
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
    /// The recipe book: lay a known recipe in the crafting grid with the inventory's
    /// items, once or (`all`) as many times as possible.
    FillRecipe {
        recipe: RecipeId,
        all: bool,
    },
    /// The stonecutter: cut what lies in its cell with this recipe (an index in
    /// `Cooking::cuttings`).
    ChooseCut(u16),
}

/// A container the player opened, as the screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenedContainer {
    /// The block clicked (its name is the screen's title).
    pub block: BlockId,
    pub pos: BlockPos,
    /// Both halves of a double chest, in order.
    pub contents: Container,
    /// A furnace's fire and cooking (`ServerMessage::Furnace` keeps it up to date).
    pub furnace: Option<FurnaceView>,
}

/// The player's crafting grid, as the screen shows it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CraftingView {
    /// 2 in the inventory, 3 at a crafting table.
    pub side: u8,
    pub grid: Container,
    /// What taking the result would give.
    pub result: Option<ItemStack>,
    /// The items laid are part of a recipe not known yet.
    pub hint: bool,
    /// The work station opened (its name is the screen's title).
    pub station: Option<(BlockId, BlockPos)>,
    /// At a stonecutter: what the item laid can become (recipe index, result), and the
    /// one chosen.
    pub cuts: Vec<(u16, ItemStack)>,
    pub chosen: Option<u16>,
}

/// The open furnace's fire and cooking, for its flame and arrow.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct FurnaceView {
    pub burn_left: u32,
    pub burn_total: u32,
    pub progress: u32,
    /// Ticks the item in the input takes (0: nothing to cook).
    pub cook_time: u32,
    /// Seconds a usual cooking (10 s in a stone furnace) takes here.
    pub seconds: f32,
    /// Cookings the packs' reference fuel (coal) gives here.
    pub per_coal: Option<f32>,
}

impl FurnaceView {
    /// Fraction of the fuel left, 0 to 1.
    pub fn flame(&self) -> f32 {
        fraction(self.burn_left, self.burn_total)
    }

    /// Fraction of the cooking done, 0 to 1.
    pub fn arrow(&self) -> f32 {
        fraction(self.progress, self.cook_time)
    }

    /// The view one tick later if nothing but time changes: the fire burns down, the
    /// cooking goes on (and starts again once done), or cools when the fire is out. The
    /// server sends a view only when the furnace leaves this course; the client follows
    /// it in between.
    pub fn advanced(mut self) -> Self {
        if self.burn_left == 0 {
            self.progress = self.progress.saturating_sub(2);
            return self;
        }
        self.burn_left -= 1;
        self.progress = match self.cook_time {
            0 => 0,
            time if self.progress + 1 >= time => 0,
            _ => self.progress + 1,
        };
        self
    }
}

fn fraction(part: u32, whole: u32) -> f32 {
    if whole == 0 {
        return 0.0;
    }
    (part as f32 / whole as f32).clamp(0.0, 1.0)
}
