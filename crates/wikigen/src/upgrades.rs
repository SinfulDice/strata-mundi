//! The upgrades table of the « Upgrades » page, read from the packs: each level, its
//! effects and what it costs at the anvil.

use crate::html::escape;
use crate::items::item_link;
use crate::site::Ctx;

pub fn table(ctx: &Ctx<'_>, from: &str) -> String {
    let content = &ctx.data.content;
    let rows: String = content
        .upgrades
        .list()
        .iter()
        .flat_map(|upgrade| {
            let name = content.upgrade_name(&upgrade.id, ctx.lang);
            let slot = ctx.tr(&upgrade.slot.translation_key("upgrade-slot"));
            upgrade.levels.iter().enumerate().map(move |(index, level)| {
                let effects: Vec<String> = level
                    .effects
                    .iter()
                    .map(|effect| content.effect_text(effect, ctx.lang))
                    .collect();
                let material = level
                    .material
                    .items()
                    .first()
                    .map_or_else(String::new, |&item| item_link(ctx, from, item));
                format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{} × {material}</td><td>{}</td></tr>",
                    escape(&name),
                    escape(&slot),
                    index + 1,
                    escape(&effects.join(", ")),
                    level.count,
                    level.xp
                )
            })
        })
        .collect();
    let head = ["upgrade", "slot", "level", "effects", "material", "xp"]
        .map(|column| {
            format!(
                "<th>{}</th>",
                escape(&ctx.tr(&format!("wiki-upgrades-{column}")))
            )
        })
        .concat();
    format!(
        "<h2>{}</h2><table class=\"data\"><tr>{head}</tr>{rows}</table>",
        escape(&ctx.tr("wiki-upgrades-table"))
    )
}
