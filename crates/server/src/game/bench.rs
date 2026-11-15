//! The benches (the jeweller now): four cells, a recipe chosen in the list (which lays
//! its ingredients from the inventory), the result taken like a crafting result.

use super::Game;
use super::crafting::CraftWindow;
use crate::loader::ColumnLoader;
use std::sync::Arc;
use strata_core::crafting::{bench_result, fill_bench, take_bench};
use strata_core::id::Identifier;
use strata_core::inventory::{Click, ItemStack};
use strata_protocol::CraftingView;

impl Game {
    pub(super) fn bench_click(&mut self, loader: &ColumnLoader, click: Click) {
        let Some(recipe) = self.bench_recipe() else {
            return;
        };
        self.with_window(loader, |window, rules| {
            take_bench(window, &recipe, click, rules)
        });
    }

    /// The recipe chosen, if the open bench makes it: its ingredients are laid from the
    /// inventory when the cells do not hold them.
    pub(super) fn choose_bench_recipe(&mut self, loader: &ColumnLoader, choice: u16) {
        let benches = Arc::clone(&self.rules.benches);
        let Some(window) = self.ecs.crafts.get_mut(self.player) else {
            return;
        };
        let Some(recipe) = window
            .bench
            .as_ref()
            .and_then(|bench| benches.chosen(bench, choice))
        else {
            return;
        };
        window.cut = Some(choice);
        self.craft_changed();
        self.with_window(loader, |window, rules| fill_bench(window, recipe, rules));
    }

    fn bench_recipe(&self) -> Option<strata_core::crafting::BenchRecipe> {
        let window = self.ecs.crafts.get(self.player)?;
        let bench = window.bench.as_ref()?;
        self.rules.benches.chosen(bench, window.cut?).cloned()
    }

    pub(super) fn bench_view(&self, window: &CraftWindow, bench: &Identifier) -> CraftingView {
        let benches = &self.rules.benches;
        let recipe = window.cut.and_then(|choice| benches.chosen(bench, choice));
        let cuts = benches
            .of(bench)
            .map(|(index, recipe)| {
                (
                    index,
                    ItemStack::new(recipe.result, recipe.count, &self.rules),
                )
            })
            .collect();
        CraftingView {
            side: window.grid.side,
            grid: window.grid.slots.clone(),
            result: bench_result(recipe, &window.grid, &self.rules),
            hint: false,
            station: window.station,
            cuts,
            chosen: window.cut,
            cost: None,
        }
    }
}
