//! Infobox rows of tools, weapons and armour, read from the pack: tier, mining level and
//! speed, damage, protection, weight and upgrade slots.

use crate::html::escape;
use crate::site::Ctx;
use strata_core::content::Item;

pub fn facts(ctx: &Ctx<'_>, item: &Item) -> Vec<(String, String)> {
    let content = &ctx.data.content;
    let number = |value: f32| {
        content
            .translations
            .short_decimal(ctx.lang, f64::from(value))
    };
    let mut rows = Vec::new();
    if let Some(tier) = &item.gear.tier {
        let name = ctx.tr(&tier.translation_key("tier"));
        rows.push((ctx.tr("wiki-fact-tier"), escape(&name)));
    }
    if let Some(tool) = &item.tool {
        let kinds: Vec<String> = tool
            .kinds
            .iter()
            .map(|kind| content.tool_name(kind, ctx.lang))
            .collect();
        rows.push((ctx.tr("wiki-fact-tool-kinds"), escape(&kinds.join(", "))));
        rows.push((ctx.tr("wiki-fact-mining-level"), tool.level.to_string()));
        rows.push((
            ctx.tr("wiki-fact-mining-speed"),
            format!("×{}", number(tool.speed)),
        ));
        if tool.sweep > 0 {
            rows.push((ctx.tr("wiki-fact-sweep"), tool.sweep.to_string()));
        }
    }
    if let Some(weapon) = &item.gear.weapon {
        rows.push((ctx.tr("wiki-fact-damage"), number(weapon.damage)));
        rows.push((
            ctx.tr("wiki-fact-attack-speed"),
            number(weapon.attack_speed),
        ));
    }
    if let Some(armor) = &item.gear.armor {
        let weight = ctx.tr(&armor.weight.translation_key("armor-weight"));
        rows.push((ctx.tr("wiki-fact-protection"), armor.protection.to_string()));
        rows.push((ctx.tr("wiki-fact-weight"), escape(&weight)));
        if armor.burden.speed > 0.0 {
            let value = number(armor.burden.speed * 100.0);
            let percent = ctx.tr_with("wiki-value-percent", &[("value", value)]);
            rows.push((ctx.tr("wiki-fact-slowdown"), escape(&percent)));
        }
    }
    if !item.gear.upgrade_slots.is_empty() {
        let slots = content.slot_counts(&item.gear.upgrade_slots, ctx.lang);
        rows.push((ctx.tr("wiki-fact-upgrade-slots"), escape(&slots)));
    }
    if item.gear.enchantability > 0 {
        let value = item.gear.enchantability.to_string();
        rows.push((ctx.tr("wiki-fact-enchantability"), value));
    }
    rows
}
