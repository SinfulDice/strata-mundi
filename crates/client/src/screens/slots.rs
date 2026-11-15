//! Survival inventory and container screens: the player's slots, the open chest or
//! furnace, the work stations, sorting, locks, quick deposit and search. Every click
//! becomes a request the server answers.

use super::journal::JournalState;
use super::menu::{Effect, Menu};
use super::worlds::edit;
use crate::crafting::ClientCrafting;
use crate::inventory::{ClientInventory, wear_share};
use crate::ui::{ScreenModel, SlotMark, SlotPress};
use strata_core::content::{Content, ItemId, StationDef};
use strata_core::cooking::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_MODULES, FURNACE_OUTPUT};
use strata_core::inventory::{
    ACCESSORIES, ARMOR, Area, Click, HOTBAR, ItemStack, MAIN, OFFHAND, SlotRef, SortOrder,
};
use strata_core::upgrade::max_durability;
use strata_protocol::{InventoryRequest, OpenedContainer};

const MAX_SEARCH: usize = 32;

/// What an inventory screen remembers between frames.
#[derive(Debug, Default)]
pub struct SlotsState {
    /// Text searched for in the open container.
    pub search: String,
    /// Opened for a container: closes when the container goes.
    pub for_container: bool,
    /// Rows of the anvil's list scrolled past.
    pub upgrade_scroll: usize,
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
        "accessories" => (Area::Player, ACCESSORIES),
        "chest" | "furnace_modules" => {
            let start = if id == "chest" { 0 } else { FURNACE_MODULES };
            (Area::Open, start..usize::from(u16::MAX))
        }
        "furnace_input" => (Area::Open, FURNACE_INPUT..FURNACE_INPUT + 1),
        "furnace_fuel" => (Area::Open, FURNACE_FUEL..FURNACE_FUEL + 1),
        "furnace_output" => (Area::Open, FURNACE_OUTPUT..FURNACE_OUTPUT + 1),
        "craft" | "table" | "cutter" | "anvil" | "bench" => (Area::Craft, 0..usize::from(u16::MAX)),
        "craft_result" | "table_result" | "cutter_result" | "anvil_result" | "bench_result" => {
            (Area::CraftResult, 0..1)
        }
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
        let query = self.state.search.to_lowercase();
        let searched_out = area == Area::Open
            && !query.is_empty()
            && !self.name(stack.item).to_lowercase().contains(&query);
        SlotMark {
            count: stack.count,
            wear: wear_share(stack, self.content),
            locked: area == Area::Player && self.inventory.inventory().is_locked(index),
            dim: searched_out || stack.is_broken(),
        }
    }

    /// The item's tooltip, with the uses left of this one (or that it is broken).
    fn stack_tooltip(&self, stack: &ItemStack) -> String {
        let mut text = self.content.item_tooltip(stack.item, self.language);
        let full = max_durability(stack, &self.content.items, &self.content.upgrades);
        if let (Some(left), Some(full)) = (stack.durability, full) {
            let args = [("left", left.to_string()), ("full", full.to_string())];
            let tr = &self.content.translations;
            text.push('\n');
            text.push_str(&tr.text_with(self.language, "gear-uses-line", &args));
            if stack.is_broken() {
                text.push('\n');
                text.push_str(&self.tr("gear-broken-line"));
            }
        }
        let worn = self.inventory.worn_stats(self.content);
        let sets = self.content.set_lines(stack.item, &worn, self.language);
        for line in self
            .content
            .stack_upgrade_lines(stack, self.language)
            .into_iter()
            .chain(sets)
        {
            text.push('\n');
            text.push_str(&line);
        }
        text
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
        self.content.blocks.get(block)?.station.clone()
    }

    fn furnace(&self) -> bool {
        self.container
            .and_then(|open| self.content.blocks.get(open.block))
            .is_some_and(|block| block.furnace.is_some())
    }

    /// A click, a press of a gamepad button or a lock toggle on element `index` of `id`.
    pub fn press(&mut self, id: &str, index: usize, press: SlotPress) -> bool {
        if let Some(pressed) = self.list_press(id, index, press) {
            return pressed;
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
            "protection" => Some(
                self.content.translations.text_with(
                    self.language,
                    "ui-inventory-protection",
                    &[(
                        "points",
                        self.inventory
                            .worn_stats(self.content)
                            .protection
                            .to_string(),
                    )],
                ),
            ),
            "craft_note" => Some(self.craft_note()),
            "furnace_info" => Some(self.furnace_info()),
            "cutter_note" => Some(self.cutter_note()),
            "anvil_note" => Some(self.anvil_note()),
            "bench_note" => Some(self.bench_note()),
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
        if let Some(items) = self.list_items(id) {
            return items;
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
            _ => self.list_selected(id),
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
        if let Some(tooltip) = self.list_tooltip(id, index) {
            return tooltip;
        }
        let stack = self.stack(self.slot(id, index)?)?;
        Some(self.stack_tooltip(stack))
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
            "anvil_side" => !open && station == Some(StationDef::Anvil),
            "bench_side" => !open && matches!(station, Some(StationDef::Bench(_))),
            "anvil_scroll" => self.upgrades_scroll(),
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
            "anvil_repair" => self.choose_repair(),
            "anvil_earlier" => self.scroll_upgrades(-1),
            "anvil_later" => self.scroll_upgrades(1),
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

    fn scroll(&mut self, id: &str, rows: i32) {
        if id == "anvil_choices" {
            self.scroll_upgrades(rows);
        }
    }

    fn type_text(&mut self, id: &str, text: &str) {
        if id == "search" {
            edit(&mut self.state.search, text, MAX_SEARCH, |_| true);
        }
    }
}

impl SlotsModel<'_> {
    /// A block's name; a lit or turned variant is named like its family.
    fn block_title(&self, block: strata_core::content::BlockId) -> String {
        let id = self.content.block_name(block);
        id.map_or_else(String::new, |id| self.tr(&id.translation_key("block")))
    }

    /// The open furnace's speed and economy, for a usual cooking and the reference fuel.
    fn furnace_info(&self) -> String {
        let Some(view) = self.container.and_then(|c| c.furnace) else {
            return String::new();
        };
        let Some(per_coal) = view.per_coal else {
            return String::new();
        };
        let tr = &self.content.translations;
        let number = |value: f32| tr.short_decimal(self.language, f64::from(value));
        tr.text_with(
            self.language,
            "ui-furnace-info",
            &[("time", number(view.seconds)), ("cooks", number(per_coal))],
        )
    }
}

#[cfg(test)]
mod tests;
