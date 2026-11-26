//! The « Temperature » page: each biome's temperature by day and by night, and the blocks
//! that warm the air, read from the packs.

use crate::biomes::biome_link;
use crate::html::escape;
use crate::site::Ctx;
use std::collections::BTreeMap;

pub fn tables(ctx: &Ctx<'_>, from: &str) -> String {
    zones(ctx) + &biomes(ctx, from) + &heat(ctx)
}

fn degrees(value: f32) -> String {
    format!("{} °C", (value * 10.0).round() / 10.0)
}

fn zones(ctx: &Ctx<'_>) -> String {
    let rules = &ctx.data.content.survival.temperature;
    let z = &rules.zones;
    let rows = [
        ("freezing", format!("< {}", degrees(z.freezing))),
        (
            "cold",
            format!("{} – {}", degrees(z.freezing), degrees(z.cold)),
        ),
        (
            "comfortable",
            format!("{} – {}", degrees(z.cold), degrees(z.hot)),
        ),
        (
            "hot",
            format!("{} – {}", degrees(z.hot), degrees(z.scorching)),
        ),
        ("scorching", format!("> {}", degrees(z.scorching))),
    ];
    let rows: String = rows
        .iter()
        .map(|(zone, range)| {
            format!(
                "<tr><td>{}</td><td>{}</td></tr>",
                escape(&ctx.tr(&format!("temperature-zone-{zone}"))),
                escape(range)
            )
        })
        .collect();
    table(ctx, "wiki-temperature-zones", &["zone", "range"], &rows)
}

fn biomes(ctx: &Ctx<'_>, from: &str) -> String {
    let rules = &ctx.data.content.survival.temperature;
    let rows: String = (rules.biomes.iter())
        .map(|(id, biome)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                biome_link(ctx, from, id),
                escape(&degrees(biome.day)),
                escape(&degrees(biome.night))
            )
        })
        .collect();
    table(
        ctx,
        "wiki-temperature-biomes",
        &["biome", "day", "night"],
        &rows,
    )
}

fn heat(ctx: &Ctx<'_>) -> String {
    let content = &ctx.data.content;
    let mut sources = BTreeMap::new();
    for (block, _, _) in content.blocks.iter() {
        let (Some(source), Some(name)) =
            (content.survival.heat.get(block), content.block_name(block))
        else {
            continue;
        };
        sources.insert(ctx.tr(&name.translation_key("block")), source);
    }
    let rows: String = (sources.iter())
        .map(|(name, source)| {
            format!(
                "<tr><td>{}</td><td>+{}</td><td>{}</td></tr>",
                escape(name),
                escape(&degrees(source.warmth)),
                (source.range * 10.0).round() / 10.0
            )
        })
        .collect();
    table(
        ctx,
        "wiki-temperature-heat",
        &["block", "warmth", "range"],
        &rows,
    )
}

fn table(ctx: &Ctx<'_>, title: &str, columns: &[&str], rows: &str) -> String {
    let head: String = columns
        .iter()
        .map(|column| {
            let key = format!("{title}-{column}");
            format!("<th>{}</th>", escape(&ctx.tr(&key)))
        })
        .collect();
    format!(
        "<h2>{}</h2><table class=\"data\"><tr>{head}</tr>{rows}</table>",
        escape(&ctx.tr(title))
    )
}
