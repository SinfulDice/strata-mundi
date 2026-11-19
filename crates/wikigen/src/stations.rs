//! The « Work stations » page: every block that opens a station or cooks (furnaces),
//! what kind it is and, for a bench, everything it makes, all read from the packs.

use crate::blocks::block_link;
use crate::html::escape;
use crate::recipes::icon;
use crate::site::Ctx;
use strata_core::content::{Block, StationDef};

pub fn table(ctx: &Ctx<'_>, from: &str) -> String {
    let content = &ctx.data.content;
    let rows: String = (content.blocks.iter())
        .filter(|(key, _, block)| block.family(*key) == *key)
        .filter_map(|(key, _, block)| Some((key, block, kind_key(block)?)))
        .map(|(key, block, kind)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                block_link(ctx, from, key),
                escape(&ctx.tr(kind)),
                makes(ctx, from, block)
            )
        })
        .collect();
    let head = ["station", "kind", "makes"]
        .map(|column| {
            format!(
                "<th>{}</th>",
                escape(&ctx.tr(&format!("wiki-stations-{column}")))
            )
        })
        .concat();
    format!(
        "<h2>{}</h2><table class=\"data\"><tr>{head}</tr>{rows}</table>",
        escape(&ctx.tr("wiki-stations-table"))
    )
}

fn kind_key(block: &Block) -> Option<&'static str> {
    if block.furnace.is_some() {
        return Some("wiki-station-furnace");
    }
    Some(match block.station.as_ref()? {
        StationDef::Crafting(_) => "wiki-station-crafting",
        StationDef::Stonecutter => "wiki-station-stonecutter",
        StationDef::Anvil => "wiki-station-anvil",
        StationDef::Bench(_) => "wiki-station-bench",
    })
}

/// The icons of what a bench makes, in its list's order.
fn makes(ctx: &Ctx<'_>, from: &str, block: &Block) -> String {
    let Some(StationDef::Bench(bench)) = &block.station else {
        return String::new();
    };
    let content = &ctx.data.content;
    (content.benches.of(bench))
        .map(|(_, recipe)| {
            let name = content.item_name(recipe.result, ctx.lang);
            icon(ctx, from, recipe.result, &name)
        })
        .collect()
}
