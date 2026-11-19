//! The open container's grids in the inventory screen: a chest's storage, scrolled by
//! rows when it is taller than the screen, its upgrade slots (and a furnace's), and the
//! filter of a filter or destruction upgrade, set with the carried item.

use super::slots::SlotsModel;
use crate::ui::SlotPress;
use std::ops::Range;
use strata_core::content::ItemId;
use strata_core::inventory::{Area, Click, ItemStack};
use strata_core::storage::{FILTER_CELLS, filter_of};
use strata_core::upgrade::Effect;
use strata_protocol::InventoryRequest;

/// Rows of the chest grid on screen.
const CHEST_ROWS: usize = 6;
const COLUMNS: usize = 9;

impl SlotsModel<'_> {
    /// The open container's slots behind a grid.
    pub(super) fn storage_grid(&self, id: &str) -> Option<(Area, Range<usize>)> {
        let open = self.container?;
        let upgrades = usize::from(open.upgrades_at);
        let range = match id {
            "chest" => {
                let start = (self.state.chest_scroll * COLUMNS).min(upgrades);
                start..upgrades.min(start + CHEST_ROWS * COLUMNS)
            }
            "chest_upgrades" | "furnace_modules" => upgrades..open.contents.len(),
            _ => return None,
        };
        Some((Area::Open, range))
    }

    fn storage_rows(&self) -> usize {
        self.container
            .map_or(0, |open| usize::from(open.upgrades_at).div_ceil(COLUMNS))
    }

    pub(super) fn scroll_chest(&mut self, rows: i32) {
        let max = self.storage_rows().saturating_sub(CHEST_ROWS);
        let scrolled = self.state.chest_scroll as i64 + i64::from(rows);
        self.state.chest_scroll = scrolled.clamp(0, max as i64) as usize;
    }

    pub(super) fn storage_shown(&self, id: &str) -> Option<bool> {
        let chest = self.container.is_some() && self.content_furnace().is_none();
        Some(match id {
            "chest_scroll" => chest && self.storage_rows() > CHEST_ROWS,
            "storage_upgrades" => chest && !self.upgrade_slots().is_empty(),
            "storage_side" => {
                chest && (!self.upgrade_slots().is_empty() || self.storage_rows() > CHEST_ROWS)
            }
            "storage_filter" => chest && !self.filtering_slots().is_empty(),
            "filter_choices" => self.filtering_slots().len() > 1,
            _ => return None,
        })
    }

    fn content_furnace(&self) -> Option<&strata_core::content::FurnaceDef> {
        let open = self.container?;
        self.content.blocks.get(open.block)?.furnace.as_ref()
    }

    fn upgrade_slots(&self) -> Range<usize> {
        self.container.map_or(0..0, |open| {
            usize::from(open.upgrades_at)..open.contents.len()
        })
    }

    /// The upgrade slots holding an upgrade with a filter (filter, destruction).
    fn filtering_slots(&self) -> Vec<usize> {
        let Some(open) = self.container else {
            return Vec::new();
        };
        let upgrades = &self.content.upgrades;
        self.upgrade_slots()
            .filter(|&slot| {
                let upgrade = open
                    .contents
                    .get(slot)
                    .and_then(|stack| upgrades.of_item(&self.content.items, stack.item));
                upgrade.and_then(|u| u.levels.first()).is_some_and(|level| {
                    (level.effects.iter()).any(|e| matches!(e, Effect::Filter | Effect::Void))
                })
            })
            .collect()
    }

    /// The slot and stack of the filtering upgrade the panel shows.
    fn chosen_filter(&self) -> Option<(usize, &ItemStack)> {
        let slots = self.filtering_slots();
        let slot = *slots.get(self.state.filter_choice % slots.len().max(1))?;
        Some((slot, self.container?.contents.get(slot)?))
    }

    pub(super) fn next_filter(&mut self) {
        self.state.filter_choice = self.state.filter_choice.wrapping_add(1);
    }

    pub(super) fn storage_text(&self, id: &str) -> Option<String> {
        let tr = &self.content.translations;
        match id {
            "chest_rows" => {
                let first = self.state.chest_scroll + 1;
                let total = self.storage_rows();
                let last = (self.state.chest_scroll + CHEST_ROWS).min(total);
                let args = [
                    ("first", first.to_string()),
                    ("last", last.to_string()),
                    ("total", total.to_string()),
                ];
                Some(tr.text_with(self.language, "ui-storage-rows", &args))
            }
            "filter_title" => {
                let (_, stack) = self.chosen_filter()?;
                Some(self.content.item_name(stack.item, self.language))
            }
            "filter_mode" => {
                let (_, stack) = self.chosen_filter()?;
                let deny = filter_of(stack, &self.content.items).deny;
                let mode = if deny { "deny" } else { "allow" };
                let args = [("mode", mode.to_owned())];
                Some(tr.text_with(self.language, "ui-storage-filter-mode", &args))
            }
            _ => None,
        }
    }

    pub(super) fn filter_cells(&self) -> Vec<Option<ItemId>> {
        match self.chosen_filter() {
            Some((_, stack)) => filter_of(stack, &self.content.items).cells,
            None => vec![None; FILTER_CELLS],
        }
    }

    pub(super) fn filter_cell_tip(&self, index: usize) -> Option<String> {
        match self.filter_cells().get(index).copied().flatten() {
            Some(item) => Some(self.content.item_tooltip(item, self.language)),
            None => Some(
                self.content
                    .translations
                    .text(self.language, "ui-storage-filter-cell"),
            ),
        }
    }

    /// A click on a filter cell lists the carried item there, or empties it.
    pub(super) fn set_filter_cell(&mut self, index: usize, press: SlotPress) -> bool {
        let Some((slot, _)) = self.chosen_filter() else {
            return false;
        };
        if !matches!(press, SlotPress::Click(Click::Primary | Click::Secondary)) {
            return false;
        }
        let item = self.inventory.inventory().cursor.as_ref().map(|s| s.item);
        self.inventory.request(InventoryRequest::SetFilter {
            slot: slot as u16,
            cell: u8::try_from(index).ok(),
            item,
        });
        true
    }

    pub(super) fn toggle_filter_mode(&mut self) {
        if let Some((slot, _)) = self.chosen_filter() {
            self.inventory.request(InventoryRequest::SetFilter {
                slot: slot as u16,
                cell: None,
                item: None,
            });
        }
    }
}
