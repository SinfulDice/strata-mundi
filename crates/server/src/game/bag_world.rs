//! Bags in the world: what the player picks up goes into their pouches and pickup bags
//! first, bag magnets draw the items around the player, and a backpack put down keeps all
//! it holds in its block entity until it is broken (then it is the backpack again, never
//! its contents scattered).

use super::Game;
use super::containers::BlockEntity;
use super::drops::DROP_SIZE;
use crate::ecs::Entity;
use glam::DVec3;
use strata_core::bag::{BagContents, BagWorld, bag_slots, pick_into_bags};
use strata_core::content::BlockId;
use strata_core::coords::BlockPos;
use strata_core::inventory::ItemStack;
use strata_protocol::ServerMessage;

impl Game {
    /// Picks up `stack`: the pouches that take it and the bags with a pickup upgrade
    /// first, then the inventory. Returns what did not fit.
    pub(super) fn pick_up_stack(&mut self, stack: ItemStack) -> Option<ItemStack> {
        let count = stack.count;
        let world = BagWorld {
            bags: &self.rules.bags,
            upgrades: &self.rules.upgrades,
            items: self.rules.items(),
            compressions: &self.rules.storage.compressions,
        };
        let Some(inventory) = self.ecs.inventories.get_mut(self.player) else {
            return Some(stack);
        };
        let left = pick_into_bags(&mut inventory.slots, stack, &world);
        if left.as_ref().is_none_or(|left| left.count < count) {
            self.inventory_changed();
            self.open_bag_follows();
        }
        self.give(left?)
    }

    /// The open bag took something in on its own: its screen follows.
    fn open_bag_follows(&mut self) {
        let Some(window) = self.ecs.windows.get(self.player).cloned() else {
            return;
        };
        if window.carried.is_some() {
            let contents = self.window_contents(&window);
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
    }

    /// A drop within reach goes to the player (what fits).
    pub(super) fn collect_drop(&mut self, entity: Entity) {
        let Some(stack) = self.ecs.drops.get(entity).map(|d| d.stack.clone()) else {
            return;
        };
        match self.pick_up_stack(stack.clone()) {
            None => self.remove_drop(entity),
            Some(left) if left != stack => {
                if let Some(drop) = self.ecs.drops.get_mut(entity) {
                    drop.stack = left;
                }
            }
            Some(_) => {}
        }
    }

    /// The strongest magnet among the player's bags: radius and period.
    fn bag_magnet(&self) -> Option<(f32, u16)> {
        bag_slots()
            .filter(|&slot| self.bag_at(slot).is_some())
            .filter_map(|slot| self.carried_upkeep(slot).magnet)
            .max_by(|a, b| a.0.total_cmp(&b.0))
    }

    /// A bag with a magnet upgrade draws the items around the player at its period
    /// (world option « magnets »), as if the player walked over them.
    pub(super) fn bag_magnets_tick(&mut self) {
        if self.magnets.disabled || self.ecs.drops.len() == 0 {
            return;
        }
        let Some((radius, period)) = self.bag_magnet() else {
            return;
        };
        let Some(body) = self.player_body().copied() else {
            return;
        };
        if !self.tick.is_multiple_of(u64::from(period.max(1))) {
            return;
        }
        let reach = body.aabb();
        let centre = (reach.min + reach.max) / 2.0;
        let near: Vec<Entity> = (self.ecs.drops.iter())
            .filter(|(_, drop)| drop.pickup_delay == 0)
            .filter(|(_, drop)| {
                let middle = drop.body.position + DVec3::Y * (DROP_SIZE / 2.0);
                middle.distance(centre) <= f64::from(radius)
            })
            .map(|(entity, _)| entity)
            .collect();
        for entity in near {
            self.collect_drop(entity);
        }
    }

    /// Whether the held bag becomes `block` when put down.
    pub(super) fn holds_bag_of(&self, block: BlockId) -> bool {
        let held = self.held_item().and_then(|item| self.rules.bags.get(item));
        held.is_some_and(|bag| bag.placed == Some(block))
    }

    /// The held bag was put down at `pos`: it leaves the hand (creative too: it is moved,
    /// not copied) and what it holds goes into the block's entity.
    pub(super) fn put_down_bag(&mut self, pos: BlockPos) {
        if self.pinned() == Some(self.selected()) {
            self.close_container();
        }
        let taken = self.ecs.inventories.get_mut(self.player);
        let Some(stack) = taken.and_then(|inventory| inventory.selected_slot_mut().take()) else {
            return;
        };
        self.inventory_changed();
        let Some(rules) = self.rules.bags.get(stack.item).cloned() else {
            return;
        };
        let (contents, cut) = BagContents::read(&stack.data, &rules.layout);
        let mut data = stack.data.clone();
        BagContents::strip(&mut data);
        BagContents::write_tank(&mut data, contents.tank.as_ref());
        BagContents::set_put_down_as(&mut data, self.rules.items().identifier(stack.item));
        let entity = BlockEntity {
            container: contents.storage,
            upgrades: contents.upgrades,
            data,
            furnace: None,
        };
        self.block_entities.insert(pos, entity);
        self.refresh_magnet(pos);
        let centre = super::containers::block_centre(pos);
        for stack in cut {
            self.spawn_drop(stack, centre, DVec3::ZERO);
        }
    }

    /// A bag put down went away (broken, burnt, washed off): the bag item with all it
    /// held, or `None` if it was no bag (its contents then fall as a chest's).
    pub(super) fn bag_taken_up(&self, entity: &mut BlockEntity) -> Option<ItemStack> {
        let item = self
            .rules
            .items()
            .key(BagContents::put_down_as(&entity.data)?)?;
        let mut data = std::mem::take(&mut entity.data);
        BagContents::set_put_down_as(&mut data, None);
        let contents = BagContents {
            storage: std::mem::take(&mut entity.container),
            upgrades: std::mem::take(&mut entity.upgrades),
            tank: BagContents::read_tank(&data),
        };
        let mut stack = ItemStack::new(item, 1, &self.rules);
        stack.data = data;
        contents.write(&mut stack.data);
        Some(stack)
    }

    /// Whether the block entity at `pos` is a bag put down (it takes no bag).
    pub(super) fn is_put_down_bag(&self, pos: BlockPos) -> bool {
        (self.block_entities.get(&pos)).is_some_and(|e| BagContents::put_down_as(&e.data).is_some())
    }

    /// A kit raised the bag put down at `pos` to `block`: it gives back that tier's bag.
    pub(super) fn follow_bag_tier(&mut self, pos: BlockPos, block: BlockId) {
        let bag = self.rules.bags.of_block(block);
        let id = bag
            .and_then(|item| self.rules.items().identifier(item))
            .cloned();
        let entity = self.block_entities.get_mut(&pos);
        if let (Some(entity), Some(id)) = (entity, id)
            && BagContents::put_down_as(&entity.data).is_some()
        {
            BagContents::set_put_down_as(&mut entity.data, Some(&id));
        }
    }
}
