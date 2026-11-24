//! What the player picks up goes first into the pouches that take it, then into the bags
//! with a pickup upgrade whose filter lets it in; the rest into the inventory.

use super::Bags;
use super::contents::BagContents;
use crate::content::{Item, ItemId, Registry};
use crate::inventory::{BACK, Container, HOTBAR, ItemStack, MAIN, OFFHAND};
use crate::storage::{Compression, Upkeep, after_entry};
use crate::upgrade::Upgrades;

/// Where the player's bags count: the back first, then the hotbar, the main slots and
/// the off hand (never armour or accessory slots).
pub fn bag_slots() -> impl Iterator<Item = usize> + Clone {
    std::iter::once(BACK)
        .chain(HOTBAR)
        .chain(MAIN)
        .chain(std::iter::once(OFFHAND))
}

/// What a bag's upgrades need to work.
pub struct BagWorld<'a> {
    pub bags: &'a Bags,
    pub upgrades: &'a Upgrades,
    pub items: &'a Registry<ItemId, Item>,
    pub compressions: &'a [Compression],
}

/// Offers `stack` to the bags among the player's `slots`; returns what none took. A
/// pouch takes what its tag lists; a bag with a pickup upgrade, what its filter lets in
/// (its destruction and compression upgrades then work on what came in).
pub fn pick_into_bags(
    slots: &mut Container,
    mut stack: ItemStack,
    world: &BagWorld<'_>,
) -> Option<ItemStack> {
    let bags = bag_slots()
        .filter(|&slot| (slots.get(slot)).is_some_and(|bag| world.bags.is_bag(bag.item)));
    let (pouches, others): (Vec<usize>, Vec<usize>) =
        bags.partition(|&slot| is_pouch(slots, slot, world.bags));
    for index in pouches.into_iter().chain(others) {
        let Some(Some(bag)) = slots.slots.get_mut(index) else {
            continue;
        };
        stack = offer(bag, stack, world)?;
    }
    Some(stack)
}

fn is_pouch(slots: &Container, slot: usize, bags: &Bags) -> bool {
    slots
        .get(slot)
        .and_then(|stack| bags.get(stack.item))
        .is_some_and(|rules| rules.accepts.is_some())
}

/// One bag takes what it can of `stack`: only its upgrades are read until it is sure to
/// take it.
fn offer(bag: &mut ItemStack, stack: ItemStack, world: &BagWorld<'_>) -> Option<ItemStack> {
    let Some(rules) = world.bags.get(bag.item) else {
        return Some(stack);
    };
    if !world.bags.admits(rules, &stack) {
        return Some(stack);
    }
    let upgrades = BagContents::read_upgrades(&bag.data, &rules.layout);
    let upgrades: Vec<&ItemStack> = upgrades.slots.iter().flatten().collect();
    let upkeep = Upkeep::new(world.upgrades, world.items, &upgrades);
    let takes = rules.accepts.is_some() || upkeep.pickup;
    if !takes || !upkeep.lets_in(stack.item) {
        return Some(stack);
    }
    let (mut contents, cut) = BagContents::read(&bag.data, &rules.layout);
    if !cut.is_empty() {
        return Some(stack);
    }
    let before = upkeep.acts().then(|| contents.storage.clone());
    let count = stack.count;
    let left = contents.storage.insert_anywhere(stack, world.items);
    if left.as_ref().is_some_and(|left| left.count == count) {
        return left;
    }
    if let Some(before) = before {
        let storage = &mut contents.storage;
        after_entry(
            &before,
            storage,
            &upkeep,
            world.compressions,
            world.items,
            upkeep.sort,
        );
    }
    contents.write(&mut bag.data);
    left
}
