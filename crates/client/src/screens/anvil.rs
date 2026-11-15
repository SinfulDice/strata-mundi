//! The anvil's side of the inventory screen: the upgrades the item laid can take (shown
//! by their material), the one chosen, and what taking the result costs. The list is
//! worked out here from the same packs as the server, which decides.

use super::slots::SlotsModel;
use crate::ui::SlotPress;
use strata_core::content::ItemId;
use strata_core::inventory::{Click, ItemStack};
use strata_core::upgrade::{ANVIL_ITEM, ANVIL_MATERIAL, Choice, choices};
use strata_protocol::InventoryRequest;

/// The list shows this many upgrades a row, and this many rows.
const COLUMNS: usize = 4;
const ROWS: usize = 2;

impl SlotsModel<'_> {
    fn anvil_cell(&self, index: usize) -> Option<&ItemStack> {
        self.crafting?.view.grid.get(index)
    }

    /// The upgrades offered for the item laid, in the packs' order.
    pub(super) fn anvil_choices(&self) -> Vec<Choice<'_>> {
        let Some(stack) = self.anvil_cell(ANVIL_ITEM) else {
            return Vec::new();
        };
        choices(stack, &self.content.items, &self.content.upgrades)
    }

    /// The first choice shown: the list scrolls by rows when it has more than it shows.
    fn first_shown(&self, count: usize) -> usize {
        let rows = count.div_ceil(COLUMNS).saturating_sub(ROWS);
        self.state.upgrade_scroll.min(rows) * COLUMNS
    }

    /// Each choice shown by the first item that pays for it.
    pub(super) fn anvil_icons(&self) -> Vec<Option<ItemId>> {
        let choices = self.anvil_choices();
        let first = self.first_shown(choices.len());
        let icons = choices.iter().skip(first).take(COLUMNS * ROWS);
        icons
            .map(|c| c.cost.material.items().first().copied())
            .collect()
    }

    pub(super) fn anvil_selected(&self) -> Option<usize> {
        let chosen = self.crafting?.view.chosen?;
        let choices = self.anvil_choices();
        let first = self.first_shown(choices.len());
        let at = choices.iter().position(|c| c.index == chosen)?;
        at.checked_sub(first)
            .filter(|&shown| shown < COLUMNS * ROWS)
    }

    pub(super) fn anvil_tooltip(&self, index: usize) -> Option<String> {
        let item = self.anvil_cell(ANVIL_ITEM)?.item;
        let choices = self.anvil_choices();
        let choice = choices.get(self.first_shown(choices.len()) + index)?;
        Some(self.content.choice_tooltip(choice, item, self.language))
    }

    /// The list has more than it shows: its scroll buttons appear.
    pub(super) fn upgrades_scroll(&self) -> bool {
        self.anvil_choices().len() > COLUMNS * ROWS
    }

    pub(super) fn scroll_upgrades(&mut self, rows: i32) {
        let count = self.anvil_choices().len();
        let most = count.div_ceil(COLUMNS).saturating_sub(ROWS);
        let now = self.state.upgrade_scroll.min(most);
        self.state.upgrade_scroll = now.saturating_add_signed(rows as isize).min(most);
    }

    /// What goes in an empty cell of the anvil.
    pub(super) fn anvil_cell_tip(&self, index: usize) -> Option<String> {
        if self.anvil_cell(index).is_some() {
            return None;
        }
        let key = match index {
            ANVIL_ITEM => "ui-anvil-item-tip",
            ANVIL_MATERIAL => "ui-anvil-material-tip",
            _ => return None,
        };
        Some(self.tr_key(key))
    }

    /// Choosing the upgrade chosen again goes back to repairing.
    pub(super) fn choose_upgrade(&mut self, index: usize, press: SlotPress) -> bool {
        let choices = self.anvil_choices();
        let shown = choices.get(self.first_shown(choices.len()) + index);
        let Some(choice) = shown.map(|c| c.index) else {
            return false;
        };
        if matches!(press, SlotPress::Click(Click::Primary | Click::Transfer)) {
            let chosen = self.crafting.and_then(|c| c.view.chosen);
            let next = (chosen != Some(choice)).then_some(choice);
            self.inventory
                .request(InventoryRequest::ChooseUpgrade(next));
        }
        true
    }

    pub(super) fn choose_repair(&mut self) {
        self.inventory
            .request(InventoryRequest::ChooseUpgrade(None));
    }

    /// What the result is (an upgrade or a repair) and its price, or how to use the anvil.
    pub(super) fn anvil_note(&self) -> String {
        let Some(view) = self.crafting.map(|c| &c.view) else {
            return String::new();
        };
        let tr = &self.content.translations;
        let xp = view.cost.unwrap_or(0).to_string();
        match (view.result.is_some(), view.chosen) {
            (true, Some(_)) => tr.text_with(self.language, "ui-anvil-upgrade", &[("xp", xp)]),
            (true, None) => tr.text_with(self.language, "ui-anvil-repair", &[("xp", xp)]),
            (false, Some(_)) => self.tr_key("ui-anvil-missing"),
            (false, None)
                if self.anvil_cell(ANVIL_ITEM).is_some()
                    && self.anvil_cell(ANVIL_MATERIAL).is_some() =>
            {
                self.tr_key("ui-anvil-no-repair")
            }
            (false, None) => self.tr_key("ui-anvil-help"),
        }
    }

    fn tr_key(&self, key: &str) -> String {
        self.content.translations.text(self.language, key)
    }
}
