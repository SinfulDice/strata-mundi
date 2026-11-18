//! Crafting: recipes from the packs, finding the one laid in a grid, crafting it and
//! filling the grid from the recipe book; the benches and their lists of recipes. Pure logic, applied by the server.

mod bench;
mod bench_load;
mod bench_style;
mod craft;
mod matching;
mod plan;
mod recipe;

pub use bench::{
    BENCH_CELLS, BENCH_SIDE, BenchRecipe, Benches, bench_result, can_lay, fill_bench, take_bench,
};
pub use bench_load::BenchRecipeDef;
pub(crate) use bench_load::load_benches;
pub use bench_style::{BenchLayout, BenchStyle};
pub use craft::{Crafted, grid_recipe, grid_result, take_result};
pub(crate) use craft::{into_cursor, into_inventory};
pub use matching::{GridItems, Recipes};
pub use plan::{can_make, counts, fill, plan};
pub use recipe::{Ingredient, IngredientDef, MAX_GRID, Recipe, RecipeDef, RecipeShape, ResultDef};
pub(crate) use recipe::{Names, ingredient, load_recipes};

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod bench_tests;
