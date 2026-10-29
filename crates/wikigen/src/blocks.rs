//! Block pages: infobox from the pack, where the block generates (from the world
//! generation content), the written text and the history; and the list by category.

use crate::html::{escape, image, infobox, link};
use crate::pages::history_section;
use crate::site::{Ctx, Page, id_path};
use std::collections::BTreeMap;
use strata_core::content::{Block, BlockId, RenderKind};
use strata_core::id::Identifier;
use strata_worldgen::Feature;

pub fn pages(ctx: &Ctx<'_>) -> Vec<Page> {
    let blocks = families(ctx);
    let mut pages: Vec<Page> = blocks
        .iter()
        .map(|(id, block)| page(ctx, id, block))
        .collect();
    pages.push(index(ctx, &blocks));
    pages
}

/// One page per block family (lying logs share the upright log's page), air excluded.
fn families<'a>(ctx: &Ctx<'a>) -> Vec<(&'a Identifier, &'a Block)> {
    ctx.data
        .content
        .blocks
        .iter()
        .filter(|(key, _, block)| *key != BlockId::AIR && block.family(*key) == *key)
        .map(|(_, id, block)| (id, block))
        .collect()
}

pub fn page_path(id: &Identifier) -> String {
    format!("blocks/{}.html", id_path(id))
}

pub fn name(ctx: &Ctx<'_>, id: &Identifier) -> String {
    ctx.tr(&id.translation_key("block"))
}

/// A link with the block's icon, from the page at `from`.
pub fn block_link(ctx: &Ctx<'_>, from: &str, block: BlockId) -> String {
    let content = &ctx.data.content;
    let Some(block_def) = content.blocks.get(block) else {
        return String::new();
    };
    let family = block_def.family(block);
    let Some(id) = content.blocks.identifier(family) else {
        return String::new();
    };
    let label = name(ctx, id);
    format!(
        "<a href=\"{}\">{}{}</a>",
        link(from, &page_path(id)),
        image(ctx, from, "icons", &id_path(id), &label, "icon"),
        escape(&label)
    )
}

fn page(ctx: &Ctx<'_>, id: &Identifier, block: &Block) -> Page {
    let path = page_path(id);
    let key = format!("blocks/{}", id_path(id));
    let title = name(ctx, id);
    let picture = image(ctx, &path, "icons", &id_path(id), &title, "");
    let box_html = infobox(&title, &picture, &facts(ctx, id, block, &key));
    let found = generation(ctx, &path, id);
    let body = format!(
        "{box_html}{}{found}{}",
        ctx.text(&key),
        history_section(ctx, &key)
    );
    Page { path, title, body }
}

fn facts(ctx: &Ctx<'_>, id: &Identifier, block: &Block, key: &str) -> Vec<(String, String)> {
    let yes_no = |value: bool| ctx.tr(if value { "wiki-yes" } else { "wiki-no" });
    let category = block
        .category
        .as_ref()
        .map_or_else(String::new, |c| ctx.tr(&c.translation_key("category")));
    let look = ctx.tr(match block.render {
        RenderKind::Opaque => "wiki-render-opaque",
        RenderKind::Cutout => "wiki-render-cutout",
        RenderKind::Translucent => "wiki-render-translucent",
        RenderKind::Invisible => "wiki-render-invisible",
    });
    // A negative hardness means the block cannot be broken (outside creative).
    let hardness = if block.hardness < 0.0 {
        escape(&ctx.tr("wiki-unbreakable"))
    } else {
        format!("{}", block.hardness)
    };
    let mut rows = vec![(ctx.tr("wiki-fact-category"), escape(&category))];
    if block.has_item {
        rows.push((ctx.tr("wiki-fact-stack"), block.max_stack.to_string()));
    } else {
        rows.push((ctx.tr("wiki-fact-item"), escape(&ctx.tr("wiki-no"))));
    }
    rows.extend([
        (ctx.tr("wiki-fact-hardness"), hardness),
        (ctx.tr("wiki-fact-tool"), escape(&ctx.tr("wiki-tool-none"))),
        (ctx.tr("wiki-fact-solid"), escape(&yes_no(block.solid))),
        (ctx.tr("wiki-fact-render"), escape(&look)),
        (ctx.tr("wiki-fact-light"), block.light_opacity.to_string()),
        (
            ctx.tr("wiki-fact-oriented"),
            escape(&yes_no(block.orientation.is_some())),
        ),
        (ctx.tr("wiki-fact-id"), format!("<code>{id}</code>")),
        (ctx.tr("wiki-fact-added"), escape(&ctx.added_in(key))),
    ]);
    rows
}

/// Where the world generation puts this block, read from the generation content.
fn generation(ctx: &Ctx<'_>, from: &str, id: &Identifier) -> String {
    let Some(block) = ctx.data.content.blocks.key(id) else {
        return String::new();
    };
    let worldgen = &ctx.data.worldgen;
    let terrain = &worldgen.terrain;
    let mut places: Vec<String> = Vec::new();
    let mut say = |key: &str, args: &[(&str, String)]| places.push(escape(&ctx.tr_with(key, args)));
    if block == terrain.stone {
        say("wiki-gen-stone", &[]);
    }
    if block == terrain.deep_stone {
        say(
            "wiki-gen-deep",
            &[("level", terrain.deep_level.to_string())],
        );
    }
    if block == terrain.bedrock {
        say("wiki-gen-bedrock", &[]);
    }
    if block == terrain.water {
        say(
            "wiki-gen-water",
            &[("level", terrain.sea_level.to_string())],
        );
    }
    if block == terrain.ice {
        say("wiki-gen-ice", &[]);
    }
    for feature in &worldgen.features {
        if let Feature::Ore {
            block: ore,
            deep_block,
            min_y,
            max_y,
            ..
        } = feature
            && let Some((min, max)) = ore_range(
                *ore,
                *deep_block,
                (*min_y, *max_y),
                block,
                terrain.deep_level,
            )
        {
            say(
                "wiki-gen-ore",
                &[("min", min.to_string()), ("max", max.to_string())],
            );
        }
    }
    let biomes = biome_roles(ctx, from, block);
    for (role, list) in &biomes {
        places.push(format!("{} {}", escape(&ctx.tr(role)), list.join(", ")));
    }
    if places.is_empty() {
        return String::new();
    }
    let items: String = places.iter().map(|p| format!("<li>{p}</li>")).collect();
    format!(
        "<h2>{}</h2><ul>{items}</ul>",
        escape(&ctx.tr("wiki-generation"))
    )
}

/// Heights where `block` appears for this ore: an ore with a deep variant takes the
/// variant's look in deep stone (below `deep_level`) and its own above.
fn ore_range(
    ore: BlockId,
    deep: Option<BlockId>,
    (min, max): (i32, i32),
    block: BlockId,
    deep_level: i32,
) -> Option<(i32, i32)> {
    let (min, max) = match deep {
        Some(deep) if deep == block => (min, max.min(deep_level - 1)),
        Some(_) if ore == block => (min.max(deep_level), max),
        None if ore == block => (min, max),
        _ => return None,
    };
    (min <= max).then_some((min, max))
}

/// Biomes using the block, by role (surface, under the surface, trees…), as links.
fn biome_roles(ctx: &Ctx<'_>, from: &str, block: BlockId) -> BTreeMap<&'static str, Vec<String>> {
    let worldgen = &ctx.data.worldgen;
    let mut roles: BTreeMap<&'static str, Vec<String>> = BTreeMap::new();
    for biome in worldgen.biomes() {
        let link = crate::biomes::biome_link(ctx, from, &biome.id);
        let mut add = |role: &'static str| roles.entry(role).or_default().push(link.clone());
        if biome.surface == block {
            add("wiki-gen-surface");
        }
        if biome.subsurface == block {
            add("wiki-gen-subsurface");
        }
        if biome.underwater == block {
            add("wiki-gen-underwater");
        }
        if biome.peaks.iter().any(|(_, peak)| *peak == block) {
            add("wiki-gen-peaks");
        }
        let in_feature = biome.features.iter().any(|&(index, _)| {
            worldgen
                .features
                .get(index)
                .is_some_and(|f| feature_uses(f, block))
        });
        if in_feature {
            add("wiki-gen-feature");
        }
    }
    for list in roles.values_mut() {
        list.dedup();
    }
    roles
}

fn feature_uses(feature: &Feature, block: BlockId) -> bool {
    match feature {
        Feature::Tree { log, leaves, .. } => *log == block || *leaves == block,
        Feature::Pillar { block: pillar, .. } => *pillar == block,
        Feature::Ore { .. } => false,
    }
}

/// Every block, grouped by category, with icons.
fn index(ctx: &Ctx<'_>, blocks: &[(&Identifier, &Block)]) -> Page {
    let path = "blocks/index.html".to_owned();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, block) in blocks {
        let category = block.category.as_ref().map_or_else(
            || ctx.tr("wiki-other"),
            |c| ctx.tr(&c.translation_key("category")),
        );
        let label = name(ctx, id);
        groups.entry(category).or_default().push(format!(
            "<li><a href=\"{}\">{}{}</a></li>",
            link(&path, &page_path(id)),
            image(ctx, &path, "icons", &id_path(id), &label, "icon"),
            escape(&label)
        ));
    }
    let body: String = groups
        .iter()
        .map(|(category, items)| {
            format!(
                "<h2>{}</h2><ul class=\"grid\">{}</ul>",
                escape(category),
                items.concat()
            )
        })
        .collect();
    Page {
        path,
        title: ctx.tr("wiki-nav-blocks"),
        body,
    }
}
