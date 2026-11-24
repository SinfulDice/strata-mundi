//! The lists of the work stations in the inventory screen: the stonecutter's variants,
//! the anvil's upgrades and the benches' recipes. Each takes clicks like slots, shows a
//! choice and tooltips. Also the stonecutter's and the furnace's notes.

use super::slots::SlotsModel;
use crate::ui::SlotPress;
use strata_core::content::ItemId;
use strata_core::inventory::{Click, ItemStack};
use strata_protocol::InventoryRequest;

impl SlotsModel<'_> {
    pub(super) fn list_items(&self, id: &str) -> Option<Vec<Option<ItemId>>> {
        match id {
            "cut_choices" => Some(self.cuts().map(|(_, stack)| Some(stack.item)).collect()),
            "anvil_choices" => Some(self.anvil_icons()),
            "bench_choices" => Some(self.bench_icons()),
            "chest_filter" => Some(self.filter_cells()),
            _ => None,
        }
    }

    pub(super) fn list_selected(&self, id: &str) -> Option<usize> {
        match id {
            "cut_choices" => {
                let chosen = self.crafting?.view.chosen?;
                self.cuts().position(|(index, _)| *index == chosen)
            }
            "anvil_choices" => self.anvil_selected(),
            "bench_choices" => self.bench_selected(),
            _ => None,
        }
    }

    /// `Some` for a list (its tooltip, if any), `None` for anything else.
    pub(super) fn list_tooltip(&self, id: &str, index: usize) -> Option<Option<String>> {
        match id {
            "cut_choices" => {
                let tooltip = self.cuts().nth(index);
                Some(tooltip.map(|(_, s)| self.content.item_tooltip(s.item, self.language)))
            }
            "anvil_choices" => Some(self.anvil_tooltip(index)),
            "bench_choices" => Some(self.bench_tooltip(index)),
            "chest_filter" => Some(self.filter_cell_tip(index)),
            "filter_mode" => Some(Some(self.filter_mode_tip())),
            "anvil" => self.anvil_cell_tip(index).map(Some),
            "accessories" => self.accessory_slot_tip(index).map(Some),
            "back" => self.back_slot_tip().map(Some),
            _ => None,
        }
    }

    pub(super) fn list_press(&mut self, id: &str, index: usize, press: SlotPress) -> Option<bool> {
        match id {
            "cut_choices" => Some(self.choose_cut(index, press)),
            "anvil_choices" => Some(self.choose_upgrade(index, press)),
            "bench_choices" => Some(self.choose_bench_recipe(index, press)),
            "chest_filter" => Some(self.set_filter_cell(index, press)),
            _ => None,
        }
    }

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

    pub(super) fn cutter_note(&self) -> String {
        let laid = self.crafting.is_some_and(|c| c.view.grid.get(0).is_some());
        let key = if laid && self.cuts().next().is_none() {
            "ui-stonecutter-none"
        } else {
            "ui-stonecutter-help"
        };
        self.content.translations.text(self.language, key)
    }

    /// The open furnace's speed and economy, for a usual cooking and the reference fuel.
    pub(super) fn furnace_info(&self) -> String {
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
