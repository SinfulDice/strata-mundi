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
use strata_core::crafting::{Ingredient, MAX_GRID, Recipe, can_make, counts};
use strata_core::inventory::{HOTBAR, MAIN};
use strata_protocol::InventoryRequest;

/// Rows of the recipe list visible at once (matches `journal.ron`).
pub const VISIBLE_ROWS: usize = 5;
const MAX_SEARCH: usize = 32;

/// The tabs of the journal; later milestones add objects studied, bestiary, places…
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum JournalSection {
    #[default]
    Recipes,
}

impl JournalSection {
    pub const ALL: [Self; 1] = [Self::Recipes];

    fn title_key(self) -> &'static str {
        match self {
            Self::Recipes => "ui-journal-recipes",
        }
    }
}

#[derive(Debug, Default)]
pub struct JournalState {
    pub section: JournalSection,
    pub search: String,
    /// Only the recipes the inventory holds the ingredients of.
    pub craftable: bool,
    pub scroll: usize,
    pub selected: Option<RecipeId>,
    /// Opened from a crafting screen: a click lays the recipe in its grid.
    pub for_grid: bool,
    list: ListCache,
}

impl JournalState {
    /// Opened from a crafting screen.
    pub fn for_grid() -> Self {
        Self {
            for_grid: true,
            ..Self::default()
        }
    }
}

/// The filtered list, rebuilt only when what it depends on changes.
#[derive(Debug, Default)]
struct ListCache {
    key: Option<ListKey>,
    /// Recipes shown, by name, each with whether it can be made now.
    entries: Vec<(RecipeId, bool)>,
}

#[derive(Debug, Clone, PartialEq)]
struct ListKey {
    search: String,
    craftable: bool,
    language: String,
    /// Items the player could use, sorted.
    available: Vec<(ItemId, u32)>,
    known: (usize, bool),
}

pub struct JournalModel<'a> {
    state: &'a mut JournalState,
    inventory: Option<&'a mut ClientInventory>,
    crafting: Option<&'a ClientCrafting>,
    content: &'a Content,
    language: &'a str,
    effects: &'a mut Vec<Effect>,
}

impl<'a> JournalModel<'a> {
    pub fn new(
        state: &'a mut JournalState,
        inventory: Option<&'a mut ClientInventory>,
        crafting: Option<&'a ClientCrafting>,
        content: &'a Content,
        language: &'a str,
        effects: &'a mut Vec<Effect>,
    ) -> Self {
        let mut model = Self {
            state,
            inventory,
            crafting,
            content,
            language,
            effects,
        };
        model.refresh();
        model
    }

    fn tr(&self, key: &str) -> String {
        self.content.translations.text(self.language, key)
    }

    fn tr_with(&self, key: &str, args: &[(&str, String)]) -> String {
        self.content
            .translations
            .text_with(self.language, key, args)
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

    fn refresh(&mut self) {
        let Some(crafting) = self.crafting else {
            self.state.list = ListCache::default();
            return;
        };
        let mut available = self.available();
        let mut sorted: Vec<(ItemId, u32)> = available.iter().map(|(&i, &c)| (i, c)).collect();
        sorted.sort();
        let key = ListKey {
            search: self.state.search.clone(),
            craftable: self.state.craftable,
            language: self.language.to_owned(),
            available: sorted,
            known: (crafting.known_count(), crafting.all_known),
        };
        if self.state.list.key.as_ref() == Some(&key) {
            return;
        }
        let query = key.search.to_lowercase();
        let mut found: Vec<(String, RecipeId, bool)> = Vec::new();
        for (id, _, recipe) in self.content.recipes.iter() {
            if !crafting.knows(id) {
                continue;
            }
            let name = self.name(recipe.result);
            if !query.is_empty() && !name.to_lowercase().contains(&query) {
                continue;
            }
            let makeable = can_make(recipe, &mut available);
            if makeable || !self.state.craftable {
                found.push((name, id, makeable));
            }
        }
        found.sort();
        self.state.list = ListCache {
            key: Some(key),
            entries: found.into_iter().map(|(_, id, can)| (id, can)).collect(),
        };
        let max = self.state.list.entries.len().saturating_sub(VISIBLE_ROWS);
        self.state.scroll = self.state.scroll.min(max);
    }

    fn ingredient_name(&self, ingredient: &Ingredient) -> String {
        match (&ingredient.tag, ingredient.items().first()) {
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
            .map(|(name, count)| {
                self.tr_with(
                    "ui-journal-ingredient",
                    &[("count", count.to_string()), ("name", name)],
                )
            })
            .collect();
        parts.join(&self.tr("ui-journal-separator"))
    }

    /// Side of the grid the recipes go into (the inventory's unless a table is open).
    fn grid_side(&self) -> usize {
        self.crafting.map_or(
            usize::from(strata_core::inventory::CraftGrid::INVENTORY),
            |c| usize::from(c.view.side),
        )
    }

    fn row(&self, id: RecipeId, makeable: bool) -> Option<ListRow> {
        let recipe = self.content.recipes.get(id)?;
        let title = match recipe.count {
            1 => self.name(recipe.result),
            count => self.tr_with(
                "ui-journal-result",
                &[
                    ("item", self.name(recipe.result)),
                    ("count", count.to_string()),
                ],
            ),
        };
        let detail = match (recipe.grid_size() > self.grid_side(), makeable) {
            (true, _) => self.tr("ui-journal-needs-table"),
            (false, true) => self.tr("ui-journal-can-make"),
            (false, false) => self.tr("ui-journal-missing"),
        };
        Some(ListRow {
            title,
            detail,
            icon: None,
            item: Some(recipe.result),
            dim: !makeable,
        })
    }

    fn selected_recipe(&self) -> Option<&Recipe> {
        self.content.recipes.get(self.state.selected?)
    }

    /// The selected recipe laid in a 3×3 grid (a tag shows its first item).
    fn pattern(&self) -> Vec<Option<ItemId>> {
        let mut cells = vec![None; MAX_GRID * MAX_GRID];
        if let Some(recipe) = self.selected_recipe() {
            for (cell, ingredient) in recipe.cells_in(MAX_GRID) {
                cells[cell] = ingredient.items().first().copied();
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
        JournalSection::ALL
            .iter()
            .map(|section| self.tr(section.title_key()))
            .collect()
    }

    fn selected(&self, id: &str) -> Option<usize> {
        match id {
            "tabs" => JournalSection::ALL
                .iter()
                .position(|&s| s == self.state.section),
            "list" => {
                let selected = self.state.selected?;
                let entries = &self.state.list.entries;
                let index = entries.iter().position(|&(r, _)| r == selected)?;
                index
                    .checked_sub(self.state.scroll)
                    .filter(|&i| i < VISIBLE_ROWS)
            }
            _ => None,
        }
    }

    fn shown(&self, id: &str) -> bool {
        match id {
            "recipes_section" => self.state.section == JournalSection::Recipes,
            "fill_buttons" => self.state.for_grid && self.state.selected.is_some(),
            _ => true,
        }
    }

    fn rows(&self, _: &str) -> Vec<ListRow> {
        self.state
            .list
            .entries
            .iter()
            .skip(self.state.scroll)
            .take(VISIBLE_ROWS)
            .filter_map(|&(id, makeable)| self.row(id, makeable))
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
            "craftable" => Some(self.tr_with(
                "ui-journal-craftable",
                &[(
                    "only",
                    if self.state.craftable { "on" } else { "off" }.to_owned(),
                )],
            )),
            "detail" => Some(match self.selected_recipe() {
                Some(recipe) => self.ingredients_text(recipe),
                None if self.state.list.entries.is_empty() => self.tr("ui-journal-empty"),
                None => self.tr("ui-journal-choose"),
            }),
            _ => None,
        }
    }

    fn tooltip(&self, id: &str, index: usize) -> Option<String> {
        let item = match id {
            "list" => {
                let &(recipe, _) = self.state.list.entries.get(self.state.scroll + index)?;
                self.content.recipes.get(recipe)?.result
            }
            _ => self.items(id).get(index).copied().flatten()?,
        };
        Some(self.content.item_tooltip(item, self.language))
    }

    fn scroll(&mut self, _: &str, rows: i32) {
        let max = self.state.list.entries.len().saturating_sub(VISIBLE_ROWS) as i32;
        self.state.scroll = (self.state.scroll as i32 + rows).clamp(0, max) as usize;
    }

    fn type_text(&mut self, id: &str, text: &str) {
        if id == "search" {
            edit(&mut self.state.search, text, MAX_SEARCH, |_| true);
            self.state.scroll = 0;
            self.refresh();
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        match id {
            "tabs" => {
                if let Some(&section) = JournalSection::ALL.get(index) {
                    self.state.section = section;
                }
            }
            "craftable" => {
                self.state.craftable = !self.state.craftable;
                self.state.scroll = 0;
                self.refresh();
            }
            "list" => {
                let Some(&(recipe, _)) = self.state.list.entries.get(self.state.scroll + index)
                else {
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
