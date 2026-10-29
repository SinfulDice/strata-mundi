//! Biome pages: a view rendered by the game, the blocks and features from the pack, the
//! written text and the history; and the list of biomes.

use crate::blocks::block_link;
use crate::html::{escape, image, infobox, link};
use crate::pages::history_section;
use crate::site::{Ctx, Page, id_path};
use strata_core::content::BlockId;
use strata_core::id::Identifier;
use strata_worldgen::{Biome, Feature};

pub fn pages(ctx: &Ctx<'_>) -> Vec<Page> {
    let biomes = ctx.data.worldgen.biomes();
    let mut pages: Vec<Page> = biomes.iter().map(|biome| page(ctx, biome)).collect();
    pages.push(index(ctx, biomes));
    pages
}

pub fn page_path(id: &Identifier) -> String {
    format!("biomes/{}.html", id_path(id))
}

pub fn name(ctx: &Ctx<'_>, id: &Identifier) -> String {
    ctx.tr(&id.translation_key("biome"))
}

pub fn biome_link(ctx: &Ctx<'_>, from: &str, id: &Identifier) -> String {
    format!(
        "<a href=\"{}\">{}</a>",
        link(from, &page_path(id)),
        escape(&name(ctx, id))
    )
}

fn page(ctx: &Ctx<'_>, biome: &Biome) -> Page {
    let path = page_path(&biome.id);
    let key = format!("biomes/{}", id_path(&biome.id));
    let title = name(ctx, &biome.id);
    let picture = image(ctx, &path, "biomes", &id_path(&biome.id), &title, "");
    let box_html = infobox(&title, &picture, &facts(ctx, &path, biome, &key));
    let body = format!(
        "{box_html}{}{}{}",
        ctx.text(&key),
        climate(ctx, biome),
        history_section(ctx, &key)
    );
    Page { path, title, body }
}

fn facts(ctx: &Ctx<'_>, from: &str, biome: &Biome, key: &str) -> Vec<(String, String)> {
    let yes_no = |value: bool| escape(&ctx.tr(if value { "wiki-yes" } else { "wiki-no" }));
    let blocks = |ids: &[BlockId]| -> String {
        let mut links: Vec<String> = ids.iter().map(|&b| block_link(ctx, from, b)).collect();
        links.dedup();
        links.join("<br>")
    };
    let peaks: Vec<BlockId> = biome.peaks.iter().map(|&(_, block)| block).collect();
    let mut rows = vec![
        (ctx.tr("wiki-fact-surface"), blocks(&[biome.surface])),
        (
            ctx.tr("wiki-fact-subsurface"),
            format!(
                "{} ({})",
                blocks(&[biome.subsurface]),
                escape(&ctx.tr_with(
                    "wiki-depth",
                    &[("depth", biome.subsurface_depth.to_string())]
                ))
            ),
        ),
        (ctx.tr("wiki-fact-underwater"), blocks(&[biome.underwater])),
    ];
    if !peaks.is_empty() {
        rows.push((ctx.tr("wiki-fact-peaks"), blocks(&peaks)));
    }
    let features = feature_blocks(ctx, biome);
    if !features.is_empty() {
        rows.push((ctx.tr("wiki-fact-features"), blocks(&features)));
    }
    rows.extend([
        (ctx.tr("wiki-fact-frozen"), yes_no(biome.frozen)),
        (ctx.tr("wiki-fact-spawn"), yes_no(biome.spawn)),
        (ctx.tr("wiki-fact-id"), format!("<code>{}</code>", biome.id)),
        (ctx.tr("wiki-fact-added"), escape(&ctx.added_in(key))),
    ]);
    rows
}

/// The blocks its trees and plants are made of.
fn feature_blocks(ctx: &Ctx<'_>, biome: &Biome) -> Vec<BlockId> {
    biome
        .features
        .iter()
        .filter_map(|&(index, _)| ctx.data.worldgen.features.get(index))
        .flat_map(|feature| match feature {
            Feature::Tree { log, leaves, .. } => vec![*log, *leaves],
            Feature::Pillar { block, .. } => vec![*block],
            Feature::Ore { .. } => Vec::new(),
        })
        .collect()
}

/// Where the biome sits on the climate maps (each from -1 to 1).
fn climate(ctx: &Ctx<'_>, biome: &Biome) -> String {
    let c = &biome.climate;
    let rows: String = [
        ("wiki-climate-temperature", c.temperature),
        ("wiki-climate-humidity", c.humidity),
        ("wiki-climate-continentalness", c.continentalness),
        ("wiki-climate-erosion", c.erosion),
    ]
    .iter()
    .map(|(key, value)| {
        format!(
            "<tr><th>{}</th><td>{value:+.2}</td></tr>",
            escape(&ctx.tr(key))
        )
    })
    .collect();
    format!(
        "<h2>{}</h2><p>{}</p><table class=\"data\">{rows}</table>",
        escape(&ctx.tr("wiki-climate")),
        escape(&ctx.tr("wiki-climate-help"))
    )
}

fn index(ctx: &Ctx<'_>, biomes: &[Biome]) -> Page {
    let path = "biomes/index.html".to_owned();
    let cards: String = biomes
        .iter()
        .map(|biome| {
            let title = name(ctx, &biome.id);
            format!(
                "<li>{}<a href=\"{}\">{}</a></li>",
                image(ctx, &path, "biomes", &id_path(&biome.id), &title, ""),
                link(&path, &page_path(&biome.id)),
                escape(&title)
            )
        })
        .collect();
    Page {
        path,
        title: ctx.tr("wiki-nav-biomes"),
        body: format!("<ul class=\"cards\">{cards}</ul>"),
    }
}
