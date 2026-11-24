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
use strata_core::inventory::{Area, Click, ItemStack, SortOrder};
use strata_core::texture::Rgba;
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
    /// Rows of a bench's list scrolled past.
    pub bench_scroll: usize,
    /// Rows of a large chest scrolled past.
    pub chest_scroll: usize,
    /// Which of the open storage's filtering upgrades the filter panel shows.
    pub filter_choice: usize,
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

impl SlotsModel<'_> {
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
        if let Some(line) = self.content.bag_stack_line(stack, self.language) {
            text.push('\n');
            text.push_str(&line);
        }
        let equipment = self.inventory.equipment(self.content);
        let sets = (self.content).set_lines(stack.item, &equipment.stats, self.language);
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

    pub(super) fn station(&self) -> Option<StationDef> {
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
                            .equipment(self.content)
                            .stats
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
            "chest_rows" | "filter_title" | "filter_mode" => self.storage_text(id),
            "help" => Some(
                Some(self.craft_note())
                    .filter(|note| !note.is_empty())
                    .unwrap_or_else(|| self.tr("ui-inventory-survival-help")),
            ),
            "station_title" => self
                .crafting
                .and_then(|c| c.view.station)
                .map(|(block, _)| self.block_title(block)),
            "title" => self.container.map(|open| match open.bag {
                Some(bag) => self.name(bag),
                None => self.block_title(open.block),
            }),
            "tank_contents" => self.tank_text(),
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
        let Some((area, range)) = self.grid(id) else {
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
        if id == "tank_level" {
            return self.tank_level();
        }
        let view = self.container?.furnace?;
        match id {
            "furnace_flame" => Some(view.flame()),
            "furnace_arrow" => Some(view.arrow()),
            _ => None,
        }
    }

    fn color(&self, id: &str) -> Option<Rgba> {
        self.bench_color(id)
    }

    fn tooltip(&self, id: &str, index: usize) -> Option<String> {
        if let Some(tooltip) = self.list_tooltip(id, index) {
            return tooltip;
        }
        let stack = self.stack(self.slot(id, index)?)?;
        Some(self.stack_tooltip(stack))
    }

    fn shown(&self, id: &str) -> bool {
        if let Some(shown) = self
            .bench_group_shown(id)
            .or_else(|| self.storage_shown(id))
            .or_else(|| self.bag_shown(id))
        {
            return shown;
        }
        let open = self.container.is_some();
        let station = self.station();
        match id {
            "chest_side" => open && !self.furnace(),
            "furnace_side" => open && self.furnace(),
            "furnace_no_modules" => self
                .container
                .is_some_and(|c| c.contents.len() <= usize::from(c.upgrades_at)),
            "table_side" => !open && matches!(station, Some(StationDef::Crafting(_))),
            "cutter_side" => !open && station == Some(StationDef::Stonecutter),
            "anvil_side" => !open && station == Some(StationDef::Anvil),
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
            "bench_earlier" => self.scroll_bench(-1),
            "bench_later" => self.scroll_bench(1),
            "chest_earlier" => self.scroll_chest(-1),
            "chest_later" => self.scroll_chest(1),
            "filter_next" => self.next_filter(),
            "filter_mode" => self.toggle_filter_mode(),
            "tank" => self.inventory.request(InventoryRequest::Tank),
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
        match id {
            "anvil_choices" => self.scroll_upgrades(rows),
            "bench_choices" => self.scroll_bench(rows),
            "chest" => self.scroll_chest(rows),
            _ => {}
        }
    }

    fn scroll_position(&self, id: &str) -> Option<usize> {
        (id == "chest").then(|| self.chest_scroll())
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
}

mod grids;

#[cfg(test)]
mod tests;
