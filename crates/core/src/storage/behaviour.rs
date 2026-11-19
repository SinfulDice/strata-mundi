//! What a storage's upgrades make it do on its own: destroy what the destruction filter
//! selects, press nuggets into ingots and ingots into blocks, sort itself, let in only
//! what the filter allows, draw the items lying around.

use super::Compression;
use super::filter::{ItemFilter, filter_of};
use crate::content::{Item, ItemId, Registry};
use crate::inventory::{Container, ItemRules, ItemStack, clear_empty, sort};
use crate::upgrade::{Effect, Upgrades};

/// The behaviours a storage's upgrades give it, read once from its upgrade slots.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Upkeep {
    pub sort: bool,
    pub compress: bool,
    /// What the destruction upgrade's filter selects goes.
    pub void: Option<ItemFilter>,
    /// What the filter upgrade lets in by shift click, quick deposit and magnet.
    pub filter: Option<ItemFilter>,
    /// Radius in blocks and period in ticks.
    pub magnet: Option<(f32, u16)>,
}

impl Upkeep {
    pub fn new(upgrades: &Upgrades, items: &Registry<ItemId, Item>, slots: &[&ItemStack]) -> Self {
        let filter_on = |brick: Effect| {
            let index = upgrades.slot_with(items, slots.iter().copied().enumerate(), &brick)?;
            Some(filter_of(slots.get(index)?, items))
        };
        let mut upkeep = Self {
            void: filter_on(Effect::Void).filter(|f| !f.is_empty()),
            filter: filter_on(Effect::Filter),
            ..Self::default()
        };
        for effect in upgrades.slotted_effects(items, slots.iter().copied()) {
            match effect {
                Effect::Sort => upkeep.sort = true,
                Effect::Compress => upkeep.compress = true,
                Effect::Magnet { radius, period } => upkeep.magnet = Some((radius, period)),
                _ => {}
            }
        }
        upkeep
    }

    /// Whether anything happens when items come in.
    pub fn acts(&self) -> bool {
        self.sort || self.compress || self.void.is_some()
    }

    /// Whether `item` may come in by shift click, quick deposit or magnet.
    pub fn lets_in(&self, item: ItemId) -> bool {
        self.filter.as_ref().is_none_or(|f| f.lets_in(item))
    }
}

/// After items came in: destruction, compression, then sorting when `sort_now` (not
/// while the screen is open: stacks would jump under the pointer). Returns whether the
/// storage changed.
pub fn upkeep(
    storage: &mut Container,
    upkeep: &Upkeep,
    compressions: &[Compression],
    items: &Registry<ItemId, Item>,
    sort_now: bool,
) -> bool {
    let before = storage.clone();
    if let Some(void) = &upkeep.void {
        destroy_filtered(storage, void);
    }
    if upkeep.compress {
        compress(storage, compressions, items);
    }
    if upkeep.sort && sort_now {
        let key = |item: ItemId| {
            let category = items.get(item).and_then(|i| i.category.clone());
            (category, item.0)
        };
        let range = 0..storage.len();
        // Sorting only moves stacks: nothing is left over.
        let _ = sort(storage, range, &[], items, key);
    }
    *storage != before
}

/// Destroys the stacks the filter selects; returns how many items went.
pub fn destroy_filtered(storage: &mut Container, filter: &ItemFilter) -> u32 {
    let mut gone = 0;
    for slot in &mut storage.slots {
        if let Some(stack) = slot.take_if(|stack| filter.selects(stack.item)) {
            gone += u32::from(stack.count);
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

/// An item drawn by a magnet or put by a quick deposit: in if the filter allows it;
/// returns what stayed out.
pub fn take_in(
    storage: &mut Container,
    stack: ItemStack,
    upkeep: &Upkeep,
    rules: &impl ItemRules,
) -> Option<ItemStack> {
    if !upkeep.lets_in(stack.item) {
        return Some(stack);
    }
    storage.insert_anywhere(stack, rules)
}
