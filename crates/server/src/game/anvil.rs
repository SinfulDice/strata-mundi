//! The anvil: its two cells belong to the player like a crafting grid's; the server
//! works out what they make (`strata_core::upgrade`) and hands it over on a click.

use super::Game;
use super::crafting::CraftWindow;
use crate::loader::ColumnLoader;
use strata_core::inventory::Click;
use strata_core::upgrade::{ANVIL_ITEM, ANVIL_MATERIAL, Outcome, anvil_outcome, choices};
use strata_protocol::CraftingView;

impl Game {
    /// What the anvil's cells make with the upgrade chosen (or a repair).
    fn anvil_outcome(&self, window: &CraftWindow) -> Option<Outcome> {
        let cells = &window.grid.slots;
        anvil_outcome(
            cells.get(ANVIL_ITEM),
            cells.get(ANVIL_MATERIAL),
            window.cut,
            self.rules.items(),
            &self.rules.upgrades,
        )
    }

    /// A click on the anvil's result: the experience is paid, the item and the material
    /// are used up.
    pub(super) fn anvil_click(&mut self, loader: &ColumnLoader, click: Click) {
        let outcome = self
            .ecs
            .crafts
            .get(self.player)
            .and_then(|w| self.anvil_outcome(w));
        let Some(outcome) = outcome.filter(|o| self.can_pay_xp(o.xp)) else {
            return;
        };
        let xp = outcome.xp;
        let taken = self.with_window(loader, |window, rules| {
            strata_core::upgrade::take_outcome(window, outcome, click, rules)
        });
        if taken {
            self.pay_xp(xp);
        }
    }

    /// The upgrade to put next, if the item laid can take it; none to repair.
    pub(super) fn choose_upgrade(&mut self, choice: Option<u16>) {
        let Some(window) = self.ecs.crafts.get_mut(self.player) else {
            return;
        };
        if !window.anvil {
            return;
        }
        let offered = |index: u16| {
            window.grid.slots.get(ANVIL_ITEM).is_some_and(|stack| {
                choices(stack, self.rules.items(), &self.rules.upgrades)
                    .iter()
                    .any(|c| c.index == index)
            })
        };
        if choice.is_none_or(offered) {
            window.cut = choice;
            self.craft_changed();
        }
    }

    pub(super) fn anvil_view(&self, window: &CraftWindow) -> CraftingView {
        let outcome = self.anvil_outcome(window);
        CraftingView {
            side: window.grid.side,
            grid: window.grid.slots.clone(),
            result: outcome.as_ref().map(|o| o.result.clone()),
            hint: false,
            station: window.station,
            cuts: Vec::new(),
            chosen: window.cut,
            cost: outcome.map(|o| o.xp),
        }
    }

    /// Experience does not exist before milestone 9: every cost can be paid, and
    /// paying takes nothing. Milestone 9 checks and takes the player's levels here.
    fn can_pay_xp(&self, _cost: u32) -> bool {
        true
    }

    fn pay_xp(&mut self, _cost: u32) {}
}
