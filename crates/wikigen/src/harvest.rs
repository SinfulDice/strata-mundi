//! Breaking facts of a block page: hardness, tool, time by hand and loot, all read from
//! the packs.

use crate::blocks::block_link;
use crate::html::escape;
use crate::site::Ctx;
use strata_core::content::{Block, BlockId, ItemId, ItemKind, LootTable, TICKS_PER_SECOND};

/// Rows of the infobox, already escaped.
pub fn facts(ctx: &Ctx<'_>, from: &str, key: BlockId, block: &Block) -> Vec<(String, String)> {
    let content = &ctx.data.content;
    // A negative hardness means the block cannot be broken (outside creative).
    let Some(ticks) = block.break_ticks(None) else {
        return vec![(
            ctx.tr("wiki-fact-hardness"),
            escape(&ctx.tr("wiki-unbreakable")),
        )];
    };
    let mut rows = vec![(ctx.tr("wiki-fact-hardness"), format!("{}", block.hardness))];
    let tool = block.tool.as_ref();
    let required = tool.filter(|tool| tool.required).map(|tool| {
        let name = content.tool_name(&tool.kind, ctx.lang);
        match tool.level {
            0 => name,
            level => ctx.tr_with(
                "wiki-tool-level",
                &[("tool", name), ("level", level.to_string())],
            ),
        }
    });
    let none = || ctx.tr("wiki-tool-none");
    rows.push((
        ctx.tr("wiki-fact-tool"),
        escape(&required.unwrap_or_else(none)),
    ));
    if let Some(best) = tool.filter(|tool| !tool.required) {
        let name = content.tool_name(&best.kind, ctx.lang);
        rows.push((ctx.tr("wiki-fact-best-tool"), escape(&name)));
    }
    let seconds = ticks as f32 / TICKS_PER_SECOND as f32;
    let time = ctx.tr_with("wiki-seconds", &[("seconds", format!("{seconds:.2}"))]);
    rows.push((ctx.tr("wiki-fact-break-time"), escape(&time)));
    rows.push((ctx.tr("wiki-fact-loot"), loot(ctx, from, key)));
    rows
}

/// What breaking it gives (with the right tool), and the tools the loot depends on.
fn loot(ctx: &Ctx<'_>, from: &str, key: BlockId) -> String {
    let content = &ctx.data.content;
    let itself = content
        .block_name(key)
        .and_then(|family| content.items.key(family));
    let fallback = LootTable::itself();
    let table = content.block_loot(key).unwrap_or(&fallback);
    let items: Vec<String> = table
        .possible_items(itself)
        .into_iter()
        .map(|item| item_label(ctx, from, item))
        .collect();
    let mut text = match items.is_empty() {
        true => escape(&ctx.tr("wiki-loot-nothing")),
        false => items.join(", "),
    };
    for kind in table.tool_kinds() {
        let tool = content.tool_name(kind, ctx.lang);
        text.push_str(&format!(
            " ({})",
            escape(&ctx.tr_with("wiki-loot-with", &[("tool", tool)]))
        ));
    }
    text
}

fn item_label(ctx: &Ctx<'_>, from: &str, item: ItemId) -> String {
    let content = &ctx.data.content;
    match content.items.get(item).map(|entry| entry.kind) {
        Some(ItemKind::Block(block)) => block_link(ctx, from, block),
        _ => escape(&content.item_name(item, ctx.lang)),
    }
}
