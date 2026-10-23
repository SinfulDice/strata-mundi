use crate::hotbar::{Hotbar, SLOTS};
use crate::ui::ScreenModel;
use strata_core::content::{Content, ItemId};
use strata_core::id::Identifier;

const COLUMNS: usize = 9;
const ROWS: usize = 5;
/// The only interface text chosen in code rather than in a screen file.
pub const TAB_ALL_KEY: &str = "ui-tab-all";

/// Creative inventory: every item, filtered by category tab and by search text.
pub struct InventoryModel<'a> {
    pub state: &'a mut InventoryState,
    pub hotbar: &'a mut Hotbar,
    pub content: &'a Content,
    pub language: &'a str,
}

/// What the inventory remembers between frames, including the filtered list (rebuilt
/// only when the tab or the search changes).
#[derive(Debug, Default)]
pub struct InventoryState {
    pub tab: usize,
    pub search: String,
    pub scroll_rows: usize,
    pub search_focused: bool,
    categories: Vec<Identifier>,
    items: Vec<ItemId>,
    items_for: Option<(usize, String)>,
}

impl<'a> InventoryModel<'a> {
    pub fn new(
        state: &'a mut InventoryState,
        hotbar: &'a mut Hotbar,
        content: &'a Content,
        language: &'a str,
    ) -> Self {
        let mut model = Self {
            state,
            hotbar,
            content,
            language,
        };
        model.refresh();
        model
    }

    fn refresh(&mut self) {
        let key = (self.state.tab, self.state.search.clone());
        if self.state.items_for.as_ref() == Some(&key) {
            return;
        }
        if self.state.categories.is_empty() {
            self.state.categories = categories(self.content);
        }
        self.state.items = self.filter();
        self.state.items_for = Some(key);
    }

    fn name(&self, item: ItemId) -> String {
        let Some(id) = self.content.items.identifier(item) else {
            return String::new();
        };
        let kind = match self.content.items.get(item).map(|i| i.kind) {
            Some(strata_core::content::ItemKind::Block(_)) => "block",
            _ => "item",
        };
        self.content
            .translations
            .text(self.language, &id.translation_key(kind))
    }

    /// Items of the current tab matching the search, in registry order.
    fn filter(&self) -> Vec<ItemId> {
        let category = self
            .state
            .tab
            .checked_sub(1)
            .and_then(|i| self.state.categories.get(i).cloned());
        let query = self.state.search.to_lowercase();
        self.content
            .items
            .iter()
            .filter(|(_, _, item)| category.is_none() || item.category == category)
            .filter(|(_, _, item)| item.category.is_some())
            .map(|(id, _, _)| id)
            .filter(|&id| query.is_empty() || self.name(id).to_lowercase().contains(&query))
            .collect()
    }

    /// "All" plus one tab per category.
    pub fn tab_count(&self) -> usize {
        1 + self.state.categories.len()
    }

    /// Scrolls the grid by whole rows, within the item list.
    pub fn scroll(&mut self, rows: i32) {
        let total_rows = self.state.items.len().div_ceil(COLUMNS);
        let max = total_rows.saturating_sub(ROWS) as i32;
        self.state.scroll_rows = (self.state.scroll_rows as i32 + rows).clamp(0, max) as usize;
    }

    /// Item shown in grid cell `index`, if any.
    pub fn grid_item(&self, index: usize) -> Option<ItemId> {
        self.state
            .items
            .get(self.state.scroll_rows * COLUMNS + index)
            .copied()
    }

    pub fn type_text(&mut self, text: &str) {
        for ch in text.chars() {
            match ch {
                '\u{8}' => {
                    self.state.search.pop();
                }
                c if !c.is_control() && self.state.search.chars().count() < 32 => {
                    self.state.search.push(c)
                }
                _ => {}
            }
        }
        self.state.scroll_rows = 0;
        self.refresh();
    }
}

fn categories(content: &Content) -> Vec<Identifier> {
    let mut categories: Vec<Identifier> = content
        .items
        .iter()
        .filter_map(|(_, _, item)| item.category.clone())
        .collect();
    categories.sort();
    categories.dedup();
    categories
}

impl ScreenModel for InventoryModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        (id == "search").then(|| self.state.search.clone())
    }

    fn items(&self, id: &str) -> Vec<Option<ItemId>> {
        match id {
            "items" => {
                let items = &self.state.items;
                let start = self.state.scroll_rows * COLUMNS;
                (start..start + COLUMNS * ROWS)
                    .map(|i| items.get(i).copied())
                    .collect()
            }
            "hotbar" => (0..SLOTS).map(|i| self.hotbar.slot(i)).collect(),
            _ => Vec::new(),
        }
    }

    fn selected(&self, id: &str) -> Option<usize> {
        match id {
            "tabs" => Some(self.state.tab),
            "hotbar" => Some(self.hotbar.selected_index()),
            _ => None,
        }
    }

    fn tabs(&self, _: &str) -> Vec<String> {
        let tr = |key: &str| self.content.translations.text(self.language, key);
        std::iter::once(tr(TAB_ALL_KEY))
            .chain(
                self.state
                    .categories
                    .iter()
                    .map(|c| tr(&c.translation_key("category"))),
            )
            .collect()
    }

    fn tooltip(&self, id: &str, index: usize) -> Option<String> {
        match id {
            "items" => self.grid_item(index).map(|item| self.name(item)),
            "hotbar" => self.hotbar.slot(index).map(|item| self.name(item)),
            _ => None,
        }
    }

    fn activate(&mut self, id: &str, index: usize) {
        match id {
            "items" => {
                if let Some(item) = self.grid_item(index) {
                    self.hotbar.set(self.hotbar.selected_index(), Some(item));
                }
            }
            "hotbar" => self.hotbar.select(index),
            "tabs" => {
                self.state.tab = index;
                self.state.scroll_rows = 0;
                self.refresh();
            }
            "search" => self.state.search_focused = true,
            _ => {}
        }
    }
}
