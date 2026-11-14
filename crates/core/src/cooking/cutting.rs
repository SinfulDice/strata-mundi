//! The stonecutter: one item in its single cell, a variant chosen in the list of what it
//! cuts into, then taken like a crafting result (one, or as many as fit).

use super::{Cooking, CuttingRecipe};
use crate::crafting::{into_cursor, into_inventory};
use crate::inventory::{Click, CraftGrid, ItemRules, ItemStack, Window, clear_empty};

/// Cuts in one shift click at most (a full stack).
const MAX_REPEATS: u32 = 64;

/// The choices for the item laid: (index in `Cooking::cuttings`, recipe), in pack order.
pub fn cut_choices<'a>(cooking: &'a Cooking, grid: &CraftGrid) -> Vec<(u16, &'a CuttingRecipe)> {
    let Some(input) = grid.slots.get(0) else {
        return Vec::new();
    };
    cooking
        .cuttings()
        .iter()
        .enumerate()
        .filter(|(_, recipe)| recipe.ingredient.accepts(input.item))
        .filter_map(|(index, recipe)| Some((u16::try_from(index).ok()?, recipe)))
        .collect()
}

/// What taking the result gives now: the chosen recipe, if it cuts the item laid.
pub fn cut_result(
    cooking: &Cooking,
    grid: &CraftGrid,
    choice: Option<u16>,
    rules: &impl ItemRules,
) -> Option<ItemStack> {
    let recipe = cooking.cuttings().get(usize::from(choice?))?;
    let input = grid.slots.get(0)?;
    recipe
        .ingredient
        .accepts(input.item)
        .then(|| ItemStack::new(recipe.result, recipe.count, rules))
}

/// A click on the result: returns how many cuts were made.
pub fn take_cut(
    window: &mut Window<'_>,
    cooking: &Cooking,
    choice: Option<u16>,
    click: Click,
    rules: &impl ItemRules,
) -> u32 {
    let made = |window: &Window<'_>| {
        let grid = window.craft.as_deref()?;
        cut_result(cooking, grid, choice, rules)
    };
    match click {
        Click::Primary | Click::Secondary => {
            let Some(result) = made(window) else {
                return 0;
            };
            if !into_cursor(window, result, rules) {
                return 0;
            }
            consume(window);
            1
        }
        Click::Transfer => {
            let mut times = 0;
            while times < MAX_REPEATS {
                let Some(result) = made(window) else {
                    break;
                };
                if !into_inventory(window, result, rules) {
                    break;
                }
                consume(window);
                times += 1;
            }
            times
        }
        _ => 0,
    }
}

fn consume(window: &mut Window<'_>) {
    let Some(slot) = window
        .craft
        .as_deref_mut()
        .and_then(|grid| grid.slots.slots.get_mut(0))
    else {
        return;
    };
    if let Some(stack) = slot.as_mut() {
        stack.count -= 1;
    }
    clear_empty(slot);
}
