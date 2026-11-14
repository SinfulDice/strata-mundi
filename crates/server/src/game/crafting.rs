//! Crafting: the player's grid (2×2, 3×3 at a crafting table, the single cell of the
//! stonecutter), the result, the recipes learnt by crafting them and the recipe book
//! filling the grid.

use super::{Game, MAX_REACH};
use crate::loader::ColumnLoader;
use std::collections::BTreeSet;
use strata_core::content::{BlockId, RecipeId, StationDef};
use strata_core::cooking::{cut_choices, cut_result, take_cut};
use strata_core::coords::BlockPos;
use strata_core::crafting::{GridItems, fill, grid_result, take_result};
use strata_core::inventory::{Click, CraftGrid, ItemStack};
use strata_protocol::{CraftingView, ServerMessage};

/// The grid in front of a player and the work station it belongs to, if any.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct CraftWindow {
    pub grid: CraftGrid,
    pub station: Option<(BlockId, BlockPos)>,
    /// At a stonecutter: the cutting chosen (an index in `Cooking::cuttings`).
    pub cut: Option<u16>,
    pub stonecutter: bool,
}

impl Game {
    /// Right click on a work station: its grid replaces the inventory's (whose stacks go
    /// back first, as when a screen closes).
    pub(super) fn open_station(
        &mut self,
        loader: &ColumnLoader,
        pos: BlockPos,
        block: BlockId,
        station: StationDef,
    ) -> bool {
        if !self.in_reach(pos) || !self.take_action() {
            return false;
        }
        self.close_window(loader);
        let (side, stonecutter) = match station {
            StationDef::Crafting(side) => (side, false),
            StationDef::Stonecutter => (1, true),
        };
        let window = CraftWindow {
            grid: CraftGrid::new(side),
            station: Some((block, pos)),
            cut: None,
            stonecutter,
        };
        self.ecs.crafts.insert(self.player, window);
        self.craft_changed();
        true
    }

    /// Whether `recipe` counts as known: learnt, or every recipe is (creative, option).
    pub fn knows(&self, recipe: RecipeId) -> bool {
        self.all_recipes_known()
            || self
                .ecs
                .recipes
                .get(self.player)
                .is_some_and(|known| known.contains(&recipe))
    }

    pub fn all_recipes_known(&self) -> bool {
        !self.discover_recipes || self.is_creative()
    }

    /// World option: recipes are learnt by playing.
    pub fn set_recipe_discovery(&mut self, discover: bool) {
        self.discover_recipes = discover;
        self.recipes_changed();
    }

    /// The recipes learnt, as saved; the starting ones are always added.
    pub fn set_known_recipes(&mut self, mut known: BTreeSet<RecipeId>) {
        known.extend(self.rules.recipes.starting());
        self.ecs.recipes.insert(self.player, known);
        self.recipes_changed();
    }

    pub fn known_recipes(&self) -> Vec<RecipeId> {
        self.ecs
            .recipes
            .get(self.player)
            .map(|known| known.iter().copied().collect())
            .unwrap_or_default()
    }

    pub(super) fn recipes_changed(&mut self) {
        self.messages.push(ServerMessage::Recipes {
            known: self.known_recipes(),
            all: self.all_recipes_known(),
        });
    }

    /// A recipe crafted: learnt if it was not (no penalty for knowing the game). Nothing
    /// is learnt while every recipe counts as known.
    fn learn(&mut self, recipe: RecipeId) {
        if self.all_recipes_known() {
            return;
        }
        let Some(known) = self.ecs.recipes.get_mut(self.player) else {
            return;
        };
        if known.insert(recipe) {
            self.messages.push(ServerMessage::RecipeLearned(recipe));
        }
    }

    /// What the crafting grid holds (saved with the inventory).
    pub fn grid_stacks(&self) -> Vec<ItemStack> {
        self.ecs
            .crafts
            .get(self.player)
            .map(|window| window.grid.slots.slots.iter().flatten().cloned().collect())
            .unwrap_or_default()
    }

    /// Stacks of a saved grid go back to the inventory (or to the ground).
    pub fn give_back(&mut self, stacks: Vec<ItemStack>) {
        let left: Vec<_> = stacks.into_iter().filter_map(|s| self.give(s)).collect();
        self.throw_all(left);
    }

    /// A click on the result slot.
    pub(super) fn craft_click(&mut self, loader: &ColumnLoader, click: Click) {
        if let Some(choice) = self.stonecutter_choice() {
            let cooking = std::sync::Arc::clone(&self.rules.cooking);
            self.with_window(loader, |window, rules| {
                take_cut(window, &cooking, choice, click, rules)
            });
            return;
        }
        let recipes = std::sync::Arc::clone(&self.rules.recipes);
        let crafted = self.with_window(loader, |window, rules| {
            take_result(window, &recipes, click, rules)
        });
        if let Some(crafted) = crafted {
            self.learn(crafted.recipe);
            self.throw_all(crafted.thrown);
        }
    }

    /// The recipe book: lays a known recipe with the inventory's items.
    pub(super) fn fill_recipe(&mut self, loader: &ColumnLoader, recipe: RecipeId, all: bool) {
        if !self.knows(recipe) || self.stonecutter_choice().is_some() {
            return;
        }
        let recipes = std::sync::Arc::clone(&self.rules.recipes);
        let Some(recipe) = recipes.get(recipe) else {
            return;
        };
        self.with_window(loader, |window, rules| fill(window, recipe, all, rules));
    }

    /// The station gone or out of reach: its grid gives its stacks back and the
    /// inventory's grid comes back.
    pub(super) fn check_station(&mut self, loader: &ColumnLoader) {
        let station = self.ecs.crafts.get(self.player).and_then(|c| c.station);
        let Some((block, pos)) = station else {
            return;
        };
        if loader.block(pos) != Some(block) || !self.within(pos, MAX_REACH + 1.0) {
            self.reset_craft();
        }
    }

    /// Back to the inventory's empty 2×2 grid; what the old one held goes to the
    /// inventory (or the ground).
    pub(super) fn reset_craft(&mut self) {
        let old = self.ecs.crafts.remove(self.player);
        self.ecs.crafts.insert(self.player, CraftWindow::default());
        if old.as_ref().is_some_and(|w| *w != CraftWindow::default()) {
            self.craft_changed();
        }
        let stacks = old.map(|mut w| w.grid.slots.drain()).unwrap_or_default();
        let left: Vec<_> = stacks.into_iter().filter_map(|s| self.give(s)).collect();
        self.throw_all(left);
    }

    pub(super) fn craft_changed(&mut self) {
        self.craft_dirty = true;
    }

    /// At a stonecutter, the cutting chosen (`Some(None)` when none is).
    fn stonecutter_choice(&self) -> Option<Option<u16>> {
        let window = self.ecs.crafts.get(self.player)?;
        window.stonecutter.then_some(window.cut)
    }

    /// The stonecutter: the cutting chosen for what lies in its cell.
    pub(super) fn choose_cut(&mut self, choice: u16) {
        let cooking = std::sync::Arc::clone(&self.rules.cooking);
        let Some(window) = self.ecs.crafts.get_mut(self.player) else {
            return;
        };
        let valid = window.stonecutter
            && cut_choices(&cooking, &window.grid)
                .iter()
                .any(|(index, _)| *index == choice);
        if valid {
            window.cut = Some(choice);
            self.craft_changed();
        }
    }

    pub(super) fn crafting_view(&self) -> Option<CraftingView> {
        let window = self.ecs.crafts.get(self.player)?;
        if window.stonecutter {
            return Some(self.cutting_view(window));
        }
        let recipes = &self.rules.recipes;
        let items = window.grid.items();
        let grid = GridItems {
            side: usize::from(window.grid.side),
            cells: &items,
        };
        Some(CraftingView {
            side: window.grid.side,
            grid: window.grid.slots.clone(),
            result: grid_result(recipes, &window.grid, &self.rules),
            hint: !self.all_recipes_known() && recipes.hint(grid, |r| self.knows(r)),
            station: window.station,
            cuts: Vec::new(),
            chosen: None,
        })
    }

    fn cutting_view(&self, window: &CraftWindow) -> CraftingView {
        let cooking = &self.rules.cooking;
        let cuts = cut_choices(cooking, &window.grid)
            .into_iter()
            .map(|(index, recipe)| {
                let stack = ItemStack::new(recipe.result, recipe.count, &self.rules);
                (index, stack)
            })
            .collect();
        CraftingView {
            side: window.grid.side,
            grid: window.grid.slots.clone(),
            result: cut_result(cooking, &window.grid, window.cut, &self.rules),
            hint: false,
            station: window.station,
            cuts,
            chosen: window.cut,
        }
    }
}
