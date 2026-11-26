//! The « Survival » page: what each food, drink and care item gives, and every kind of
//! damage with what lessens it, all read from the packs.

use crate::html::escape;
use crate::items::item_link;
use crate::site::Ctx;

pub fn tables(ctx: &Ctx<'_>, from: &str) -> String {
    consumables(ctx, from) + &damage(ctx)
}

fn consumables(ctx: &Ctx<'_>, from: &str) -> String {
    let content = &ctx.data.content;
    let number = |value: f32| format!("{}", (value * 10.0).round() / 10.0);
    let rows: String = (content.survival.consumables.iter())
        .map(|(&item, use_)| {
            let effects: Vec<String> = (use_.effects.iter())
                .map(|grant| {
                    let name = ctx.tr(&grant.effect.translation_key("effect"));
                    format!("{name} ({} s)", number(grant.seconds))
                })
                .chain(use_.cures.iter().map(|cured| {
                    let name = ctx.tr(&cured.translation_key("effect"));
                    ctx.tr_with("wiki-survival-cures", &[("effect", name)])
                }))
                .collect();
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                item_link(ctx, from, item),
                number(use_.food),
                number(use_.saturation),
                number(use_.thirst),
                number(f32::from(use_.ticks) / 20.0),
                escape(&effects.join(", "))
            )
        })
        .collect();
    table(
        ctx,
        "wiki-survival-consumables",
        &["item", "food", "saturation", "thirst", "seconds", "effects"],
        &rows,
    )
}

fn damage(ctx: &Ctx<'_>) -> String {
    let content = &ctx.data.content;
    let rows: String = (content.survival.damage.iter())
        .map(|(id, kind)| {
            let yes_no = |on: bool| ctx.tr(if on { "ui-on" } else { "ui-off" });
            let resistance = (kind.resistance.as_ref())
                .map(|stat| ctx.tr(&stat.translation_key("item-stat")))
                .unwrap_or_default();
            format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>",
                escape(&ctx.tr(&id.translation_key("damage"))),
                escape(&yes_no(kind.armor)),
                escape(&resistance),
                escape(&yes_no(!kind.ignores_invulnerability)),
            )
        })
        .collect();
    table(
        ctx,
        "wiki-survival-damage",
        &["kind", "armor", "resistance", "invulnerability"],
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
