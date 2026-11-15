//! Item pages (items that are not blocks: sticks, coal, buckets, boats…): infobox from
//! the pack, how to craft them and what they are used for; and the list by category.
//! Written texts are optional for now: shown when `wiki-content` has one.

use crate::blocks::block_link;
use crate::html::{escape, image, infobox, link};
use crate::site::{Ctx, Page, id_path};
use std::collections::BTreeMap;
use strata_core::content::{Item, ItemId, ItemKind};
use strata_core::id::Identifier;

pub fn pages(ctx: &Ctx<'_>) -> Vec<Page> {
    let items = flat_items(ctx);
    let mut pages: Vec<Page> = items
        .iter()
        .map(|&(key, id, item)| page(ctx, key, id, item))
        .collect();
    pages.push(index(ctx, &items));
    pages
}

fn flat_items<'a>(ctx: &Ctx<'a>) -> Vec<(ItemId, &'a Identifier, &'a Item)> {
    ctx.data
        .content
        .items
        .iter()
        .filter(|(_, _, item)| matches!(item.kind, ItemKind::Flat(_)))
        .collect()
}

fn page_path(id: &Identifier) -> String {
    format!("items/{}.html", id_path(id))
}

/// A link to the page of an item (a block's item links to the block's page).
pub fn item_link(ctx: &Ctx<'_>, from: &str, item: ItemId) -> String {
    let content = &ctx.data.content;
    let Some(entry) = content.items.get(item) else {
        return String::new();
    };
    let Some(id) = content.items.identifier(item) else {
        return String::new();
    };
    if let ItemKind::Block(block) = entry.kind {
        return block_link(ctx, from, block);
    }
    let label = content.item_name(item, ctx.lang);
    format!(
        "<a href=\"{}\">{}{}</a>",
        link(from, &page_path(id)),
        image(ctx, from, "icons", &id_path(id), &label, "icon"),
        escape(&label)
    )
}

fn page(ctx: &Ctx<'_>, key: ItemId, id: &Identifier, item: &Item) -> Page {
    let path = page_path(id);
    let title = ctx.data.content.item_name(key, ctx.lang);
    let picture = image(ctx, &path, "icons", &id_path(id), &title, "");
    let box_html = infobox(&title, &picture, &facts(ctx, id, item));
    let text = ctx
        .data
        .texts
        .html(ctx.lang, &format!("items/{}", id_path(id)))
        .unwrap_or_default();
    let crafting =
        crate::recipes::section(ctx, &path, key) + &crate::cooking::sections(ctx, &path, key);
    let uses = crate::recipes::uses(ctx, &path, key);
    Page {
        path,
        title,
        body: format!("{box_html}{text}{crafting}{uses}"),
    }
}

fn facts(ctx: &Ctx<'_>, id: &Identifier, item: &Item) -> Vec<(String, String)> {
    let category = item
        .category
        .as_ref()
        .map_or_else(String::new, |c| ctx.tr(&c.translation_key("category")));
    let mut rows = vec![
        (ctx.tr("wiki-fact-category"), escape(&category)),
        (ctx.tr("wiki-fact-stack"), item.max_stack.to_string()),
    ];
    if let Some(uses) = item.durability {
        rows.push((ctx.tr("wiki-fact-durability"), uses.to_string()));
    }
    rows.extend(crate::gear::facts(ctx, item));
    rows.push((ctx.tr("wiki-fact-id"), format!("<code>{id}</code>")));
    rows
}

/// Every item, grouped by category, with icons.
fn index(ctx: &Ctx<'_>, items: &[(ItemId, &Identifier, &Item)]) -> Page {
    let path = "items/index.html".to_owned();
    let mut groups: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for &(key, _, item) in items {
        let category = item.category.as_ref().map_or_else(
            || ctx.tr("wiki-other"),
            |c| ctx.tr(&c.translation_key("category")),
        );
        let entry = format!("<li>{}</li>", item_link(ctx, &path, key));
        groups.entry(category).or_default().push(entry);
    }
    let body: String = groups
        .iter()
        .map(|(category, entries)| {
            format!(
                "<h2>{}</h2><ul class=\"grid\">{}</ul>",
                escape(category),
                entries.concat()
            )
        })
        .collect();
    Page {
        path,
        title: ctx.tr("wiki-nav-items"),
        body,
    }
}
