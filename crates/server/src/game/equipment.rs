//! What the player wears, each tick: armour slows, and equipment gives its movement
//! abilities (double jump, dash, climbing) unless the world turns them off. Worked out
//! again only when a worn slot changes (`WornCache`).

use super::Game;
use std::sync::Arc;
use strata_core::inventory::worn_slots;
use strata_core::physics::Mobility;
use strata_core::worn::{Equipment, worn_stats};

impl Game {
    /// The armour, accessories and set bonuses worn, and the abilities they give.
    pub fn equipment(&self) -> Arc<Equipment> {
        let Some(inventory) = self.ecs.inventories.get(self.player) else {
            return Arc::default();
        };
        let compute = || {
            let worn = worn_slots().filter_map(|slot| {
                let stack = inventory.slots.get(slot)?;
                Some((stack, self.slot_stats(slot, stack)))
            });
            worn_stats(worn, self.rules.items(), &self.rules.worn)
        };
        self.worn_cache.get(inventory, compute, &self.rules.worn)
    }

    pub(super) fn apply_equipment(&mut self) {
        let equipment = self.equipment();
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            let movement = &mut control.movement;
            movement.speed = f64::from(equipment.stats.burden.speed_factor());
            movement.mobility = if movement.gear_mobility {
                equipment.mobility
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
