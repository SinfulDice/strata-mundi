//! Survival inventory and container screens: the player's slots, the open chest, sorting,
//! locks, quick deposit and search. Every click becomes a request the server answers.

use super::worlds::edit;
use crate::hotbar::Hotbar;
use crate::ui::{ScreenModel, SlotMark, SlotPress};
use strata_core::content::{Content, ItemId};
use strata_core::inventory::{
    ARMOR, Area, Click, HOTBAR, ItemRules, ItemStack, MAIN, OFFHAND, SlotRef, SortOrder,
};
use strata_protocol::{InventoryRequest, OpenedContainer};

const MAX_SEARCH: usize = 32;

/// What an inventory screen remembers between frames.
#[derive(Debug, Default)]
pub struct SlotsState {
    /// Text searched for in the open container.
    pub search: String,
    /// Opened for a container: closes when the container goes.
    pub for_container: bool,
}

pub struct SlotsModel<'a> {
    pub state: &'a mut SlotsState,
    pub hotbar: &'a mut Hotbar,
    pub container: Option<&'a OpenedContainer>,
    pub content: &'a Content,
    pub language: &'a str,
}

/// Where the slots of a grid are: the area and its first index.
fn grid(id: &str) -> Option<(Area, std::ops::Range<usize>)> {
    Some(match id {
        "hotbar" => (Area::Player, HOTBAR),
        "main" => (Area::Player, MAIN),
        "armor" => (Area::Player, ARMOR),
        "offhand" => (Area::Player, OFFHAND..OFFHAND + 1),
        "chest" => (Area::Open, 0..usize::from(u16::MAX)),
        _ => return None,
    })
}

impl SlotsModel<'_> {
    /// The slot behind element `index` of grid `id`.
    pub fn slot(&self, id: &str, index: usize) -> Option<SlotRef> {
        let (area, range) = grid(id)?;
        let slot = range.start + index;
        let exists = match area {
            Area::Player => range.contains(&slot),
            Area::Open => self.container.is_some_and(|c| slot < c.contents.len()),
        };
        exists.then_some(SlotRef {
            area,
            index: slot as u16,
        })
    }

    fn stack(&self, slot: SlotRef) -> Option<&ItemStack> {
        let index = usize::from(slot.index);
        match slot.area {
            Area::Player => self.hotbar.stack(index),
            Area::Open => self.container?.contents.get(index),
        }
    }

    fn stacks(&self, id: &str) -> Vec<Option<&ItemStack>> {
        let Some((area, range)) = grid(id) else {
            return Vec::new();
        };
        let end = match area {
            Area::Player => range.end,
            Area::Open => self.container.map_or(0, |c| c.contents.len()),
        };
        (range.start..end)
            .map(|index| {
                self.stack(SlotRef {
                    area,
                    index: index as u16,
                })
            })
            .collect()
    }

    fn mark(&self, area: Area, index: usize, stack: Option<&ItemStack>) -> SlotMark {
        let Some(stack) = stack else {
            return SlotMark {
                locked: area == Area::Player && self.hotbar.inventory().is_locked(index),
                ..SlotMark::default()
            };
        };
        let full = self.content.items.durability(stack.item);
        let query = self.state.search.to_lowercase();
        SlotMark {
            count: stack.count,
            wear: stack
                .durability
                .zip(full)
                .map(|(left, full)| f32::from(left) / f32::from(full.max(1))),
            locked: area == Area::Player && self.hotbar.inventory().is_locked(index),
            dim: area == Area::Open
                && !query.is_empty()
                && !self.name(stack.item).to_lowercase().contains(&query),
        }
    }

    fn name(&self, item: ItemId) -> String {
        self.content.item_name(item, self.language)
    }

    /// A click, a press of a gamepad button or a lock toggle on element `index` of `id`.
    pub fn press(&mut self, id: &str, index: usize, press: SlotPress) -> bool {
        let Some(slot) = self.slot(id, index) else {
            return false;
        };
        match press {
            SlotPress::Click(click) => self.hotbar.click(slot, click),
            SlotPress::ToggleLock if slot.area == Area::Player => {
                self.hotbar
                    .request(InventoryRequest::ToggleLock(slot.index));
            }
            SlotPress::ToggleLock => {}
        }
        true
    }
}

impl ScreenModel for SlotsModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        match id {
            "search" => Some(self.state.search.clone()),
            "title" => self.container.map(|open| {
                let block = self.content.blocks.identifier(open.block);
                block.map_or_else(String::new, |b| {
                    self.content
                        .translations
                        .text(self.language, &b.translation_key("block"))
                })
            }),
            _ => None,
        }
    }

    fn items(&self, id: &str) -> Vec<Option<ItemId>> {
        self.stacks(id)
            .into_iter()
            .map(|stack| stack.map(|s| s.item))
            .collect()
    }

    fn marks(&self, id: &str) -> Vec<SlotMark> {
        let Some((area, range)) = grid(id) else {
            return Vec::new();
        };
        self.stacks(id)
            .into_iter()
            .enumerate()
            .map(|(offset, stack)| self.mark(area, range.start + offset, stack))
            .collect()
    }

    fn carried(&self) -> Option<(ItemId, u16)> {
        let carried = self.hotbar.inventory().cursor.as_ref()?;
        Some((carried.item, carried.count))
    }

    fn selected(&self, id: &str) -> Option<usize> {
        (id == "hotbar").then(|| self.hotbar.selected_index())
    }

    fn tooltip(&self, id: &str, index: usize) -> Option<String> {
        let stack = self.stack(self.slot(id, index)?)?;
        Some(self.name(stack.item))
    }

    fn shown(&self, id: &str) -> bool {
        match id {
            "chest_side" => self.container.is_some(),
            "player_side" => self.container.is_none(),
            _ => true,
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        let language = self.language.to_owned();
        match id {
            "sort_type" => self.hotbar.sort(Area::Player, SortOrder::Type, &language),
            "sort_name" => self.hotbar.sort(Area::Player, SortOrder::Name, &language),
            "sort_chest" => self.hotbar.sort(Area::Open, SortOrder::Type, &language),
            "deposit" => self.hotbar.request(InventoryRequest::QuickDeposit),
            _ => {
                self.press(id, index, SlotPress::Click(Click::Primary));
            }
        }
    }

    fn slot_press(&mut self, id: &str, index: usize, press: SlotPress) -> bool {
        self.press(id, index, press)
    }

    fn quick_slot(&mut self, id: &str, index: usize, slot: usize) {
        self.press(id, index, SlotPress::Click(Click::HotbarSwap(slot as u8)));
    }

    fn type_text(&mut self, id: &str, text: &str) {
        if id == "search" {
            edit(&mut self.state.search, text, MAX_SEARCH, |_| true);
        }
    }
}

#[cfg(test)]
mod tests;
