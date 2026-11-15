//! Furnaces and the stonecutter in the wiki: how an item is cooked or cut, how long it
//! burns, what a furnace cooks and how fast, all read from the packs.

use crate::html::escape;
use crate::recipes::{icon, ingredient};
use crate::site::Ctx;
use strata_core::content::{Block, ItemId};
use strata_core::cooking::{CookingRecipe, DEFAULT_COOK_TIME, FurnaceStats};
use strata_core::id::Identifier;

/// Ticks of a usual cooking.
const USUAL: f64 = DEFAULT_COOK_TIME as f64;

/// The « Cooking », « Stonecutter » and « Fuel » sections of the page of `item`.
pub fn sections(ctx: &Ctx<'_>, from: &str, item: ItemId) -> String {
    let cooking = &ctx.data.content.cooking;
    let cooked: String = cooking
        .recipes()
        .iter()
        .filter(|recipe| recipe.result == item)
        .map(|recipe| cooking_row(ctx, from, recipe))
        .collect();
    let cut: String = cooking
        .cuttings()
        .iter()
        .filter(|recipe| recipe.result == item)
        .map(|recipe| {
            let count = (recipe.count > 1).then(|| recipe.count.to_string());
            row(
                ctx,
                from,
                &recipe.ingredient,
                recipe.result,
                count,
                String::new(),
            )
        })
        .collect();
    let mut html = String::new();
    for (title, rows) in [("wiki-cooking", cooked), ("wiki-cutting", cut)] {
        if !rows.is_empty() {
            let title = escape(&ctx.tr(title));
            html.push_str(&format!(
                "<h2>{title}</h2><div class=\"recipes\">{rows}</div>"
            ));
        }
    }
    if let Some(ticks) = cooking.fuel(item) {
        let args = [
            ("seconds", seconds(ctx, f64::from(ticks))),
            ("cooks", round(ctx, f64::from(ticks) / USUAL)),
        ];
        html.push_str(&format!(
            "<h2>{}</h2><p>{}</p>",
            escape(&ctx.tr("wiki-fuel")),
            escape(&ctx.tr_with("wiki-fuel-burns", &args))
        ));
    }
    html
}

fn cooking_row(ctx: &Ctx<'_>, from: &str, recipe: &CookingRecipe) -> String {
    let stations = stations_cooking(ctx, &recipe.kind).join(", ");
    let args = [
        ("seconds", seconds(ctx, f64::from(recipe.time))),
        ("stations", stations),
    ];
    let note = escape(&ctx.tr_with("wiki-cooking-note", &args));
    let count = (recipe.count > 1).then(|| recipe.count.to_string());
    row(ctx, from, &recipe.ingredient, recipe.result, count, note)
}

fn row(
    ctx: &Ctx<'_>,
    from: &str,
    input: &strata_core::crafting::Ingredient,
    result: ItemId,
    count: Option<String>,
    note: String,
) -> String {
    let name = ctx.data.content.item_name(result, ctx.lang);
    let count = count.map_or_else(String::new, |c| format!("<span class=\"count\">{c}</span>"));
    format!(
        "<figure class=\"recipe\"><span class=\"cell\">{}</span><span class=\"arrow\">→</span>\
         <span class=\"cell\">{}{count}</span><figcaption>{note}</figcaption></figure>",
        ingredient(ctx, from, input),
        icon(ctx, from, result, &name),
    )
}

/// Names of the stations that cook recipes of `kind`: the first block of each furnace
/// family (the one the family is named after).
fn stations_cooking(ctx: &Ctx<'_>, kind: &Identifier) -> Vec<String> {
    let mut names: Vec<String> = ctx
        .data
        .content
        .blocks
        .iter()
        .filter(|(key, _, block)| block.family(*key) == *key)
        .filter_map(|(_, id, block)| Some((id, block.furnace.as_ref()?)))
        .filter(|(id, furnace)| **id == furnace.family)
        .filter(|(_, furnace)| furnace.kinds.is_empty() || furnace.kinds.contains(kind))
        .map(|(id, _)| ctx.tr(&id.translation_key("block")))
        .collect();
    names.sort();
    names
}

/// Infobox rows of a furnace: time of a usual cooking, cookings per coal, upgrade slots.
pub fn furnace_facts(ctx: &Ctx<'_>, block: &Block) -> Vec<(String, String)> {
    let Some(furnace) = &block.furnace else {
        return Vec::new();
    };
    let Some(tier) = ctx.data.content.cooking.tier(&furnace.tier) else {
        return Vec::new();
    };
    let stats = FurnaceStats::new(furnace, tier, &[]);
    let usual = USUAL * stats.time_factor;
    let mut rows = vec![(ctx.tr("wiki-fact-cook-time"), seconds(ctx, usual))];
    if let Some(coal) = ctx.data.content.cooking.reference_fuel() {
        let cooks = f64::from(stats.burn_ticks(coal)) / usual;
        rows.push((ctx.tr("wiki-fact-cooks-per-coal"), round(ctx, cooks)));
    }
    rows.push((
        ctx.tr("wiki-fact-upgrade-slots"),
        tier.upgrade_slots.to_string(),
    ));
    rows
}

fn seconds(ctx: &Ctx<'_>, ticks: f64) -> String {
    round(ctx, ticks / 20.0)
}

fn round(ctx: &Ctx<'_>, value: f64) -> String {
    let rounded = (value * 10.0).round() / 10.0;
    ctx.data
        .content
        .translations
        .short_decimal(ctx.lang, rounded)
}
