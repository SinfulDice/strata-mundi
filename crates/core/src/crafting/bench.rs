//! Work benches with a list of recipes (`bench_recipes/`): the jeweller now, the forge,
//! the tannery and the loom later. Up to four ingredients, with counts, laid in any of
//! the bench's cells; the recipe is chosen in a list (choosing one lays its ingredients
//! from the inventory), the result taken like a crafting result.

use super::{into_cursor, into_inventory};
use crate::content::ItemId;
use crate::crafting::Ingredient;
use crate::id::Identifier;
use crate::inventory::{Click, CraftGrid, HOTBAR, ItemRules, ItemStack, MAIN, Window, clear_empty};

/// Cells of a bench (a 2×2 grid).
pub const BENCH_SIDE: u8 = 2;
pub const BENCH_CELLS: usize = 4;
/// Crafts in one shift click at most.
const MAX_REPEATS: u32 = 64;

/// A recipe of a bench, its names resolved.
#[derive(Debug, Clone, PartialEq)]
pub struct BenchRecipe {
    pub id: Identifier,
    /// The bench that makes it (`station: Some(Bench("strata:jeweller"))` on a block).
    pub bench: Identifier,
    /// Each ingredient and how many of it one craft takes.
    pub ingredients: Vec<(Ingredient, u16)>,
    pub result: ItemId,
    pub count: u16,
}

/// The bench recipes of the active packs, in identifier order (a choice's index).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Benches {
    list: Vec<BenchRecipe>,
}

impl Benches {
    pub fn new(mut list: Vec<BenchRecipe>) -> Self {
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Self { list }
    }

    pub fn list(&self) -> &[BenchRecipe] {
        &self.list
    }

    pub fn get(&self, index: u16) -> Option<&BenchRecipe> {
        self.list.get(usize::from(index))
    }

    /// The recipes of `bench` with their index.
    pub fn of<'a>(&'a self, bench: &'a Identifier) -> impl Iterator<Item = (u16, &'a BenchRecipe)> {
        self.list
            .iter()
            .enumerate()
            .filter(move |(_, recipe)| recipe.bench == *bench)
            .filter_map(|(index, recipe)| Some((u16::try_from(index).ok()?, recipe)))
    }

    /// Recipe `choice`, if `bench` makes it.
    pub fn chosen(&self, bench: &Identifier, choice: u16) -> Option<&BenchRecipe> {
        self.get(choice).filter(|recipe| recipe.bench == *bench)
    }
}

impl BenchRecipe {
    /// The cell of each ingredient, when the cells hold them all (one cell each).
    pub fn cells(&self, grid: &CraftGrid) -> Option<Vec<usize>> {
        let mut used: Vec<usize> = Vec::new();
        for (ingredient, count) in &self.ingredients {
            let cell = (0..grid.slots.len()).find(|cell| {
                !used.contains(cell)
                    && grid
                        .slots
                        .get(*cell)
                        .is_some_and(|s| ingredient.accepts(s.item) && s.count >= *count)
            })?;
            used.push(cell);
        }
        Some(used)
    }
}

/// What taking the result gives now.
pub fn bench_result(
    recipe: Option<&BenchRecipe>,
    grid: &CraftGrid,
    rules: &impl ItemRules,
) -> Option<ItemStack> {
    let recipe = recipe?;
    recipe.cells(grid)?;
    Some(ItemStack::new(recipe.result, recipe.count, rules))
}

/// A click on the result: returns how many crafts were made.
pub fn take_bench(
    window: &mut Window<'_>,
    recipe: &BenchRecipe,
    click: Click,
    rules: &impl ItemRules,
) -> u32 {
    let made = |window: &Window<'_>| bench_result(Some(recipe), window.craft.as_deref()?, rules);
    let mut times = 0;
    let repeats = match click {
        Click::Primary | Click::Secondary => 1,
        Click::Transfer => MAX_REPEATS,
        _ => 0,
    };
    while times < repeats {
        let Some(result) = made(window) else {
            break;
        };
        let placed = if click == Click::Transfer {
            into_inventory(window, result, rules)
        } else {
            into_cursor(window, result, rules)
        };
        if !placed {
            break;
        }
        consume(window, recipe);
        times += 1;
    }
    times
}

fn consume(window: &mut Window<'_>, recipe: &BenchRecipe) {
    let Some(grid) = window.craft.as_deref_mut() else {
        return;
    };
    let Some(cells) = recipe.cells(grid) else {
        return;
    };
    for (cell, (_, count)) in cells.into_iter().zip(&recipe.ingredients) {
        if let Some(slot) = grid.slots.slots.get_mut(cell) {
            if let Some(stack) = slot.as_mut() {
                stack.count = stack.count.saturating_sub(*count);
            }
            clear_empty(slot);
        }
    }
}

/// Lays the ingredients of `recipe` from the inventory, once, after giving the cells
/// back. Nothing changes when the inventory lacks something.
pub fn fill_bench(window: &mut Window<'_>, recipe: &BenchRecipe, rules: &impl ItemRules) -> bool {
    let Some(grid) = window.craft.as_deref() else {
        return false;
    };
    if recipe.cells(grid).is_some() {
        return true;
    }
    let mut player = window.player.clone();
    let mut grid = grid.clone();
    for stack in grid.slots.drain() {
        if player.give(stack, rules).is_some() {
            return false;
        }
    }
    for (cell, (ingredient, count)) in recipe.ingredients.iter().enumerate() {
        let Some(laid) = take_from(&mut player.slots.slots, ingredient, *count) else {
            return false;
        };
        let Some(slot) = grid.slots.slots.get_mut(cell) else {
            return false;
        };
        *slot = Some(laid);
    }
    *window.player = player;
    if let Some(target) = window.craft.as_deref_mut() {
        *target = grid;
    }
    true
}

/// `count` items of one kind the ingredient accepts, from the hotbar and main slots.
fn take_from(
    slots: &mut [Option<ItemStack>],
    ingredient: &Ingredient,
    count: u16,
) -> Option<ItemStack> {
    let indices = || HOTBAR.chain(MAIN);
    let kind = indices().find_map(|index| {
        let stack = slots.get(index)?.as_ref()?;
        let total: u32 = indices()
            .filter_map(|i| slots.get(i)?.as_ref())
            .filter(|other| other.stacks_with(stack))
            .map(|other| u32::from(other.count))
            .sum();
        (ingredient.accepts(stack.item) && total >= u32::from(count)).then(|| stack.clone())
    })?;
    let mut left = count;
    for index in indices().rev() {
        let Some(slot) = slots.get_mut(index) else {
            continue;
        };
        if let Some(stack) = slot.as_mut().filter(|s| s.stacks_with(&kind)) {
            let taken = stack.count.min(left);
            stack.count -= taken;
            left -= taken;
            clear_empty(slot);
        }
        if left == 0 {
            break;
        }
    }
    Some(ItemStack { count, ..kind })
}
