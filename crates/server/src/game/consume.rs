//! Eating, drinking and care: an item held down with the use button is consumed once its
//! time has passed (again and again while the button stays down and it still helps);
//! empty bottles and waterskins fill at a water source, salt water by the sea; sneaking
//! with an empty hand drinks at the source; a bag with the feeding upgrade feeds a hungry
//! player from its food.

use super::Game;
use crate::ecs::Using;
use crate::loader::ColumnLoader;
use strata_core::bag::bag_slots;
use strata_core::content::ItemId;
use strata_core::coords::BlockPos;
use strata_core::inventory::{DataValue, HOTBAR, ItemStack, clear_empty};
use strata_core::survival::{Consumable, sips_key, sips_left};

/// Ticks between two looks into the bags for food.
const FEED_PERIOD: u64 = 20;

impl Game {
    /// Starts consuming the item in hotbar slot `slot` (the one held) if it would help.
    pub(super) fn start_use(&mut self, item: ItemId, slot: u8) {
        let slot = usize::from(slot);
        let held = self.ecs.inventories.get(self.player).and_then(|inventory| {
            let stack = inventory.slots.get(slot)?;
            (inventory.selected_index() == slot && stack.item == item).then_some(stack)
        });
        let consumable = self.rules.survival.consumables.get(&item);
        let (Some(_), Some(consumable)) = (held, consumable) else {
            return;
        };
        if !HOTBAR.contains(&slot) || !self.would_help(consumable) {
            return;
        }
        let needed = consumable.ticks;
        let started = self.tick;
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.using = Some(Using {
                item,
                slot,
                started,
                needed,
            });
        }
    }

    pub(super) fn stop_use(&mut self) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.using = None;
        }
    }

    /// Food when not full, water when thirsty; care only when wounded or with an effect it
    /// cures; salt water (`always`, nothing to give) whenever.
    fn would_help(&self, consumable: &Consumable) -> bool {
        let care = !consumable.effects.is_empty() || !consumable.cures.is_empty();
        if consumable.always && care {
            let wounded = (self.ecs.healths.get(self.player)).is_some_and(|h| h.current < h.max);
            let effects = self.ecs.effects.get(self.player);
            let cured = effects.is_some_and(|e| consumable.cures.iter().any(|c| e.has(c)));
            return wounded || cured;
        }
        if consumable.always {
            return true;
        }
        let Some(needs) = self.ecs.needs.get(self.player) else {
            return false;
        };
        let rules = &self.rules.survival.rules;
        let options = self.survival_options;
        let hungry = options.hunger && consumable.food > 0.0 && needs.food < rules.hunger.max;
        let thirsty = options.thirst && consumable.thirst > 0.0 && needs.thirst < rules.thirst.max;
        hungry || thirsty || self.is_creative()
    }

    /// The item held down is consumed once its time is up; it stops if the hand changed.
    pub(super) fn use_tick(&mut self) {
        let Some(using) = self.ecs.controls.get(self.player).and_then(|c| c.using) else {
            return;
        };
        let still_held = self
            .ecs
            .inventories
            .get(self.player)
            .is_some_and(|inventory| {
                inventory.selected_index() == using.slot
                    && inventory.slots.get(using.slot).map(|s| s.item) == Some(using.item)
            });
        if !still_held {
            self.stop_use();
            return;
        }
        if self.tick < using.started + u64::from(using.needed) {
            return;
        }
        let Some(consumable) = self.rules.survival.consumables.get(&using.item).cloned() else {
            self.stop_use();
            return;
        };
        self.consume(&consumable);
        self.use_up_slot(using.slot, &consumable);
        self.stop_use();
        self.start_use(using.item, u8::try_from(using.slot).unwrap_or(u8::MAX));
    }

    /// What eating, drinking or applying it does to the player.
    fn consume(&mut self, consumable: &Consumable) {
        let rules = &self.rules.survival.rules;
        let options = self.survival_options;
        if let Some(needs) = self.ecs.needs.get_mut(self.player) {
            let food = if options.hunger { consumable.food } else { 0.0 };
            let saturation = if options.hunger {
                consumable.saturation
            } else {
                0.0
            };
            let thirst = if options.thirst {
                consumable.thirst
            } else {
                0.0
            };
            needs.nourish(food, saturation, thirst, rules);
        }
        if let Some(effects) = self.ecs.effects.get_mut(self.player) {
            effects.cure(&consumable.cures);
            for grant in &consumable.effects {
                effects.give(grant);
            }
        }
    }

    /// One sip or one item of the slot is gone; what is left in hand (a bowl, a bottle)
    /// takes the slot once it is empty, or goes in the inventory (or to the ground).
    fn use_up_slot(&mut self, slot: usize, consumable: &Consumable) {
        if self.is_creative() {
            return;
        }
        let leaves = consumable.leaves;
        let left = self.with_inventory(|inventory, rules| {
            let place = inventory.slots.slots.get_mut(slot)?;
            let stack = place.as_mut()?;
            let sips = sips_left(stack, consumable);
            if sips > 1 {
                let left = DataValue::Int(i64::from(sips - 1));
                stack.data.set(sips_key(), left);
                return None;
            }
            stack.count -= 1;
            clear_empty(place);
            let leftover = ItemStack::new(leaves?, 1, rules);
            if place.is_none() {
                *place = Some(leftover);
                return None;
            }
            inventory.give(leftover, rules)
        });
        self.throw_all(left.flatten());
    }

    /// Fills the held empty container at the source at `pos`: fresh, or salty in a biome
    /// of salt water.
    pub(super) fn fill_held(&mut self, loader: &ColumnLoader, item: ItemId, pos: BlockPos) -> bool {
        let Some(fill) = self.rules.survival.fills.get(&item).copied() else {
            return false;
        };
        let at_source = loader.block(pos) == Some(fill.liquid);
        if self.held_item() != Some(item) || !at_source || !self.in_reach(pos) {
            return false;
        }
        if !self.take_action() {
            return false;
        }
        let filled = if self.is_salty(loader, pos) {
            fill.salty
        } else {
            fill.fresh
        };
        if self.is_creative() {
            let stack = ItemStack::new(filled, 1, &self.rules);
            let left = self.give(stack);
            self.throw_all(left);
        } else {
            self.exchange_held(filled);
        }
        true
    }

    /// Sneaking with an empty hand at the drinking source: one sip (salty by the sea).
    pub(super) fn drink_at(&mut self, loader: &ColumnLoader, pos: BlockPos) -> bool {
        let survival = &self.rules.survival;
        let sneaking = self
            .ecs
            .controls
            .get(self.player)
            .is_some_and(|c| c.sneaking);
        let source = survival.drink_source.is_some() && loader.block(pos) == survival.drink_source;
        let empty_hand = (self.ecs.inventories.get(self.player))
            .is_some_and(|inventory| inventory.selected_stack().is_none());
        if !source || !sneaking || !empty_hand || !self.in_reach(pos) {
            return false;
        }
        let drinking = &survival.rules.drinking;
        let sip = if self.is_salty(loader, pos) {
            drinking.salty_sip
        } else {
            drinking.sip
        };
        if !self.survival_options.thirst || !self.take_action() {
            return false;
        }
        let rules = &self.rules.survival.rules;
        if let Some(needs) = self.ecs.needs.get_mut(self.player) {
            needs.nourish(0.0, 0.0, sip, rules);
        }
        true
    }

    /// Whether the water at `pos` is salt water (its biome is listed in `survival.ron`).
    fn is_salty(&self, loader: &ColumnLoader, pos: BlockPos) -> bool {
        let Some(biome) = loader.biome(pos) else {
            return false;
        };
        let salty = &self.rules.survival.rules.drinking.salty_biomes;
        self.biomes
            .get(usize::from(biome.0))
            .is_some_and(|id| salty.contains(id))
    }

    /// Names of the biomes by `BiomeId`, to tell salt water from fresh.
    pub fn set_biomes(&mut self, biomes: Vec<strata_core::id::Identifier>) {
        self.biomes = biomes;
    }

    /// A bag with the feeding upgrade gives a food from its slots once it would not be
    /// wasted (the hunger it fills is all missing). Not while a screen is open.
    pub(super) fn feed_from_bags(&mut self) {
        if !self.survival_options.hunger
            || !self.tick.is_multiple_of(FEED_PERIOD)
            || self.ecs.windows.get(self.player).is_some()
        {
            return;
        }
        let Some(needs) = self.ecs.needs.get(self.player) else {
            return;
        };
        let missing = self.rules.survival.rules.hunger.max - needs.food;
        let survival = std::sync::Arc::clone(&self.rules.survival);
        let fits = |stack: &ItemStack| {
            let food = survival.consumables.get(&stack.item);
            food.is_some_and(|f| f.food > 0.0 && f.food <= missing && f.leaves.is_none())
        };
        let found = bag_slots()
            .filter(|&slot| self.bag_at(slot).is_some() && self.carried_upkeep(slot).feed)
            .find_map(|slot| {
                let contents = self.carried_contents(slot)?;
                let index = contents
                    .storage
                    .slots
                    .iter()
                    .position(|s| s.as_ref().is_some_and(fits))?;
                Some((slot, index))
            });
        let Some((slot, index)) = found else {
            return;
        };
        let eaten = self.with_carried(slot, |contents, _| {
            let place = contents.storage.slots.get_mut(index)?;
            let item = place.as_ref()?.item;
            if let Some(stack) = place.as_mut() {
                stack.count -= 1;
            }
            clear_empty(place);
            Some(item)
        });
        if let Some(food) = eaten
            .flatten()
            .and_then(|item| survival.consumables.get(&item))
        {
            self.consume(food);
        }
    }
}
