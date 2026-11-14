//! Which items of an inventory a recipe would use: « can be made now » in the recipe
//! book, and filling the grid from the inventory when a recipe is clicked.

use super::recipe::{Ingredient, Recipe};
use crate::content::ItemId;
use crate::inventory::{HOTBAR, ItemRules, ItemStack, MAIN, PlayerInventory, Window, clear_empty};
use std::collections::HashMap;

/// Stacks put in each cell when the recipe book fills the grid « as many as possible ».
const MAX_FILL_ROUNDS: u16 = 64;

/// How many of each item the stacks hold.
pub fn counts<'a>(stacks: impl Iterator<Item = &'a ItemStack>) -> HashMap<ItemId, u32> {
    let mut counts = HashMap::new();
    for stack in stacks {
        *counts.entry(stack.item).or_insert(0) += u32::from(stack.count);
    }
    counts
}

/// One item for each ingredient (in `Recipe::ingredients` order), taken from `available`;
/// for a tag, the item there is most of. `available` is the same afterwards.
pub fn plan(recipe: &Recipe, available: &mut HashMap<ItemId, u32>) -> Option<Vec<ItemId>> {
    let ingredients = recipe.ingredients();
    let mut chosen = Vec::with_capacity(ingredients.len());
    let found = choose(&ingredients, available, &mut chosen);
    for item in &chosen {
        if let Some(count) = available.get_mut(item) {
            *count += 1;
        }
    }
    found.then_some(chosen)
}

pub fn can_make(recipe: &Recipe, available: &mut HashMap<ItemId, u32>) -> bool {
    plan(recipe, available).is_some()
}

/// Backtracking; on success the counts of `chosen` are left taken.
fn choose(
    ingredients: &[&Ingredient],
    left: &mut HashMap<ItemId, u32>,
    chosen: &mut Vec<ItemId>,
) -> bool {
    let Some((ingredient, rest)) = ingredients.split_first() else {
        return true;
    };
    let mut options: Vec<(ItemId, u32)> = ingredient
        .items()
        .iter()
        .map(|&item| (item, left.get(&item).copied().unwrap_or(0)))
        .filter(|&(_, count)| count > 0)
        .collect();
    options.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    for (item, _) in options {
        if let Some(count) = left.get_mut(&item) {
            *count -= 1;
        }
        chosen.push(item);
        if choose(rest, left, chosen) {
            return true;
        }
        chosen.pop();
        if let Some(count) = left.get_mut(&item) {
            *count += 1;
        }
    }
    false
}

/// The recipe book: the grid's stacks go back to the inventory, then the recipe is laid
/// with the inventory's items, once or (`all`) as many times as possible. Nothing changes
/// when the grid is too small or the items are missing. One copy of the inventory per
/// request, so that a refusal leaves everything as it was.
pub fn fill(window: &mut Window<'_>, recipe: &Recipe, all: bool, rules: &impl ItemRules) -> bool {
    let Some(grid) = window.craft.as_deref() else {
        return false;
    };
    let side = usize::from(grid.side);
    if recipe.grid_size() > side {
        return false;
    }
    let mut player = window.player.clone();
    let mut grid = grid.clone();
    for stack in grid.slots.drain() {
        if player.give(stack, rules).is_some() {
            return false;
        }
    }
    let mut available = counts(HOTBAR.chain(MAIN).filter_map(|i| player.slots.get(i)));
    let Some(items) = plan(recipe, &mut available) else {
        return false;
    };
    let places: Vec<usize> = recipe
        .cells_in(side)
        .iter()
        .map(|(cell, _)| *cell)
        .collect();
    let rounds = if all { MAX_FILL_ROUNDS } else { 1 };
    'rounds: for _ in 0..rounds {
        if !round_fits(&grid.slots.slots, &places, &items, &mut available, rules) {
            break;
        }
        for (&cell, &item) in places.iter().zip(&items) {
            let like = grid.slots.slots[cell].clone();
            let Some(one) = take_one(&mut player, item, like.as_ref()) else {
                break 'rounds;
            };
            match &mut grid.slots.slots[cell] {
                Some(held) => held.count += 1,
                slot => *slot = Some(one),
            }
        }
    }
    *window.player = player;
    if let Some(target) = window.craft.as_deref_mut() {
        *target = grid;
    }
    true
}

/// Whether one more item fits in every place and the inventory still has them all;
/// the counts are then taken from `available`.
fn round_fits(
    grid: &[Option<ItemStack>],
    places: &[usize],
    items: &[ItemId],
    available: &mut HashMap<ItemId, u32>,
    rules: &impl ItemRules,
) -> bool {
    let mut needed: HashMap<ItemId, u32> = HashMap::new();
    for &item in items {
        *needed.entry(item).or_insert(0) += 1;
    }
    let enough = needed
        .iter()
        .all(|(item, need)| available.get(item).is_some_and(|have| have >= need));
    let room = places
        .iter()
        .zip(items)
        .all(|(&cell, &item)| match &grid[cell] {
            None => true,
            Some(held) => held.item == item && held.count < rules.max_stack(item),
        });
    if !(enough && room) {
        return false;
    }
    for (item, need) in needed {
        if let Some(have) = available.get_mut(&item) {
            *have -= need;
        }
    }
    true
}

/// One item off the main slots first, then the hotbar, from the last slot; only from a
/// stack that joins `like` (the cell's stack, same wear and data).
fn take_one(
    player: &mut PlayerInventory,
    item: ItemId,
    like: Option<&ItemStack>,
) -> Option<ItemStack> {
    for index in MAIN.rev().chain(HOTBAR.rev()) {
        let slot = &mut player.slots.slots[index];
        let fits = |stack: &&mut ItemStack| {
            stack.item == item && like.is_none_or(|like| like.stacks_with(stack))
        };
        if let Some(stack) = slot.as_mut().filter(fits) {
            let one = stack.take(1);
            clear_empty(slot);
            return one;
        }
    }
    None
}
