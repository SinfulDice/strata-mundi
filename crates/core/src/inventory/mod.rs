//! Item stacks, containers and the player's inventory, with the rules of every click.
//! Pure logic: the server applies it, the tests check it.

mod click;
mod container;
mod sort;
mod stack;

pub use click::{Area, Click, SlotRef, Window};
pub use container::{ARMOR, Container, HOTBAR, MAIN, OFFHAND, PLAYER_SLOTS, PlayerInventory};
pub use sort::{SortOrder, deposit_matching, sort};
pub use stack::{DataValue, ItemData, ItemRules, ItemStack, clear_empty};

#[cfg(test)]
mod tests;
