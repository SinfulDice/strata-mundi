//! Work benches with a list of recipes (`bench_recipes/`): the forge, the tannery, the
//! loom and the jeweller. Up to four ingredients, with counts, laid in any of the bench's
//! cells; the recipe is chosen in a list (choosing one lays its ingredients from the
//! inventory), the result taken like a crafting result.

use super::bench_style::BenchStyle;
use super::{into_cursor, into_inventory};
use crate::content::ItemId;
use crate::crafting::Ingredient;
use crate::id::Identifier;
use crate::inventory::{
    Click, CraftGrid, HOTBAR, ItemRules, ItemStack, MAIN, PlayerInventory, Window, clear_empty,
};
use std::collections::BTreeMap;

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

/// The bench recipes of the active packs, in identifier order (a choice's index), and
/// how each bench's screen looks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Benches {
    list: Vec<BenchRecipe>,
    styles: BTreeMap<Identifier, BenchStyle>,
}

impl Benches {
    pub fn new(mut list: Vec<BenchRecipe>) -> Self {
        list.sort_by(|a, b| a.id.cmp(&b.id));
        Self {
            list,
            styles: BTreeMap::new(),
        }
    }

    pub fn with_styles(mut self, styles: BTreeMap<Identifier, BenchStyle>) -> Self {
        self.styles = styles;
        self
    }

    /// How the screen of `bench` looks (plain without a `benches/` file).
    pub fn style(&self, bench: &Identifier) -> &BenchStyle {
        self.styles.get(bench).unwrap_or(&BenchStyle::PLAIN)
    }

    /// Forgets every layout `exists` refuses (the plain one is used instead); returns
    /// each bench and the layout it lost.
    pub fn drop_layouts(&mut self, exists: impl Fn(&str) -> bool) -> Vec<(Identifier, String)> {
        let mut dropped = Vec::new();
        for (bench, style) in &mut self.styles {
            if let Some(layout) = style.layout.take_if(|layout| !exists(layout)) {
                dropped.push((bench.clone(), layout));
            }
        }
        dropped
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
    /// The cell of each ingredient (in the recipe's order), when the cells hold them all,
    /// one cell each. Single items pick their cell before tags, so a tag never takes the
    /// only cell a named item could use.
    pub fn cells(&self, grid: &CraftGrid) -> Option<Vec<usize>> {
        let mut order: Vec<usize> = (0..self.ingredients.len()).collect();
        order.sort_by_key(|&index| self.ingredients[index].0.tag.is_some());
        let mut found = vec![usize::MAX; self.ingredients.len()];
        for index in order {
            let (ingredient, count) = &self.ingredients[index];
            let cell = (0..grid.slots.len()).find(|cell| {
                !found.contains(cell)
                    && grid
                        .slots
                        .get(*cell)
                        .is_some_and(|s| ingredient.accepts(s.item) && s.count >= *count)
            })?;
            found[index] = cell;
        }
        Some(found)
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

/// A click on the result: how many crafts were made, and the remainders that fitted
/// nowhere (to be thrown).
pub fn take_bench(
    window: &mut Window<'_>,
    recipe: &BenchRecipe,
    click: Click,
    rules: &impl ItemRules,
) -> (u32, Vec<ItemStack>) {
    let made = |window: &Window<'_>| bench_result(Some(recipe), window.craft.as_deref()?, rules);
    let mut times = 0;
    let mut thrown = Vec::new();
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
        consume(window, recipe, rules, &mut thrown);
        times += 1;
    }
    (times, thrown)
}

/// The ingredients off their cells; a remainder (a bucket…) takes the emptied cell, or
/// goes to the inventory, like a crafting grid's.
fn consume(
    window: &mut Window<'_>,
    recipe: &BenchRecipe,
    rules: &impl ItemRules,
    thrown: &mut Vec<ItemStack>,
) {
    let Some(grid) = window.craft.as_deref_mut() else {
        return;
    };
    let Some(cells) = recipe.cells(grid) else {
        return;
    };
    let mut leftovers = Vec::new();
    for (cell, (_, count)) in cells.into_iter().zip(&recipe.ingredients) {
        let Some(slot) = grid.slots.slots.get_mut(cell) else {
            continue;
        };
        let Some(stack) = slot.as_mut() else {
            continue;
        };
        let item = stack.item;
        stack.count = stack.count.saturating_sub(*count);
        clear_empty(slot);
        if let Some(left) = rules.remainder(item) {
            let left = ItemStack::new(left, *count, rules);
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

/// Lays the ingredients of `recipe` from the inventory, once, after giving the cells
/// back. Nothing changes when the inventory lacks something.
pub fn fill_bench(window: &mut Window<'_>, recipe: &BenchRecipe, rules: &impl ItemRules) -> bool {
    let Some(grid) = window.craft.as_deref() else {
        return false;
    };
    if recipe.cells(grid).is_some() {
        return true;
    }
    let Some((player, grid)) = laid(window.player, grid, recipe, rules) else {
        return false;
    };
    *window.player = player;
    if let Some(target) = window.craft.as_deref_mut() {
        *target = grid;
    }
    true
}

/// Whether the cells hold the ingredients, or choosing the recipe would lay them: the
/// one check of the server's `fill_bench` and the client's list.
pub fn can_lay(
    player: &PlayerInventory,
    grid: &CraftGrid,
    recipe: &BenchRecipe,
    rules: &impl ItemRules,
) -> bool {
    recipe.cells(grid).is_some() || laid(player, grid, recipe, rules).is_some()
}

/// The inventory and the cells once the ingredients are laid, if they can be.
fn laid(
    player: &PlayerInventory,
    grid: &CraftGrid,
    recipe: &BenchRecipe,
    rules: &impl ItemRules,
) -> Option<(PlayerInventory, CraftGrid)> {
    let mut player = player.clone();
    let mut grid = grid.clone();
    for stack in grid.slots.drain() {
        if player.give(stack, rules).is_some() {
            return None;
        }
    }
    for (cell, (ingredient, count)) in recipe.ingredients.iter().enumerate() {
        let laid = take_from(&mut player.slots.slots, ingredient, *count)?;
        *grid.slots.slots.get_mut(cell)? = Some(laid);
    }
    Some((player, grid))
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
