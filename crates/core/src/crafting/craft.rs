//! Taking what the grid makes: one craft into the carried stack, or as many as fit into
//! the inventory (shift click). Each craft uses one item of every filled cell.

use super::matching::{GridItems, Recipes};
use crate::content::RecipeId;
use crate::inventory::{Click, CraftGrid, ItemRules, ItemStack, Window, clear_empty};

/// Crafts in one shift click at most (a full grid of full stacks).
const MAX_REPEATS: u32 = 64 * 9;

/// What a click on the result did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Crafted {
    pub recipe: RecipeId,
    pub times: u32,
    /// Remainders that fitted nowhere, to be thrown.
    pub thrown: Vec<ItemStack>,
}

/// The recipe laid in `grid`, if any.
pub fn grid_recipe(recipes: &Recipes, grid: &CraftGrid) -> Option<RecipeId> {
    let items = grid.items();
    recipes.find(GridItems {
        side: usize::from(grid.side),
        cells: &items,
    })
}

/// What the grid would make now.
pub fn grid_result(
    recipes: &Recipes,
    grid: &CraftGrid,
    rules: &impl ItemRules,
) -> Option<ItemStack> {
    let recipe = recipes.get(grid_recipe(recipes, grid)?)?;
    Some(ItemStack::new(recipe.result, recipe.count, rules))
}

/// A click on the result slot. `None` when nothing was crafted.
pub fn take_result(
    window: &mut Window<'_>,
    recipes: &Recipes,
    click: Click,
    rules: &impl ItemRules,
) -> Option<Crafted> {
    let grid = window.craft.as_deref()?;
    let recipe = grid_recipe(recipes, grid)?;
    let made = grid_result(recipes, grid, rules)?;
    let mut thrown = Vec::new();
    let times = match click {
        Click::Primary | Click::Secondary => {
            if !into_cursor(window, made, rules) {
                return None;
            }
            consume(window, rules, &mut thrown);
            1
        }
        Click::Transfer => {
            let mut times = 0;
            while times < MAX_REPEATS
                && window
                    .craft
                    .as_deref()
                    .and_then(|g| grid_recipe(recipes, g))
                    == Some(recipe)
                && into_inventory(window, made.clone(), rules)
            {
                consume(window, rules, &mut thrown);
                times += 1;
            }
            times
        }
        _ => 0,
    };
    (times > 0).then_some(Crafted {
        recipe,
        times,
        thrown,
    })
}

fn into_cursor(window: &mut Window<'_>, made: ItemStack, rules: &impl ItemRules) -> bool {
    match &mut window.player.cursor {
        None => {
            window.player.cursor = Some(made);
            true
        }
        Some(carried)
            if carried.stacks_with(&made)
                && carried.count + made.count <= rules.max_stack(made.item) =>
        {
            carried.count += made.count;
            true
        }
        Some(_) => false,
    }
}

/// Puts the whole result in the inventory, or nothing at all.
fn into_inventory(window: &mut Window<'_>, made: ItemStack, rules: &impl ItemRules) -> bool {
    let mut trial = window.player.clone();
    if trial.give(made, rules).is_some() {
        return false;
    }
    *window.player = trial;
    true
}

/// One item off every filled cell; a remainder takes the emptied cell, or goes to the
/// inventory when the cell still holds something.
fn consume(window: &mut Window<'_>, rules: &impl ItemRules, thrown: &mut Vec<ItemStack>) {
    let Some(grid) = window.craft.as_deref_mut() else {
        return;
    };
    let mut leftovers = Vec::new();
    for slot in &mut grid.slots.slots {
        let Some(stack) = slot.as_mut() else {
            continue;
        };
        let item = stack.item;
        stack.count -= 1;
        clear_empty(slot);
        if let Some(left) = rules.remainder(item) {
            let left = ItemStack::new(left, 1, rules);
            match slot {
                None => *slot = Some(left),
                Some(_) => leftovers.push(left),
            }
        }
    }
    for stack in leftovers {
        thrown.extend(window.player.give(stack, rules));
    }
}
