//! « Crafting » section of a block page: every recipe making it, drawn as its grid of
//! item icons, read from the packs.

use crate::html::{escape, image};
use crate::site::Ctx;
use strata_core::content::ItemId;
use strata_core::crafting::{Ingredient, MAX_GRID, Recipe, RecipeShape};

/// The section, empty when no recipe makes `item`.
pub fn section(ctx: &Ctx<'_>, from: &str, item: ItemId) -> String {
    let content = &ctx.data.content;
    let grids: String = content
        .recipes
        .making(item)
        .map(|(_, recipe)| grid(ctx, from, recipe))
        .collect();
    if grids.is_empty() {
        return String::new();
    }
    format!(
        "<h2>{}</h2><div class=\"recipes\">{grids}</div>",
        escape(&ctx.tr("wiki-crafting"))
    )
}

/// The things made with `item` (a link to each result's page).
pub fn uses(ctx: &Ctx<'_>, from: &str, item: ItemId) -> String {
    let content = &ctx.data.content;
    let mut results: Vec<ItemId> = content
        .recipes
        .iter()
        .filter(|(_, _, recipe)| recipe.ingredients().iter().any(|i| i.accepts(item)))
        .map(|(_, _, recipe)| recipe.result)
        .collect();
    results.sort();
    results.dedup();
    if results.is_empty() {
        return String::new();
    }
    let links: String = results
        .into_iter()
        .map(|result| format!("<li>{}</li>", crate::items::item_link(ctx, from, result)))
        .collect();
    format!(
        "<h2>{}</h2><ul class=\"grid\">{links}</ul>",
        escape(&ctx.tr("wiki-used-in"))
    )
}

fn grid(ctx: &Ctx<'_>, from: &str, recipe: &Recipe) -> String {
    let side = recipe.grid_size().max(2);
    let mut cells: Vec<Option<&Ingredient>> = vec![None; side * side];
    for (cell, ingredient) in recipe.cells_in(side) {
        cells[cell] = Some(ingredient);
    }
    let rows: String = cells
        .chunks(side)
        .map(|row| {
            let cells: String = row
                .iter()
                .map(|cell| {
                    format!(
                        "<td>{}</td>",
                        cell.map_or_else(String::new, |i| ingredient(ctx, from, i))
                    )
                })
                .collect();
            format!("<tr>{cells}</tr>")
        })
        .collect();
    let count = match recipe.count {
        1 => String::new(),
        count => format!("<span class=\"count\">{count}</span>"),
    };
    let note = match (&recipe.shape, side) {
        (RecipeShape::Shapeless(_), _) => escape(&ctx.tr("wiki-recipe-shapeless")),
        (_, MAX_GRID) => escape(&ctx.tr("wiki-recipe-table")),
        _ => String::new(),
    };
    format!(
        "<figure class=\"recipe\"><table class=\"grid\">{rows}</table><span class=\"arrow\">→</span>\
         <span class=\"cell\">{}{count}</span><figcaption>{note}</figcaption></figure>",
        icon(
            ctx,
            from,
            recipe.result,
            &ctx.data.content.item_name(recipe.result, ctx.lang)
        ),
    )
}

/// The first item of a tag stands for it; its name says « any … ».
pub fn ingredient(ctx: &Ctx<'_>, from: &str, ingredient: &Ingredient) -> String {
    let Some(&first) = ingredient.items().first() else {
        return String::new();
    };
    let name = match &ingredient.tag {
        Some(tag) => ctx.tr(&tag.translation_key("tag")),
        None => ctx.data.content.item_name(first, ctx.lang),
    };
    icon(ctx, from, first, &name)
}

pub fn icon(ctx: &Ctx<'_>, from: &str, item: ItemId, name: &str) -> String {
    let Some(id) = ctx.data.content.items.identifier(item) else {
        return String::new();
    };
    let key = format!("{}/{}", id.namespace(), id.name());
    let picture = image(ctx, from, "icons", &key, name, "");
    let shown = if picture.is_empty() {
        escape(name)
    } else {
        picture
    };
    format!("<span title=\"{}\">{shown}</span>", escape(name))
}
