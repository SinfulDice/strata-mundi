//! The « Storage » page: every chest and barrel of a tier, its slots and upgrade slots,
//! read from the packs.

use crate::blocks::block_link;
use crate::html::escape;
use crate::site::Ctx;

pub fn table(ctx: &Ctx<'_>, from: &str) -> String {
    let content = &ctx.data.content;
    let rows: String = (content.blocks.iter())
        .filter(|(_, _, block)| {
            block
                .container
                .as_ref()
                .is_some_and(|c| c.upgradable.is_some())
        })
        .filter_map(|(key, _, _)| Some((key, content.slot_layout(key)?)))
        .map(|(key, layout)| {
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                block_link(ctx, from, key),
                layout.storage,
                layout.upgrades
            )
        })
        .collect();
    let head = ["block", "slots", "upgrades"]
        .map(|column| {
            format!(
                "<th>{}</th>",
                escape(&ctx.tr(&format!("wiki-storage-{column}")))
            )
        })
        .concat();
    format!(
        "<h2>{}</h2><table class=\"data\"><tr>{head}</tr>{rows}</table>",
        escape(&ctx.tr("wiki-storage-table"))
    )
}
