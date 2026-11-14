//! Crafting recipes as written in `recipes/<name>.ron`, resolved against the items and
//! tags, with every unknown name reported with its file.

use crate::content::{Item, ItemId, ItemTags, Registry};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Largest grid (the crafting table): recipes must fit in it.
pub const MAX_GRID: usize = 3;
const MAX_RESULT: u16 = 64;

#[derive(Debug, Clone, Deserialize)]
pub enum RecipeDef {
    /// Rows of the pattern, one character per cell (a space is an empty cell); found
    /// anywhere in the grid, and also mirrored left to right.
    Shaped {
        pattern: Vec<String>,
        key: BTreeMap<char, IngredientDef>,
        result: ResultDef,
        #[serde(default)]
        starting: bool,
    },
    /// The ingredients in any cells.
    Shapeless {
        ingredients: Vec<IngredientDef>,
        result: ResultDef,
        #[serde(default)]
        starting: bool,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub enum IngredientDef {
    Item(Identifier),
    /// Any item of the tag.
    Tag(Identifier),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Result", deny_unknown_fields)]
pub struct ResultDef {
    pub item: Identifier,
    #[serde(default = "one")]
    pub count: u16,
}

fn one() -> u16 {
    1
}

/// What one cell of a recipe accepts: one item, or any item of a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Ingredient {
    /// Sorted, for `accepts`.
    items: Vec<ItemId>,
    /// The tag it came from, for the wiki and the journal.
    pub tag: Option<Identifier>,
}

impl Ingredient {
    pub fn item(item: ItemId) -> Self {
        Self {
            items: vec![item],
            tag: None,
        }
    }

    /// Any of `items` (those of `tag`, if it comes from one).
    pub fn new(mut items: Vec<ItemId>, tag: Option<Identifier>) -> Self {
        items.sort();
        items.dedup();
        Self { items, tag }
    }

    /// The items accepted, sorted.
    pub fn items(&self) -> &[ItemId] {
        &self.items
    }

    pub fn accepts(&self, item: ItemId) -> bool {
        self.items.binary_search(&item).is_ok()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecipeShape {
    /// `cells` row by row, `width` per row.
    Shaped {
        width: usize,
        height: usize,
        cells: Vec<Option<Ingredient>>,
    },
    Shapeless(Vec<Ingredient>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recipe {
    pub shape: RecipeShape,
    pub result: ItemId,
    pub count: u16,
    /// Known from the start of a game, before any discovery.
    pub starting: bool,
}

impl Recipe {
    /// Every ingredient, one per filled cell.
    pub fn ingredients(&self) -> Vec<&Ingredient> {
        match &self.shape {
            RecipeShape::Shaped { cells, .. } => cells.iter().flatten().collect(),
            RecipeShape::Shapeless(list) => list.iter().collect(),
        }
    }

    /// Each ingredient with its cell in a grid of `side` (row by row): a shaped recipe
    /// from the top left corner, a shapeless one in the first cells.
    pub fn cells_in(&self, side: usize) -> Vec<(usize, &Ingredient)> {
        match &self.shape {
            RecipeShape::Shaped { width, cells, .. } => cells
                .iter()
                .enumerate()
                .filter_map(|(index, cell)| {
                    Some(((index / width) * side + index % width, cell.as_ref()?))
                })
                .collect(),
            RecipeShape::Shapeless(list) => list.iter().enumerate().collect(),
        }
    }

    /// Smallest square grid it fits in.
    pub fn grid_size(&self) -> usize {
        match &self.shape {
            RecipeShape::Shaped { width, height, .. } => (*width).max(*height),
            RecipeShape::Shapeless(list) => (1..=MAX_GRID)
                .find(|side| side * side >= list.len())
                .unwrap_or(MAX_GRID),
        }
    }
}

/// What a definition may refer to.
pub(crate) struct Names<'a> {
    pub items: &'a Registry<ItemId, Item>,
    pub tags: &'a ItemTags,
}

pub(crate) fn load_recipes(
    packs: &PackSet,
    names: &Names<'_>,
    errors: &mut Vec<PackError>,
) -> Vec<(Identifier, Recipe)> {
    packs
        .layered::<RecipeDef>("recipes", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| {
            let recipe = resolve(value, names, &source, errors)?;
            Some((id, recipe))
        })
        .collect()
}

fn resolve(
    def: RecipeDef,
    names: &Names<'_>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Recipe> {
    let (shape, result, starting) = match def {
        RecipeDef::Shaped {
            pattern,
            key,
            result,
            starting,
        } => (
            shaped(&pattern, &key, names, source, errors)?,
            result,
            starting,
        ),
        RecipeDef::Shapeless {
            ingredients,
            result,
            starting,
        } => {
            if ingredients.is_empty() || ingredients.len() > MAX_GRID * MAX_GRID {
                errors.push(PackError::invalid(source, "1 to 9 ingredients"));
                return None;
            }
            let list = ingredients
                .into_iter()
                .map(|def| ingredient(def, names, source, errors))
                .collect::<Option<Vec<_>>>()?;
            (RecipeShape::Shapeless(list), result, starting)
        }
    };
    let Some(item) = names.items.key(&result.item) else {
        let message = format!("unknown result item `{}`", result.item);
        errors.push(PackError::invalid(source, message));
        return None;
    };
    let max = names
        .items
        .get(item)
        .map_or(MAX_RESULT, |entry| entry.max_stack.clamp(1, MAX_RESULT));
    if !(1..=max).contains(&result.count) {
        let message = format!(
            "result count: 1 to {max} (a full stack of `{}`)",
            result.item
        );
        errors.push(PackError::invalid(source, message));
    }
    Some(Recipe {
        shape,
        result: item,
        count: result.count.clamp(1, max),
        starting,
    })
}

fn shaped(
    pattern: &[String],
    key: &BTreeMap<char, IngredientDef>,
    names: &Names<'_>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<RecipeShape> {
    let height = pattern.len();
    let width = pattern
        .iter()
        .map(|row| row.chars().count())
        .max()
        .unwrap_or(0);
    if !(1..=MAX_GRID).contains(&height) || !(1..=MAX_GRID).contains(&width) {
        errors.push(PackError::invalid(
            source,
            "pattern: 1 to 3 rows of 1 to 3 cells",
        ));
        return None;
    }
    let mut cells = Vec::with_capacity(width * height);
    for row in pattern {
        let mut chars: Vec<char> = row.chars().collect();
        chars.resize(width, ' ');
        for symbol in chars {
            if symbol == ' ' {
                cells.push(None);
                continue;
            }
            let Some(def) = key.get(&symbol) else {
                let message = format!("pattern: `{symbol}` is not in the key");
                errors.push(PackError::invalid(source, message));
                return None;
            };
            cells.push(Some(ingredient(def.clone(), names, source, errors)?));
        }
    }
    if cells.iter().all(Option::is_none) {
        errors.push(PackError::invalid(source, "pattern: no ingredient"));
        return None;
    }
    Some(trimmed(width, height, cells))
}

/// Empty outer rows and columns are left out: a pattern is found wherever it lies.
fn trimmed(width: usize, height: usize, cells: Vec<Option<Ingredient>>) -> RecipeShape {
    let filled = |x: usize, y: usize| cells[y * width + x].is_some();
    let columns: Vec<usize> = (0..width)
        .filter(|&x| (0..height).any(|y| filled(x, y)))
        .collect();
    let rows: Vec<usize> = (0..height)
        .filter(|&y| (0..width).any(|x| filled(x, y)))
        .collect();
    let (left, right) = (columns[0], columns[columns.len() - 1]);
    let (top, bottom) = (rows[0], rows[rows.len() - 1]);
    let mut kept = Vec::new();
    for y in top..=bottom {
        for x in left..=right {
            kept.push(cells[y * width + x].clone());
        }
    }
    RecipeShape::Shaped {
        width: right - left + 1,
        height: bottom - top + 1,
        cells: kept,
    }
}

pub(crate) fn ingredient(
    def: IngredientDef,
    names: &Names<'_>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<Ingredient> {
    match def {
        IngredientDef::Item(id) => match names.items.key(&id) {
            Some(item) => Some(Ingredient::item(item)),
            None => {
                errors.push(PackError::invalid(source, format!("unknown item `{id}`")));
                None
            }
        },
        IngredientDef::Tag(id) => match names.tags.get(&id) {
            Some(items) if !items.is_empty() => Some(Ingredient::new(items.to_vec(), Some(id))),
            Some(_) => {
                errors.push(PackError::invalid(source, format!("tag `{id}` is empty")));
                None
            }
            None => {
                errors.push(PackError::invalid(source, format!("unknown tag `{id}`")));
                None
            }
        },
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "recipe_tests.rs"]
mod tests;
