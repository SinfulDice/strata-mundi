//! Finding the recipe laid in a grid. Recipes are indexed by every item they accept, so
//! only those sharing an item with the grid are tried.

use super::recipe::{Ingredient, Recipe, RecipeShape};
use crate::content::{ItemId, RecipeId, Registry};
use crate::id::Identifier;
use std::collections::HashMap;

/// Every recipe, with its index.
#[derive(Debug, Clone, Default)]
pub struct Recipes {
    registry: Registry<RecipeId, Recipe>,
    by_item: HashMap<ItemId, Vec<RecipeId>>,
}

/// The items of a square grid, row by row.
#[derive(Debug, Clone, Copy)]
pub struct GridItems<'a> {
    pub side: usize,
    pub cells: &'a [Option<ItemId>],
}

impl Recipes {
    pub fn new(list: Vec<(Identifier, Recipe)>) -> Self {
        let mut recipes = Self::default();
        for (id, recipe) in list {
            let mut accepted: Vec<ItemId> = recipe
                .ingredients()
                .into_iter()
                .flat_map(|ingredient| ingredient.items().iter().copied())
                .collect();
            accepted.sort();
            accepted.dedup();
            let Some(key) = recipes.registry.insert(id, recipe) else {
                continue;
            };
            for item in accepted {
                recipes.by_item.entry(item).or_default().push(key);
            }
        }
        recipes
    }

    pub fn get(&self, recipe: RecipeId) -> Option<&Recipe> {
        self.registry.get(recipe)
    }

    pub fn key(&self, id: &Identifier) -> Option<RecipeId> {
        self.registry.key(id)
    }

    pub fn identifier(&self, recipe: RecipeId) -> Option<&Identifier> {
        self.registry.identifier(recipe)
    }

    pub fn iter(&self) -> impl Iterator<Item = (RecipeId, &Identifier, &Recipe)> {
        self.registry.iter()
    }

    pub fn len(&self) -> usize {
        self.registry.len()
    }

    pub fn is_empty(&self) -> bool {
        self.registry.is_empty()
    }

    /// Recipes known from the start of a game.
    pub fn starting(&self) -> impl Iterator<Item = RecipeId> + '_ {
        self.iter()
            .filter(|(_, _, r)| r.starting)
            .map(|(id, _, _)| id)
    }

    /// Recipes making `item`.
    pub fn making(&self, item: ItemId) -> impl Iterator<Item = (RecipeId, &Recipe)> + '_ {
        self.iter()
            .filter(move |(_, _, r)| r.result == item)
            .map(|(id, _, r)| (id, r))
    }

    /// Recipes that may use every item of the grid: the shortest list among them (a
    /// recipe missing from one item's list cannot match).
    fn candidates(&self, grid: GridItems<'_>) -> &[RecipeId] {
        let lists = grid
            .cells
            .iter()
            .flatten()
            .map(|item| self.by_item.get(item).map_or(&[][..], Vec::as_slice));
        lists.min_by_key(|list| list.len()).unwrap_or(&[])
    }

    /// The recipe laid in the grid (the first one when several match).
    pub fn find(&self, grid: GridItems<'_>) -> Option<RecipeId> {
        let trimmed = Trimmed::of(grid)?;
        self.candidates(grid)
            .iter()
            .copied()
            .find(|&id| self.get(id).is_some_and(|recipe| matches(recipe, &trimmed)))
    }

    /// Whether the grid's items are part of a recipe `known` says nothing about yet: all
    /// of them would find a place in it, and some are missing. The interface shows a hint.
    pub fn hint(&self, grid: GridItems<'_>, known: impl Fn(RecipeId) -> bool) -> bool {
        let items: Vec<ItemId> = grid.cells.iter().flatten().copied().collect();
        if items.is_empty() || self.find(grid).is_some() {
            return false;
        }
        self.candidates(grid).iter().any(|&id| {
            let Some(recipe) = self.get(id) else {
                return false;
            };
            let ingredients = recipe.ingredients();
            !known(id)
                && recipe.grid_size() <= grid.side
                && items.len() < ingredients.len()
                && assign(&items, &ingredients, &mut vec![false; ingredients.len()])
        })
    }
}

/// The filled part of a grid: the smallest rectangle holding every item.
struct Trimmed {
    width: usize,
    height: usize,
    cells: Vec<Option<ItemId>>,
}

impl Trimmed {
    fn of(grid: GridItems<'_>) -> Option<Self> {
        let side = grid.side;
        let at = |x: usize, y: usize| grid.cells.get(y * side + x).copied().flatten();
        let columns: Vec<usize> = (0..side)
            .filter(|&x| (0..side).any(|y| at(x, y).is_some()))
            .collect();
        let rows: Vec<usize> = (0..side)
            .filter(|&y| (0..side).any(|x| at(x, y).is_some()))
            .collect();
        let (&left, &right) = (columns.first()?, columns.last()?);
        let (&top, &bottom) = (rows.first()?, rows.last()?);
        let cells = (top..=bottom)
            .flat_map(|y| (left..=right).map(move |x| (x, y)))
            .map(|(x, y)| at(x, y))
            .collect();
        Some(Self {
            width: right - left + 1,
            height: bottom - top + 1,
            cells,
        })
    }
}

fn matches(recipe: &Recipe, grid: &Trimmed) -> bool {
    match &recipe.shape {
        RecipeShape::Shaped {
            width,
            height,
            cells,
        } => {
            (*width, *height) == (grid.width, grid.height)
                && (fits(cells, grid, false) || fits(cells, grid, true))
        }
        RecipeShape::Shapeless(list) => {
            let items: Vec<ItemId> = grid.cells.iter().flatten().copied().collect();
            let ingredients: Vec<&Ingredient> = list.iter().collect();
            items.len() == list.len() && assign(&items, &ingredients, &mut vec![false; list.len()])
        }
    }
}

fn fits(cells: &[Option<Ingredient>], grid: &Trimmed, mirrored: bool) -> bool {
    let width = grid.width;
    grid.cells.iter().enumerate().all(|(index, item)| {
        let (x, y) = (index % width, index / width);
        let x = if mirrored { width - 1 - x } else { x };
        match (&cells[y * width + x], item) {
            (None, None) => true,
            (Some(ingredient), Some(item)) => ingredient.accepts(*item),
            _ => false,
        }
    })
}

/// Gives each item its own ingredient (backtracking: at most nine of each).
fn assign(items: &[ItemId], ingredients: &[&Ingredient], used: &mut [bool]) -> bool {
    let Some((&item, rest)) = items.split_first() else {
        return true;
    };
    for index in 0..ingredients.len() {
        if !used[index] && ingredients[index].accepts(item) {
            used[index] = true;
            if assign(rest, ingredients, used) {
                return true;
            }
            used[index] = false;
        }
    }
    false
}
