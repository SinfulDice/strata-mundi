//! What a storage's upgrades make it do on its own: destroy what comes in and the
//! destruction filter selects, press nuggets into ingots and ingots into blocks, sort itself, let in only
//! what the filter allows, draw the items lying around.

use super::Compression;
use super::filter::{ItemFilter, filter_of};
use crate::content::{Item, ItemId, Registry};
use crate::inventory::{Container, ItemRules, ItemStack, clear_empty, sort};
use crate::upgrade::{Effect, Upgrades};

/// The behaviours a storage's upgrades give it, read once from its upgrade slots. A
/// double chest may hold two filters: an item comes in only if every filter lets it in,
/// and goes if any destruction filter selects it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Upkeep {
    pub sort: bool,
    pub compress: bool,
    /// The filters of destruction upgrades (empty ones left out: they destroy nothing).
    pub voids: Vec<ItemFilter>,
    /// The filters of filter upgrades, for shift clicks, quick deposits and magnets.
    pub filters: Vec<ItemFilter>,
    /// Radius in blocks and period in ticks.
    pub magnet: Option<(f32, u16)>,
    /// A bag: takes what the player picks up.
    pub pickup: bool,
    /// A bag: feeds the player (hunger comes with the survival gauges).
    pub feed: bool,
    /// A bag: has a crafting grid.
    pub crafting: bool,
    /// A bag: its tank holds this many buckets.
    pub tank: Option<u16>,
}

impl Upkeep {
    pub fn new(upgrades: &Upgrades, items: &Registry<ItemId, Item>, slots: &[&ItemStack]) -> Self {
        let filters_of = |brick: Effect| -> Vec<ItemFilter> {
            (slots.iter())
                .filter(|stack| {
                    let level = upgrades
                        .of_item(items, stack.item)
                        .and_then(|u| u.levels.first());
                    level.is_some_and(|level| level.effects.iter().any(|e| e.same_kind(&brick)))
                })
                .map(|stack| filter_of(stack, items))
                .collect()
        };
        let mut upkeep = Self {
            voids: filters_of(Effect::Void)
                .into_iter()
                .filter(|f| !f.is_empty())
                .collect(),
            filters: filters_of(Effect::Filter),
            ..Self::default()
        };
        for effect in upgrades.slotted_effects(items, slots.iter().copied()) {
            match effect {
                Effect::Sort => upkeep.sort = true,
                Effect::Compress => upkeep.compress = true,
                Effect::Magnet { radius, period } => upkeep.magnet = Some((radius, period)),
                Effect::Pickup => upkeep.pickup = true,
                Effect::Feed => upkeep.feed = true,
                Effect::Crafting => upkeep.crafting = true,
                Effect::Tank { buckets } => upkeep.tank = Some(buckets),
                _ => {}
            }
        }
        upkeep
    }

    /// Whether anything happens when items come in.
    pub fn acts(&self) -> bool {
        self.sort || self.compress || !self.voids.is_empty()
    }

    /// Whether `item` may come in by shift click, quick deposit or magnet.
    pub fn lets_in(&self, item: ItemId) -> bool {
        self.filters.iter().all(|f| f.lets_in(item))
    }

    /// Whether `item`, coming in, is destroyed.
    pub fn destroys(&self, item: ItemId) -> bool {
        self.voids.iter().any(|f| f.selects(item))
    }
}

/// After items came in (`before` is the storage just before): what came in and a
/// destruction filter selects goes (never what was already there), then compression,
/// then sorting when `sort_now` (not while the screen is open: stacks would jump under
/// the pointer). Returns whether the storage changed.
pub fn after_entry(
    before: &Container,
    storage: &mut Container,
    upkeep: &Upkeep,
    compressions: &[Compression],
    items: &Registry<ItemId, Item>,
    sort_now: bool,
) -> bool {
    let start = storage.clone();
    destroy_entered(before, storage, upkeep);
    if upkeep.compress {
        compress(storage, compressions, items);
    }
    if upkeep.sort && sort_now {
        sort_storage(storage, items);
    }
    *storage != start
}

/// Sorted by category, then pack order.
pub fn sort_storage(storage: &mut Container, items: &Registry<ItemId, Item>) {
    let key = |item: ItemId| {
        let category = items.get(item).and_then(|i| i.category.clone());
        (category, item.0)
    };
    let range = 0..storage.len();
    // Sorting only moves stacks: nothing is left over.
    let _ = sort(storage, range, &[], items, key);
}

/// Destroys, of each item a destruction filter selects, as many as came in since
/// `before` (a stack moved inside the storage did not come in); taken from the slots
/// that grew. Returns how many items went.
pub fn destroy_entered(before: &Container, storage: &mut Container, upkeep: &Upkeep) -> u32 {
    let mut selected: Vec<ItemId> = (storage.slots.iter().flatten())
        .map(|stack| stack.item)
        .filter(|&item| upkeep.destroys(item))
        .collect();
    selected.sort_unstable();
    selected.dedup();
    let mut gone = 0;
    for item in selected {
        let mut left = storage.count(item).saturating_sub(before.count(item));
        for (index, slot) in storage.slots.iter_mut().enumerate() {
            let Some(stack) = slot.as_mut().filter(|s| s.item == item) else {
                continue;
            };
            let had = before
                .get(index)
                .filter(|old| old.stacks_with(stack))
                .map_or(0, |old| u32::from(old.count));
            let grew = u32::from(stack.count).saturating_sub(had).min(left);
            let grew = u16::try_from(grew).unwrap_or(u16::MAX);
            stack.count -= grew;
            left -= u32::from(grew);
            gone += u32::from(grew);
            clear_empty(slot);
        }
    }
    gone
}

/// Presses `count` of an item into one of the next, in recipe order, as long as there
/// is room for the result; only plain stacks (no wear, no data) are pressed.
pub fn compress(
    storage: &mut Container,
    compressions: &[Compression],
    items: &Registry<ItemId, Item>,
) -> bool {
    let mut changed = false;
    // Each pass may feed the next recipe of a chain (nuggets, ingots, blocks).
    for _ in 0..=compressions.len() {
        let mut pressed = false;
        for recipe in compressions {
            pressed |= press(storage, recipe, items);
        }
        if !pressed {
            break;
        }
        changed = true;
    }
    changed
}

fn press(storage: &mut Container, recipe: &Compression, items: &Registry<ItemId, Item>) -> bool {
    let count = u32::from(recipe.count.max(1));
    let plain = |stack: &ItemStack| stack.item == recipe.from && is_plain(stack);
    let total: u32 = storage
        .slots
        .iter()
        .flatten()
        .filter(|s| plain(s))
        .map(|s| u32::from(s.count))
        .sum();
    let mut made = total / count;
    while made > 0 {
        let mut trial = storage.clone();
        take_plain(&mut trial, recipe.from, made * count);
        let result = ItemStack::new(recipe.to, u16::try_from(made).unwrap_or(u16::MAX), items);
        if trial.insert_anywhere(result, items).is_none() {
            *storage = trial;
            return true;
        }
        made /= 2;
    }
    false
}

fn is_plain(stack: &ItemStack) -> bool {
    stack.data.is_empty() && stack.durability.is_none()
}

/// Removes `count` plain items of `item`, from the last slots first.
fn take_plain(storage: &mut Container, item: ItemId, mut count: u32) {
    for slot in storage.slots.iter_mut().rev() {
        if count == 0 {
            break;
        }
        if let Some(stack) = slot.as_mut().filter(|s| s.item == item && is_plain(s)) {
            let taken = u16::try_from(count).unwrap_or(u16::MAX).min(stack.count);
            stack.count -= taken;
            count -= u32::from(taken);
            clear_empty(slot);
        }
    }
}

/// An item drawn by a magnet: refused if the filter does not let it in, destroyed if a
/// destruction filter selects it, put in otherwise. Returns what stayed out.
pub fn take_in(
    storage: &mut Container,
    stack: ItemStack,
    upkeep: &Upkeep,
    rules: &impl ItemRules,
) -> Option<ItemStack> {
    if !upkeep.lets_in(stack.item) {
        return Some(stack);
    }
    if upkeep.destroys(stack.item) {
        return None;
    }
    storage.insert_anywhere(stack, rules)
}
