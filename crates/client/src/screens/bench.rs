//! A bench's side of the inventory screen (the jeweller): its four cells, the list of
//! what it makes (choosing lays the ingredients from the inventory), the result.

use super::slots::SlotsModel;
use crate::ui::SlotPress;
use strata_core::content::ItemId;
use strata_core::crafting::BenchRecipe;
use strata_core::inventory::{Click, HOTBAR, MAIN};
use strata_protocol::InventoryRequest;

impl SlotsModel<'_> {
    /// The recipes of the open bench, in the server's order.
    fn bench_recipes(&self) -> impl Iterator<Item = (u16, &BenchRecipe)> {
        let benches = &self.content.benches;
        self.crafting
            .into_iter()
            .flat_map(|c| c.view.cuts.iter())
            .filter_map(|(index, _)| Some((*index, benches.get(*index)?)))
    }

    pub(super) fn bench_icons(&self) -> Vec<Option<ItemId>> {
        self.bench_recipes().map(|(_, r)| Some(r.result)).collect()
    }

    pub(super) fn bench_selected(&self) -> Option<usize> {
        let chosen = self.crafting?.view.chosen?;
        self.bench_recipes().position(|(index, _)| index == chosen)
    }

    /// The result's tooltip, then what it takes and whether the inventory has it.
    pub(super) fn bench_tooltip(&self, index: usize) -> Option<String> {
        let (_, recipe) = self.bench_recipes().nth(index)?;
        let tr = &self.content.translations;
        let mut text = self.content.item_tooltip(recipe.result, self.language);
        for (ingredient, count) in &recipe.ingredients {
            let name = ingredient
                .tag
                .as_ref()
                .map(|tag| tr.text(self.language, &tag.translation_key("tag")))
                .or_else(|| {
                    let first = ingredient.items().first()?;
                    Some(self.content.item_name(*first, self.language))
                })
                .unwrap_or_default();
            let args = [("count", count.to_string()), ("item", name)];
            text.push('\n');
            text.push_str(&tr.text_with(self.language, "ui-bench-ingredient", &args));
        }
        if !self.has_ingredients(recipe) {
            text.push('\n');
            text.push_str(&tr.text(self.language, "ui-bench-missing"));
        }
        Some(text)
    }

    /// The inventory and the cells together hold what one craft takes.
    fn has_ingredients(&self, recipe: &BenchRecipe) -> bool {
        let inventory = self.inventory.inventory();
        let cells = self.crafting.map(|c| &c.view.grid);
        let held = HOTBAR
            .chain(MAIN)
            .filter_map(|index| inventory.slots.get(index))
            .chain(
                cells
                    .into_iter()
                    .flat_map(|grid| grid.slots.iter().flatten()),
            );
        let stacks: Vec<_> = held.collect();
        recipe.ingredients.iter().all(|(ingredient, count)| {
            let total: u32 = stacks
                .iter()
                .filter(|stack| ingredient.accepts(stack.item))
                .map(|stack| u32::from(stack.count))
                .sum();
            total >= u32::from(*count)
        })
    }

    pub(super) fn choose_bench_recipe(&mut self, index: usize, press: SlotPress) -> bool {
        let Some((choice, _)) = self.bench_recipes().nth(index) else {
            return false;
        };
        if matches!(press, SlotPress::Click(Click::Primary | Click::Transfer)) {
            self.inventory.request(InventoryRequest::ChooseCut(choice));
        }
        true
    }

    pub(super) fn bench_note(&self) -> String {
        let tr = &self.content.translations;
        let Some(view) = self.crafting.map(|c| &c.view) else {
            return String::new();
        };
        let key = match (view.chosen, view.result.is_some()) {
            (Some(_), false) => "ui-bench-missing",
            _ => "ui-bench-help",
        };
        tr.text(self.language, key)
    }
}
