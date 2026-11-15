//! Benches, accessories and sets in the wiki: what a bench makes (on its page and on the
//! made item's), what a piece gives when worn, and the sets table of the « Accessories
//! and sets » page, all read from the packs.

use crate::html::escape;
use crate::items::item_link;
use crate::recipes::{icon, ingredient};
use crate::site::Ctx;
use strata_core::content::{Block, Item, ItemId, StationDef};
use strata_core::crafting::BenchRecipe;

/// The « Made at a bench » section of the page of `item`.
pub fn made_at_bench(ctx: &Ctx<'_>, from: &str, item: ItemId) -> String {
    let rows: String = (ctx.data.content.benches.list().iter())
        .filter(|recipe| recipe.result == item)
        .map(|recipe| bench_row(ctx, from, recipe))
        .collect();
    section(ctx, "wiki-bench-made", rows)
}

/// The « What it makes » section of a bench's page.
pub fn bench_makes(ctx: &Ctx<'_>, from: &str, block: &Block) -> String {
    let Some(StationDef::Bench(bench)) = &block.station else {
        return String::new();
    };
    let rows: String = (ctx.data.content.benches.of(bench))
        .map(|(_, recipe)| bench_row(ctx, from, recipe))
        .collect();
    section(ctx, "wiki-bench-makes", rows)
}

fn section(ctx: &Ctx<'_>, title: &str, rows: String) -> String {
    if rows.is_empty() {
        return rows;
    }
    let title = escape(&ctx.tr(title));
    format!("<h2>{title}</h2><div class=\"recipes\">{rows}</div>")
}

fn bench_row(ctx: &Ctx<'_>, from: &str, recipe: &BenchRecipe) -> String {
    let content = &ctx.data.content;
    let cells: String = (recipe.ingredients.iter())
        .map(|(input, count)| {
            let count = if *count > 1 {
                format!("<span class=\"count\">{count}</span>")
            } else {
                String::new()
            };
            format!(
                "<span class=\"cell\">{}{count}</span>",
                ingredient(ctx, from, input)
            )
        })
        .collect();
    let name = content.item_name(recipe.result, ctx.lang);
    let bench = ctx.tr(&recipe.bench.translation_key("block"));
    format!(
        "<figure class=\"recipe\">{cells}<span class=\"arrow\">→</span>\
         <span class=\"cell\">{}</span><figcaption>{}</figcaption></figure>",
        icon(ctx, from, recipe.result, &name),
        escape(&bench)
    )
}

/// Infobox rows of a worn piece: what it gives, the sets it belongs to.
pub fn facts(ctx: &Ctx<'_>, key: ItemId, item: &Item) -> Vec<(String, String)> {
    let content = &ctx.data.content;
    let mut rows = Vec::new();
    if !item.gear.worn.is_empty() {
        let effects: Vec<String> = (item.gear.worn.iter())
            .map(|effect| content.effect_text(effect, ctx.lang))
            .collect();
        rows.push((ctx.tr("wiki-fact-worn"), escape(&effects.join(", "))));
    }
    let sets: Vec<String> = (content.worn.sets_of(key))
        .map(|set| ctx.tr(&set.id.translation_key("set")))
        .collect();
    if !sets.is_empty() {
        rows.push((ctx.tr("wiki-fact-sets"), escape(&sets.join(", "))));
    }
    rows
}

/// Every set: its pieces and each bonus.
pub fn sets_table(ctx: &Ctx<'_>, from: &str) -> String {
    let content = &ctx.data.content;
    let rows: String = (content.worn.sets.iter())
        .map(|set| {
            let pieces: String = (set.pieces.iter())
                .map(|&piece| item_link(ctx, from, piece))
                .collect::<Vec<_>>()
                .join(", ");
            let bonuses: Vec<String> = (set.bonuses.iter())
                .map(|bonus| {
                    let effects: Vec<String> = (bonus.effects.iter())
                        .map(|effect| content.effect_text(effect, ctx.lang))
                        .collect();
                    let args = [
                        ("pieces", bonus.pieces.to_string()),
                        ("effects", effects.join(", ")),
                    ];
                    escape(&ctx.tr_with("set-bonus-line", &args))
                })
                .collect();
            format!(
                "<tr><td>{}</td><td>{pieces}</td><td>{}</td></tr>",
                escape(&ctx.tr(&set.id.translation_key("set"))),
                bonuses.join("<br>")
            )
        })
        .collect();
    let head = ["set", "pieces", "bonuses"]
        .map(|column| {
            format!(
                "<th>{}</th>",
                escape(&ctx.tr(&format!("wiki-sets-{column}")))
            )
        })
        .concat();
    format!(
        "<h2>{}</h2><table class=\"data\"><tr>{head}</tr>{rows}</table>",
        escape(&ctx.tr("wiki-sets-table"))
    )
}

/// The movement abilities and what they are worth (`mobility.ron`).
pub fn mobility_table(ctx: &Ctx<'_>) -> String {
    let rules = &ctx.data.content.worn.mobility;
    let number = |value: f64| ctx.data.content.translations.short_decimal(ctx.lang, value);
    let rows = [
        ("air-jump", number(rules.air_jump_speed)),
        (
            "dash",
            number(rules.dash_speed * f64::from(rules.dash_ticks) / 20.0),
        ),
        (
            "dash-cooldown",
            number(f64::from(rules.dash_cooldown) / 20.0),
        ),
        ("climb", number(rules.climb_speed)),
    ]
    .map(|(key, value)| {
        format!(
            "<tr><td>{}</td><td>{}</td></tr>",
            escape(&ctx.tr(&format!("wiki-mobility-{key}"))),
            escape(&value)
        )
    })
    .concat();
    format!(
        "<h2>{}</h2><table class=\"data\">{rows}</table>",
        escape(&ctx.tr("wiki-mobility-table"))
    )
}
