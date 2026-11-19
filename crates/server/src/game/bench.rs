//! The benches (the jeweller now): four cells, a recipe chosen in the list (which lays
//! its ingredients from the inventory), the result taken like a crafting result.

use super::Game;
use super::crafting::CraftWindow;
use crate::loader::ColumnLoader;
use std::collections::BTreeSet;
use std::sync::Arc;
use strata_core::content::RecipeId;
use strata_core::crafting::{bench_result, fill_bench, take_bench};
use strata_core::id::Identifier;
use strata_core::inventory::{Click, ItemStack};
use strata_protocol::CraftingView;

impl Game {
    pub(super) fn bench_click(&mut self, loader: &ColumnLoader, click: Click) {
        let Some(recipe) = self.bench_recipe() else {
            return;
        };
        let (_, thrown) = self.with_window(loader, |window, rules| {
            take_bench(window, &recipe, click, rules)
        });
        self.throw_all(thrown);
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
        if window.cut != Some(choice) {
            window.cut = Some(choice);
            self.craft_changed();
        }
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

    /// Grid recipes an old save knew that now live at a bench under the same name (the
    /// armour moved to the forge…): the bench's block is given once and its recipes
    /// learnt, so the world keeps a way to make them. Returns the other unknown names.
    pub fn adopt_moved_recipes(&mut self, unknown: Vec<Identifier>) -> Vec<Identifier> {
        let benches = Arc::clone(&self.rules.benches);
        let mut moved = BTreeSet::new();
        let left = (unknown.into_iter())
            .filter(|id| {
                let recipe = benches.list().iter().find(|recipe| recipe.id == *id);
                recipe
                    .map(|recipe| moved.insert(recipe.bench.clone()))
                    .is_none()
            })
            .collect();
        for bench in moved {
            self.give_bench(&bench);
        }
        left
    }

    fn give_bench(&mut self, bench: &Identifier) {
        let Some(item) = self.rules.bench_item(bench) else {
            return;
        };
        let recipes: Vec<RecipeId> = self.rules.recipes.making(item).map(|(id, _)| id).collect();
        if let Some(known) = self.ecs.recipes.get_mut(self.player) {
            known.extend(recipes);
        }
        self.recipes_changed();
        let stack = ItemStack::new(item, 1, &self.rules);
        let left = self.give(stack);
        self.throw_all(left);
    }
}
