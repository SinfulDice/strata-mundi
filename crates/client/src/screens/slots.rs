//! Survival inventory and container screens: the player's slots, the open chest or
//! furnace, the work stations, sorting, locks, quick deposit and search. Every click
//! becomes a request the server answers.

use super::journal::JournalState;
use super::menu::{Effect, Menu};
use super::worlds::edit;
use crate::crafting::ClientCrafting;
use crate::inventory::ClientInventory;
use crate::ui::{ScreenModel, SlotMark, SlotPress};
use strata_core::content::{Content, ItemId, StationDef};
use strata_core::cooking::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_MODULES, FURNACE_OUTPUT};
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
    pub inventory: &'a mut ClientInventory,
    pub container: Option<&'a OpenedContainer>,
    pub crafting: Option<&'a ClientCrafting>,
    pub effects: &'a mut Vec<Effect>,
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
        "chest" | "furnace_modules" => {
            let start = if id == "chest" { 0 } else { FURNACE_MODULES };
            (Area::Open, start..usize::from(u16::MAX))
        }
        "furnace_input" => (Area::Open, FURNACE_INPUT..FURNACE_INPUT + 1),
        "furnace_fuel" => (Area::Open, FURNACE_FUEL..FURNACE_FUEL + 1),
        "furnace_output" => (Area::Open, FURNACE_OUTPUT..FURNACE_OUTPUT + 1),
        "craft" | "table" | "cutter" => (Area::Craft, 0..usize::from(u16::MAX)),
        "craft_result" | "table_result" | "cutter_result" => (Area::CraftResult, 0..1),
        _ => return None,
    })
}

impl SlotsModel<'_> {
    /// The slot behind element `index` of grid `id`.
    pub fn slot(&self, id: &str, index: usize) -> Option<SlotRef> {
        let (area, range) = grid(id)?;
        let slot = range.start + index;
        let exists = range.contains(&slot)
            && match area {
                Area::Player | Area::CraftResult => true,
                Area::Open => self.container.is_some_and(|c| slot < c.contents.len()),
                Area::Craft => self.crafting.is_some_and(|c| slot < c.view.grid.len()),
            };
        exists.then_some(SlotRef {
            area,
            index: slot as u16,
        })
    }

    fn stack(&self, slot: SlotRef) -> Option<&ItemStack> {
        let index = usize::from(slot.index);
        match slot.area {
            Area::Player => self.inventory.stack(index),
            Area::Open => self.container?.contents.get(index),
            Area::Craft => self.crafting?.view.grid.get(index),
            Area::CraftResult => self.crafting?.view.result.as_ref(),
        }
    }

    fn stacks(&self, id: &str) -> Vec<Option<&ItemStack>> {
        let Some((area, range)) = grid(id) else {
            return Vec::new();
        };
        let end = match area {
            Area::Player | Area::CraftResult => range.end,
            Area::Open => self.container.map_or(0, |c| c.contents.len()),
            Area::Craft => self.crafting.map_or(0, |c| c.view.grid.len()),
        }
        .min(range.end);
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
                locked: area == Area::Player && self.inventory.inventory().is_locked(index),
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
            locked: area == Area::Player && self.inventory.inventory().is_locked(index),
            dim: area == Area::Open
                && !query.is_empty()
                && !self.name(stack.item).to_lowercase().contains(&query),
        }
    }

    fn name(&self, item: ItemId) -> String {
        self.content.item_name(item, self.language)
    }

    fn tr(&self, key: &str) -> String {
        self.content.translations.text(self.language, key)
    }

    /// Under the grid: the recipe just learnt, or a hint that the items laid belong to
    /// a recipe not known yet.
    fn craft_note(&self) -> String {
        let Some(crafting) = self.crafting else {
            return String::new();
        };
        let learned = crafting.learned.and_then(|r| self.content.recipes.get(r));
        match learned {
            Some(recipe) => self.content.translations.text_with(
                self.language,
                "ui-craft-learned",
                &[("item", self.name(recipe.result))],
            ),
            None if crafting.view.hint => self.tr("ui-craft-hint"),
            None => String::new(),
        }
    }

    fn station(&self) -> Option<StationDef> {
        let (block, _) = self.crafting?.view.station?;
        self.content.blocks.get(block)?.station
    }

    fn furnace(&self) -> bool {
        self.container
            .and_then(|open| self.content.blocks.get(open.block))
            .is_some_and(|block| block.furnace.is_some())
    }

    /// A click, a press of a gamepad button or a lock toggle on element `index` of `id`.
    pub fn press(&mut self, id: &str, index: usize, press: SlotPress) -> bool {
        if id == "cut_choices" {
            return self.choose_cut(index, press);
        }
        let Some(slot) = self.slot(id, index) else {
            return false;
        };
        match press {
            SlotPress::Click(click) => self.inventory.click(slot, click),
            SlotPress::ToggleLock if slot.area == Area::Player => {
                self.inventory
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
            "craft_note" => Some(self.craft_note()),
            "furnace_info" => Some(self.furnace_info()),
            "cutter_note" => Some(self.cutter_note()),
            "help" => Some(
                Some(self.craft_note())
                    .filter(|note| !note.is_empty())
                    .unwrap_or_else(|| self.tr("ui-inventory-survival-help")),
            ),
            "station_title" => self
                .crafting
                .and_then(|c| c.view.station)
                .map(|(block, _)| self.block_title(block)),
            "title" => self.container.map(|open| self.block_title(open.block)),
            _ => None,
        }
    }

    fn items(&self, id: &str) -> Vec<Option<ItemId>> {
        if id == "cut_choices" {
            return self.cuts().map(|(_, stack)| Some(stack.item)).collect();
        }
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
        let carried = self.inventory.inventory().cursor.as_ref()?;
        Some((carried.item, carried.count))
    }

    fn selected(&self, id: &str) -> Option<usize> {
        match id {
            "hotbar" => Some(self.inventory.selected_index()),
            "cut_choices" => {
                let chosen = self.crafting?.view.chosen?;
                self.cuts().position(|(index, _)| *index == chosen)
            }
            _ => None,
        }
    }

    fn gauge(&self, id: &str) -> Option<f32> {
        let view = self.container?.furnace?;
        match id {
            "furnace_flame" => Some(view.flame()),
            "furnace_arrow" => Some(view.arrow()),
            _ => None,
        }
    }

    fn tooltip(&self, id: &str, index: usize) -> Option<String> {
        if id == "cut_choices" {
            let (_, stack) = self.cuts().nth(index)?;
            return Some(self.content.item_tooltip(stack.item, self.language));
        }
        let stack = self.stack(self.slot(id, index)?)?;
        Some(self.content.item_tooltip(stack.item, self.language))
    }

    fn shown(&self, id: &str) -> bool {
        let open = self.container.is_some();
        let station = self.station();
        match id {
            "chest_side" => open && !self.furnace(),
            "furnace_side" => open && self.furnace(),
            "furnace_no_modules" => self
                .container
                .is_some_and(|c| c.contents.len() <= FURNACE_MODULES),
            "table_side" => !open && matches!(station, Some(StationDef::Crafting(_))),
            "cutter_side" => !open && station == Some(StationDef::Stonecutter),
            "player_side" => !open && station.is_none(),
            "crafting" => self.crafting.is_some(),
            _ => true,
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        let language = self.language.to_owned();
        match id {
            "sort_type" => self
                .inventory
                .sort(Area::Player, SortOrder::Type, &language),
            "sort_name" => self
                .inventory
                .sort(Area::Player, SortOrder::Name, &language),
            "sort_chest" => self.inventory.sort(Area::Open, SortOrder::Type, &language),
            "deposit" => self.inventory.request(InventoryRequest::QuickDeposit),
            "book" => self
                .effects
                .push(Effect::Push(Menu::Journal(JournalState::for_grid()))),
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

impl SlotsModel<'_> {
    /// What the stonecutter can make of the item laid: (recipe index, result).
    fn cuts(&self) -> impl Iterator<Item = &(u16, ItemStack)> {
        self.crafting.into_iter().flat_map(|c| c.view.cuts.iter())
    }

    fn choose_cut(&mut self, index: usize, press: SlotPress) -> bool {
        let Some(&(choice, _)) = self.cuts().nth(index) else {
            return false;
        };
        if matches!(press, SlotPress::Click(Click::Primary | Click::Transfer)) {
            self.inventory.request(InventoryRequest::ChooseCut(choice));
        }
        true
    }

    /// A block's name; a lit or turned variant is named like its family.
    fn block_title(&self, block: strata_core::content::BlockId) -> String {
        let id = self.content.block_name(block);
        id.map_or_else(String::new, |id| self.tr(&id.translation_key("block")))
    }

    fn cutter_note(&self) -> String {
        let laid = self.crafting.is_some_and(|c| c.view.grid.get(0).is_some());
        if laid && self.cuts().next().is_none() {
            self.tr("ui-stonecutter-none")
        } else {
            self.tr("ui-stonecutter-help")
        }
    }

    /// The open furnace's speed and economy, for a usual cooking and a piece of coal.
    fn furnace_info(&self) -> String {
        let Some(view) = self.container.and_then(|c| c.furnace) else {
            return String::new();
        };
        let round = |value: f32| format!("{}", (value * 10.0).round() / 10.0);
        self.content.translations.text_with(
            self.language,
            "ui-furnace-info",
            &[
                ("time", round(view.seconds)),
                ("cooks", round(view.per_coal)),
            ],
        )
    }
}

#[cfg(test)]
mod tests;
