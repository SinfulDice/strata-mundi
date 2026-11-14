//! Crafting: recipes from the packs, finding the one laid in a grid, crafting it and
//! filling the grid from the recipe book. Pure logic, applied by the server.

mod craft;
mod matching;
mod plan;
mod recipe;

pub use craft::{Crafted, grid_recipe, grid_result, take_result};
pub use matching::{GridItems, Recipes};
pub use plan::{can_make, counts, fill, plan};
pub use recipe::{Ingredient, IngredientDef, MAX_GRID, Recipe, RecipeDef, RecipeShape, ResultDef};
pub(crate) use recipe::{Names, load_recipes};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
