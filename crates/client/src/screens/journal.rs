//! The journal: what the player has learnt. Its first section is the recipe book (only
//! the recipes learnt, a search, « can be made now »); while a crafting grid is open, a
//! second click on a recipe (or « Lay ») asks the server to lay it with the inventory's
//! items.

use super::menu::Effect;
use super::worlds::edit;
use crate::crafting::ClientCrafting;
use crate::inventory::ClientInventory;
use crate::ui::{ListRow, ScreenModel};
use std::collections::HashMap;
use strata_core::content::{Content, ItemId, RecipeId};
use strata_core::crafting::{Ingredient, MAX_GRID, Recipe, RecipeShape, can_make, counts};
use strata_core::inventory::{HOTBAR, MAIN};
use strata_protocol::InventoryRequest;

/// Rows of the recipe list visible at once (matches `journal.ron`).
pub const VISIBLE_ROWS: usize = 5;
const MAX_SEARCH: usize = 32;
/// Section titles; later milestones add objects studied, bestiary, places…
const SECTIONS: [&str; 1] = ["ui-journal-recipes"];

#[derive(Debug, Default)]
pub struct JournalState {
    pub section: usize,
    pub search: String,
    /// Only the recipes the inventory holds the ingredients of.
    pub craftable: bool,
    pub scroll: usize,
    pub selected: Option<RecipeId>,
    /// Opened from a crafting screen: a click lays the recipe in its grid.
    pub for_grid: bool,
}

pub struct JournalModel<'a> {
    pub state: &'a mut JournalState,
    pub inventory: Option<&'a mut ClientInventory>,
    pub crafting: Option<&'a ClientCrafting>,
    pub content: &'a Content,
    pub language: &'a str,
    pub effects: &'a mut Vec<Effect>,
}

impl JournalModel<'_> {
    fn tr(&self, key: &str) -> String {
        self.content.translations.text(self.language, key)
    }

    fn name(&self, item: ItemId) -> String {
        self.content.item_name(item, self.language)
    }

    /// What the player could use: the inventory and the grid (it goes back first).
    fn available(&self) -> HashMap<ItemId, u32> {
        let inventory = self.inventory.as_deref();
        let slots = HOTBAR
            .chain(MAIN)
            .filter_map(|index| inventory?.stack(index));
        let grid = self
            .crafting
            .into_iter()
            .flat_map(|c| c.view.grid.slots.iter().flatten());
        counts(slots.chain(grid))
    }

    /// Recipes learnt, matching the search and the filter, by name.
    pub fn recipes(&self) -> Vec<RecipeId> {
        let Some(crafting) = self.crafting else {
            return Vec::new();
        };
        let query = self.state.search.to_lowercase();
        let available = self.available();
        let mut found: Vec<(String, RecipeId)> = self
            .content
            .recipes
            .iter()
            .filter(|(id, _, _)| crafting.knows(*id))
            .filter(|(_, _, recipe)| !self.state.craftable || can_make(recipe, &available))
            .map(|(id, _, recipe)| (self.name(recipe.result), id))
            .filter(|(name, _)| query.is_empty() || name.to_lowercase().contains(&query))
            .collect();
        found.sort();
        found.into_iter().map(|(_, id)| id).collect()
    }

    fn ingredient_name(&self, ingredient: &Ingredient) -> String {
        match (&ingredient.tag, ingredient.items.first()) {
            (Some(tag), _) => self.tr(&tag.translation_key("tag")),
            (None, Some(&item)) => self.name(item),
            (None, None) => String::new(),
        }
    }

    /// « 4 × any planks, 1 × Stick », in the order of first use.
    fn ingredients_text(&self, recipe: &Recipe) -> String {
        let mut parts: Vec<(String, u32)> = Vec::new();
        for ingredient in recipe.ingredients() {
            let name = self.ingredient_name(ingredient);
            match parts.iter_mut().find(|(n, _)| *n == name) {
                Some((_, count)) => *count += 1,
                None => parts.push((name, 1)),
            }
        }
        let parts: Vec<String> = parts
            .into_iter()
            .map(|(name, count)| format!("{count} × {name}"))
            .collect();
        parts.join(", ")
    }

    fn row(&self, id: RecipeId, available: &HashMap<ItemId, u32>) -> Option<ListRow> {
        let recipe = self.content.recipes.get(id)?;
        let title = match recipe.count {
            1 => self.name(recipe.result),
            count => format!("{} ×{count}", self.name(recipe.result)),
        };
        let detail = match (recipe.grid_size() > 2, can_make(recipe, available)) {
            (true, _) => self.tr("ui-journal-needs-table"),
            (false, true) => self.tr("ui-journal-can-make"),
            (false, false) => self.tr("ui-journal-missing"),
        };
        Some(ListRow {
            title,
            detail,
            icon: None,
            item: Some(recipe.result),
            dim: !can_make(recipe, available),
        })
    }

    fn selected_recipe(&self) -> Option<&Recipe> {
        self.content.recipes.get(self.state.selected?)
    }

    /// The selected recipe laid in a 3×3 grid (a tag shows its first item).
    fn pattern(&self) -> Vec<Option<ItemId>> {
        let mut cells = vec![None; MAX_GRID * MAX_GRID];
        let Some(recipe) = self.selected_recipe() else {
            return cells;
        };
        let first = |ingredient: &Ingredient| ingredient.items.first().copied();
        match &recipe.shape {
            RecipeShape::Shaped {
                width,
                cells: pattern,
                ..
            } => {
                for (index, cell) in pattern.iter().enumerate() {
                    let at = (index / width) * MAX_GRID + index % width;
                    cells[at] = cell.as_ref().and_then(first);
                }
            }
            RecipeShape::Shapeless(list) => {
                for (cell, ingredient) in cells.iter_mut().zip(list) {
                    *cell = first(ingredient);
                }
            }
        }
        cells
    }

    fn fill(&mut self, all: bool) {
        let Some(recipe) = self.state.selected.filter(|_| self.state.for_grid) else {
            return;
        };
        if let Some(inventory) = self.inventory.as_deref_mut() {
            inventory.request(InventoryRequest::FillRecipe { recipe, all });
            self.effects.push(Effect::Pop);
        }
    }
}

impl ScreenModel for JournalModel<'_> {
    fn tabs(&self, _: &str) -> Vec<String> {
        SECTIONS.iter().map(|key| self.tr(key)).collect()
    }

    fn selected(&self, id: &str) -> Option<usize> {
        match id {
            "tabs" => Some(self.state.section),
            "list" => {
                let selected = self.state.selected?;
                let index = self.recipes().iter().position(|&r| r == selected)?;
                index
                    .checked_sub(self.state.scroll)
                    .filter(|&i| i < VISIBLE_ROWS)
            }
            _ => None,
        }
    }

    fn shown(&self, id: &str) -> bool {
        match id {
            "recipes_section" => self.state.section == 0,
            "fill_buttons" => self.state.for_grid && self.state.selected.is_some(),
            _ => true,
        }
    }

    fn rows(&self, _: &str) -> Vec<ListRow> {
        let available = self.available();
        self.recipes()
            .into_iter()
            .skip(self.state.scroll)
            .take(VISIBLE_ROWS)
            .filter_map(|id| self.row(id, &available))
            .collect()
    }

    fn items(&self, id: &str) -> Vec<Option<ItemId>> {
        match id {
            "pattern" => self.pattern(),
            "result" => vec![self.selected_recipe().map(|r| r.result)],
            _ => Vec::new(),
        }
    }

    fn text(&self, id: &str) -> Option<String> {
        match id {
            "search" => Some(self.state.search.clone()),
            "craftable" => Some(self.content.translations.text_with(
                self.language,
                "ui-journal-craftable",
                &[(
                    "only",
                    if self.state.craftable { "on" } else { "off" }.to_owned(),
                )],
            )),
            "detail" => Some(match self.selected_recipe() {
                Some(recipe) => self.ingredients_text(recipe),
                None if self.recipes().is_empty() => self.tr("ui-journal-empty"),
                None => self.tr("ui-journal-choose"),
            }),
            _ => None,
        }
    }

    fn tooltip(&self, id: &str, index: usize) -> Option<String> {
        let item = match id {
            "list" => {
                let recipe = *self.recipes().get(self.state.scroll + index)?;
                self.content.recipes.get(recipe)?.result
            }
            _ => self.items(id).get(index).copied().flatten()?,
        };
        Some(self.content.item_tooltip(item, self.language))
    }

    fn scroll(&mut self, _: &str, rows: i32) {
        let max = self.recipes().len().saturating_sub(VISIBLE_ROWS) as i32;
        self.state.scroll = (self.state.scroll as i32 + rows).clamp(0, max) as usize;
    }

    fn type_text(&mut self, id: &str, text: &str) {
        if id == "search" {
            edit(&mut self.state.search, text, MAX_SEARCH, |_| true);
            self.state.scroll = 0;
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        match id {
            "tabs" => self.state.section = index.min(SECTIONS.len() - 1),
            "craftable" => {
                self.state.craftable = !self.state.craftable;
                self.state.scroll = 0;
            }
            "list" => {
                let Some(&recipe) = self.recipes().get(self.state.scroll + index) else {
                    return;
                };
                if self.state.selected == Some(recipe) {
                    self.fill(false);
                }
                self.state.selected = Some(recipe);
            }
            "fill" => self.fill(false),
            "fill_all" => self.fill(true),
            "close" => self.effects.push(Effect::Pop),
            _ => {}
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "journal_tests.rs"]
mod tests;
