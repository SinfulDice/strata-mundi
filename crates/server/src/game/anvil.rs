//! The anvil: its two cells belong to the player like a crafting grid's; the server
//! works out what they make (`strata_core::upgrade`) and hands it over on a click.

use super::Game;
use super::crafting::CraftWindow;
use crate::loader::ColumnLoader;
use strata_core::inventory::Click;
use strata_core::upgrade::{
    ANVIL_ITEM, ANVIL_MATERIAL, Outcome, anvil_outcome, choices, take_outcome,
};
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
    /// are used up. The result is worked out from the cells as they are at the click,
    /// once the anvil is known to be there.
    pub(super) fn anvil_click(&mut self, loader: &ColumnLoader, click: Click) {
        self.check_station(loader);
        let window = self.ecs.crafts.get(self.player);
        let Some(choice) = window.filter(|w| w.anvil).map(|w| w.cut) else {
            return;
        };
        let budget = self.xp_budget();
        let paid = self.with_window(loader, |window, rules| {
            let cells = &window.craft.as_deref()?.slots;
            let outcome = anvil_outcome(
                cells.get(ANVIL_ITEM),
                cells.get(ANVIL_MATERIAL),
                choice,
                rules.items(),
                &rules.upgrades,
            )
            .filter(|outcome| outcome.xp <= budget)?;
            let xp = outcome.xp;
            take_outcome(window, outcome, click, rules).then_some(xp)
        });
        if let Some(xp) = paid {
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
    /// paying takes nothing. Milestone 9 gives the player's experience here and takes it
    /// in `pay_xp`.
    fn xp_budget(&self) -> u32 {
        u32::MAX
    }

    fn pay_xp(&mut self, _cost: u32) {}
}
