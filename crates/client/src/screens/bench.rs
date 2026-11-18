//! A bench's side of the inventory screen (forge, tannery, loom, jeweller): its four
//! cells, the list of what it makes (choosing lays the ingredients from the inventory),
//! the result; each bench in its own arrangement and colour (`benches/`); and what goes
//! in each empty accessory slot.

use super::slots::SlotsModel;
use crate::ui::SlotPress;
use strata_core::content::{EquipSlot, ItemId, StationDef};
use strata_core::crafting::{BENCH_SIDE, BenchRecipe, BenchStyle, can_lay};
use strata_core::id::Identifier;
use strata_core::inventory::{ACCESSORIES, Click, CraftGrid, slot_equip};
use strata_core::texture::Rgba;
use strata_protocol::InventoryRequest;

/// The list shows this many recipes a row, and this many rows, in every arrangement.
pub(super) const BENCH_COLUMNS: usize = 5;
pub(super) const BENCH_ROWS: usize = 3;
/// Group id of an arrangement: `bench_layout_<name>`.
pub(super) const LAYOUT_PREFIX: &str = "bench_layout_";

impl SlotsModel<'_> {
    /// The open bench, if a bench is open.
    fn bench(&self) -> Option<Identifier> {
        match self.station()? {
            StationDef::Bench(bench) => Some(bench),
            _ => None,
        }
    }

    fn bench_style(&self) -> Option<BenchStyle> {
        Some(self.content.benches.style(&self.bench()?))
    }

    /// The recipes of the open bench, in the server's order.
    fn bench_recipes(&self) -> impl Iterator<Item = (u16, &BenchRecipe)> {
        let benches = &self.content.benches;
        self.crafting
            .into_iter()
            .flat_map(|c| c.view.cuts.iter())
            .filter_map(|(index, _)| Some((*index, benches.get(*index)?)))
    }

    /// The first recipe shown: the list scrolls by rows when it has more than it shows.
    fn bench_first(&self) -> usize {
        self.state.bench_scroll.min(self.bench_most_rows()) * BENCH_COLUMNS
    }

    fn bench_most_rows(&self) -> usize {
        let count = self.bench_recipes().count();
        count.div_ceil(BENCH_COLUMNS).saturating_sub(BENCH_ROWS)
    }

    /// Recipe `index` of the page shown.
    fn bench_shown(&self, index: usize) -> Option<(u16, &BenchRecipe)> {
        (index < BENCH_COLUMNS * BENCH_ROWS)
            .then(|| self.bench_recipes().nth(self.bench_first() + index))
            .flatten()
    }

    pub(super) fn bench_icons(&self) -> Vec<Option<ItemId>> {
        let page = self.bench_recipes().skip(self.bench_first());
        page.take(BENCH_COLUMNS * BENCH_ROWS)
            .map(|(_, r)| Some(r.result))
            .collect()
    }

    pub(super) fn bench_selected(&self) -> Option<usize> {
        let chosen = self.crafting?.view.chosen?;
        let at = self
            .bench_recipes()
            .position(|(index, _)| index == chosen)?;
        at.checked_sub(self.bench_first())
            .filter(|&shown| shown < BENCH_COLUMNS * BENCH_ROWS)
    }

    pub(super) fn scroll_bench(&mut self, rows: i32) {
        let most = self.bench_most_rows();
        let now = self.state.bench_scroll.min(most);
        self.state.bench_scroll = now.saturating_add_signed(rows as isize).min(most);
    }

    /// Whether a bench group is drawn: its side, its own arrangement, the scroll buttons.
    pub(super) fn bench_group_shown(&self, id: &str) -> Option<bool> {
        if let Some(layout) = id.strip_prefix(LAYOUT_PREFIX) {
            let open = self.bench_style().map(|style| style.layout.name());
            return Some(open == Some(layout));
        }
        match id {
            "bench_side" => Some(self.container.is_none() && self.bench().is_some()),
            "bench_scroll" => Some(self.bench_most_rows() > 0),
            _ => None,
        }
    }

    /// The bench's colour, for its bands.
    pub(super) fn bench_color(&self, id: &str) -> Option<Rgba> {
        id.starts_with("bench_band")
            .then(|| self.bench_style()?.accent)
            .flatten()
            .map(|color| color.0)
    }

    /// The result's tooltip, then what it takes and whether the inventory has it.
    pub(super) fn bench_tooltip(&self, index: usize) -> Option<String> {
        let (_, recipe) = self.bench_shown(index)?;
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

    /// The cells hold what one craft takes, or choosing the recipe would lay it (the
    /// server's own check).
    fn has_ingredients(&self, recipe: &BenchRecipe) -> bool {
        let Some(grid) = self.crafting.map(|c| &c.view.grid) else {
            return false;
        };
        let grid = CraftGrid {
            side: BENCH_SIDE,
            slots: grid.clone(),
        };
        can_lay(
            self.inventory.inventory(),
            &grid,
            recipe,
            &self.content.items,
        )
    }

    pub(super) fn choose_bench_recipe(&mut self, index: usize, press: SlotPress) -> bool {
        let Some((choice, _)) = self.bench_shown(index) else {
            return false;
        };
        if matches!(press, SlotPress::Click(Click::Primary | Click::Transfer)) {
            self.inventory.request(InventoryRequest::ChooseCut(choice));
        }
        true
    }

    /// What goes in an empty accessory slot (« Ring »…).
    pub(super) fn accessory_slot_tip(&self, index: usize) -> Option<String> {
        let slot = ACCESSORIES.start + index;
        if self.inventory.stack(slot).is_some() {
            return None;
        }
        let key = match slot_equip(slot)? {
            EquipSlot::Ring => "ui-inventory-ring",
            EquipSlot::Amulet => "ui-inventory-amulet",
            EquipSlot::Belt => "ui-inventory-belt",
            EquipSlot::Cloak => "ui-inventory-cloak",
            _ => return None,
        };
        Some(self.content.translations.text(self.language, key))
    }

    /// Ingredients missing, or the bench's own help (`bench-<namespace>-<name>`), or the
    /// usual help.
    pub(super) fn bench_note(&self) -> String {
        let tr = &self.content.translations;
        let Some(view) = self.crafting.map(|c| &c.view) else {
            return String::new();
        };
        if view.chosen.is_some() && view.result.is_none() {
            return tr.text(self.language, "ui-bench-missing");
        }
        let own = self.bench().map(|bench| bench.translation_key("bench"));
        match own.filter(|key| tr.has(self.language, key)) {
            Some(key) => tr.text(self.language, &key),
            None => tr.text(self.language, "ui-bench-help"),
        }
    }
}

#[cfg(test)]
#[path = "bench_tests.rs"]
mod tests;
