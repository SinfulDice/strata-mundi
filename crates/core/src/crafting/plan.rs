//! Which items of an inventory a recipe would use: « can be made now » in the recipe
//! book, and filling the grid from the inventory when a recipe is clicked.

use super::recipe::{Ingredient, Recipe, RecipeShape};
use crate::content::ItemId;
use crate::inventory::{HOTBAR, ItemRules, ItemStack, MAIN, Window};
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
/// for a tag, the item there is most of.
pub fn plan(recipe: &Recipe, available: &HashMap<ItemId, u32>) -> Option<Vec<ItemId>> {
    let ingredients = recipe.ingredients();
    let mut left = available.clone();
    let mut chosen = Vec::with_capacity(ingredients.len());
    choose(&ingredients, &mut left, &mut chosen).then_some(chosen)
}

pub fn can_make(recipe: &Recipe, available: &HashMap<ItemId, u32>) -> bool {
    plan(recipe, available).is_some()
}

fn choose(
    ingredients: &[&Ingredient],
    left: &mut HashMap<ItemId, u32>,
    chosen: &mut Vec<ItemId>,
) -> bool {
    let Some((ingredient, rest)) = ingredients.split_first() else {
        return true;
    };
    let mut options: Vec<(ItemId, u32)> = ingredient
        .items
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

/// The grid cell of each ingredient: shaped recipes from the top left corner.
fn cells(recipe: &Recipe, side: usize) -> Vec<usize> {
    match &recipe.shape {
        RecipeShape::Shaped { width, cells, .. } => cells
            .iter()
            .enumerate()
            .filter(|(_, cell)| cell.is_some())
            .map(|(index, _)| (index / width) * side + index % width)
            .collect(),
        RecipeShape::Shapeless(list) => (0..list.len()).collect(),
    }
}

/// The recipe book: the grid's stacks go back to the inventory, then the recipe is laid
/// with the inventory's items, once or (`all`) as many times as possible. Nothing changes
/// when the grid is too small or the items are missing.
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
    let stacks = HOTBAR.chain(MAIN).filter_map(|i| player.slots.get(i));
    let Some(items) = plan(recipe, &counts(stacks)) else {
        return false;
    };
    let places = cells(recipe, side);
    let rounds = if all { MAX_FILL_ROUNDS } else { 1 };
    for _ in 0..rounds {
        let room = places.iter().zip(&items).all(|(&cell, &item)| {
            let held = grid.slots.get(cell).map_or(0, |stack| stack.count);
            held < rules.max_stack(item)
        });
        if !room || !take_round(&mut player, &mut grid.slots.slots, &places, &items) {
            break;
        }
    }
    *window.player = player;
    if let Some(target) = window.craft.as_deref_mut() {
        *target = grid;
    }
    true
}

/// One item into each place, or nothing when one is missing.
fn take_round(
    player: &mut crate::inventory::PlayerInventory,
    grid: &mut [Option<ItemStack>],
    places: &[usize],
    items: &[ItemId],
) -> bool {
    let mut trial = player.clone();
    let mut taken = Vec::with_capacity(items.len());
    for &item in items {
        match take_one(&mut trial, item) {
            Some(stack) => taken.push(stack),
            None => return false,
        }
    }
    let joins = places.iter().zip(&taken).all(|(&cell, stack)| {
        grid[cell]
            .as_ref()
            .is_none_or(|held| held.stacks_with(stack))
    });
    if !joins {
        return false;
    }
    *player = trial;
    for (&cell, stack) in places.iter().zip(taken) {
        match &mut grid[cell] {
            Some(held) => held.count += 1,
            slot => *slot = Some(stack),
        }
    }
    true
}

/// One item off the main slots first, then the hotbar, from the last slot.
fn take_one(player: &mut crate::inventory::PlayerInventory, item: ItemId) -> Option<ItemStack> {
    let order: Vec<usize> = MAIN.rev().chain(HOTBAR.rev()).collect();
    for index in order {
        let slot = &mut player.slots.slots[index];
        if let Some(stack) = slot.as_mut().filter(|stack| stack.item == item) {
            let one = stack.take(1);
            crate::inventory::clear_empty(slot);
            return one;
        }
    }
    None
}
