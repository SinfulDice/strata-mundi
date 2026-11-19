//! Bench recipes as written in `bench_recipes/<name>.ron`, and the benches' looks.

use super::bench::{BENCH_CELLS, BenchRecipe, Benches};
use super::bench_style::load_styles;
use super::{IngredientDef, Names, ResultDef, ingredient};
use crate::id::Identifier;
use crate::inventory::ItemRules;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "BenchRecipe", deny_unknown_fields)]
pub struct BenchRecipeDef {
    pub bench: Identifier,
    /// `(Item("strata:gold_ingot"), 2)`: an ingredient and how many.
    pub ingredients: Vec<(IngredientDef, u16)>,
    pub result: ResultDef,
}

/// `stations`: the benches blocks open (`station: Some(Bench(..))`).
pub(crate) fn load_benches(
    packs: &PackSet,
    names: &Names<'_>,
    stations: &BTreeSet<Identifier>,
    errors: &mut Vec<PackError>,
) -> Benches {
    let list = packs
        .layered::<BenchRecipeDef>("bench_recipes", errors)
        .into_iter()
        .filter_map(|(id, Sourced { value, source })| recipe(id, value, names, &source, errors))
        .collect();
    Benches::new(list).with_styles(load_styles(packs, stations, errors))
}

fn recipe(
    id: Identifier,
    def: BenchRecipeDef,
    names: &Names<'_>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<BenchRecipe> {
    if def.ingredients.is_empty() || def.ingredients.len() > BENCH_CELLS {
        let message = format!("a bench recipe needs 1 to {BENCH_CELLS} ingredients");
        errors.push(PackError::invalid(source, message));
        return None;
    }
    let ingredients = def
        .ingredients
        .into_iter()
        .map(|(def, count)| Some((ingredient(def, names, source, errors)?, count.max(1))))
        .collect::<Option<Vec<_>>>()?;
    let Some(result) = names.items.key(&def.result.item) else {
        let message = format!("unknown item `{}`", def.result.item);
        errors.push(PackError::invalid(source, message));
        return None;
    };
    let max = |item| names.items.max_stack(item);
    let too_many = ingredients
        .iter()
        .any(|(ingredient, count)| ingredient.items().iter().any(|&i| *count > max(i)));
    if too_many || def.result.count > max(result) {
        let message = "a count is more than a stack of its item holds";
        errors.push(PackError::invalid(source, message));
        return None;
    }
    Some(BenchRecipe {
        id,
        bench: def.bench,
        ingredients,
        result,
        count: def.result.count.max(1),
    })
}
