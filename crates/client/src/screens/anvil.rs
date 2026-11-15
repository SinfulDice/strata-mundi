//! The anvil's side of the inventory screen: the upgrades the item laid can take (shown
//! by their material), the one chosen, and what taking the result costs. The list is
//! worked out here from the same packs as the server, which decides.

use super::slots::SlotsModel;
use crate::ui::SlotPress;
use strata_core::content::ItemId;
use strata_core::inventory::{Click, ItemStack};
use strata_core::upgrade::{ANVIL_ITEM, ANVIL_MATERIAL, Choice, choices};
use strata_protocol::InventoryRequest;

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

    /// Each choice shown by the first item that pays for it.
    pub(super) fn anvil_icons(&self) -> Vec<Option<ItemId>> {
        let choices = self.anvil_choices();
        let icons = choices.iter();
        icons
            .map(|c| c.cost.material.items().first().copied())
            .collect()
    }

    pub(super) fn anvil_selected(&self) -> Option<usize> {
        let chosen = self.crafting?.view.chosen?;
        self.anvil_choices().iter().position(|c| c.index == chosen)
    }

    pub(super) fn anvil_tooltip(&self, index: usize) -> Option<String> {
        let choices = self.anvil_choices();
        let choice = choices.get(index)?;
        Some(self.content.choice_tooltip(choice, self.language))
    }

    /// Choosing the upgrade chosen again goes back to repairing.
    pub(super) fn choose_upgrade(&mut self, index: usize, press: SlotPress) -> bool {
        let Some(choice) = self.anvil_choices().get(index).map(|c| c.index) else {
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
