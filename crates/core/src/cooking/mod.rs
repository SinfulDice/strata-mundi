//! Furnaces and the stonecutter: fuels, cooking recipes by kind, furnace tiers, kits and
//! modules, all from the packs. Pure logic, stepped by the server every tick.

mod cutting;
mod defs;
mod furnace;
mod load;
mod slots;

pub use cutting::{cut_choices, cut_result, take_cut};
pub use defs::{CookingDef, CuttingDef, FuelDef, FurnaceDef, FurnaceTier, KitDef, ModuleDef};
pub use furnace::{FurnaceState, FurnaceStats, Step, step};
pub use slots::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_MODULES, FURNACE_OUTPUT, FurnaceSlots};

pub(crate) use load::load_cooking;

use crate::content::ItemId;
use crate::crafting::Ingredient;
use crate::id::Identifier;
use std::collections::{BTreeMap, HashMap};

/// One item cooked into another.
#[derive(Debug, Clone, PartialEq)]
pub struct CookingRecipe {
    pub id: Identifier,
    pub ingredient: Ingredient,
    pub result: ItemId,
    pub count: u16,
    /// Ticks in a stone furnace.
    pub time: u32,
    pub experience: f32,
    pub kind: Identifier,
}

/// One block cut into a variant at the stonecutter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CuttingRecipe {
    pub id: Identifier,
    pub ingredient: Ingredient,
    pub result: ItemId,
    pub count: u16,
}

/// Everything furnaces and the stonecutter need, with indexes by ingredient.
#[derive(Debug, Clone, Default)]
pub struct Cooking {
    recipes: Vec<CookingRecipe>,
    by_item: HashMap<ItemId, Vec<usize>>,
    cuttings: Vec<CuttingRecipe>,
    cuts_by_item: HashMap<ItemId, Vec<usize>>,
    fuels: HashMap<ItemId, u32>,
    tiers: BTreeMap<Identifier, FurnaceTier>,
}

impl Cooking {
    pub fn new(
        recipes: Vec<CookingRecipe>,
        cuttings: Vec<CuttingRecipe>,
        fuels: HashMap<ItemId, u32>,
        tiers: BTreeMap<Identifier, FurnaceTier>,
    ) -> Self {
        let by_item = index(recipes.iter().map(|r| &r.ingredient));
        let cuts_by_item = index(cuttings.iter().map(|r| &r.ingredient));
        Self {
            recipes,
            by_item,
            cuttings,
            cuts_by_item,
            fuels,
            tiers,
        }
    }

    /// The recipe cooking `item` in a furnace of these `kinds` (empty: every kind).
    pub fn recipe_for(&self, item: ItemId, kinds: &[Identifier]) -> Option<&CookingRecipe> {
        self.by_item
            .get(&item)?
            .iter()
            .filter_map(|&i| self.recipes.get(i))
            .find(|recipe| kinds.is_empty() || kinds.contains(&recipe.kind))
    }

    pub fn recipes(&self) -> &[CookingRecipe] {
        &self.recipes
    }

    /// Ticks `item` burns in a stone furnace, if it is a fuel.
    pub fn fuel(&self, item: ItemId) -> Option<u32> {
        self.fuels.get(&item).copied()
    }

    pub fn fuels(&self) -> impl Iterator<Item = (ItemId, u32)> + '_ {
        self.fuels.iter().map(|(&item, &ticks)| (item, ticks))
    }

    pub fn tier(&self, id: &Identifier) -> Option<&FurnaceTier> {
        self.tiers.get(id)
    }

    pub fn tiers(&self) -> impl Iterator<Item = (&Identifier, &FurnaceTier)> {
        self.tiers.iter()
    }

    /// What the stonecutter makes of `item`, in pack order.
    pub fn cuts(&self, item: ItemId) -> Vec<&CuttingRecipe> {
        self.cuts_by_item
            .get(&item)
            .map(|list| list.iter().filter_map(|&i| self.cuttings.get(i)).collect())
            .unwrap_or_default()
    }

    pub fn cuttings(&self) -> &[CuttingRecipe] {
        &self.cuttings
    }
}

fn index<'a>(ingredients: impl Iterator<Item = &'a Ingredient>) -> HashMap<ItemId, Vec<usize>> {
    let mut by_item: HashMap<ItemId, Vec<usize>> = HashMap::new();
    for (i, ingredient) in ingredients.enumerate() {
        for &item in ingredient.items() {
            by_item.entry(item).or_default().push(i);
        }
    }
    by_item
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
