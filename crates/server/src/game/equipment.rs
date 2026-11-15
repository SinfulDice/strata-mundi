//! What the player wears, each tick: armour slows, and equipment gives its movement
//! abilities (double jump, dash, climbing) unless the world turns them off.

use super::Game;
use strata_core::inventory::worn_slots;
use strata_core::physics::Mobility;
use strata_core::worn::{WornStats, worn_stats};

impl Game {
    /// The armour, accessories and set bonuses worn, their stats cached until the
    /// pieces change.
    pub fn worn_stats(&self) -> WornStats {
        let Some(inventory) = self.ecs.inventories.get(self.player) else {
            return WornStats::default();
        };
        let worn = worn_slots().filter_map(|slot| {
            let stack = inventory.slots.get(slot)?;
            Some((stack, self.slot_stats(slot, stack)))
        });
        worn_stats(worn, self.rules.items(), &self.rules.worn)
    }

    pub(super) fn apply_equipment(&mut self) {
        let worn = self.worn_stats();
        let mobility = self.rules.worn.mobility.mobility(&worn);
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            let movement = &mut control.movement;
            movement.speed = f64::from(worn.burden.speed_factor());
            movement.mobility = if movement.gear_mobility {
                mobility
            } else {
                Mobility::default()
            };
        }
    }

    /// World option: equipment may give movement abilities.
    pub fn set_gear_mobility(&mut self, allowed: bool) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.movement.gear_mobility = allowed;
        }
    }
}
