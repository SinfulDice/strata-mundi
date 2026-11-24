//! What the interface tells about upgrades: their effects, what a stack holds, what the
//! anvil offers and what it costs.

use super::defs::{Effect, StatOp};
use super::{Blocked, Choice, applies, installed, stack_stats};
use crate::content::{Content, ItemId};
use crate::id::Identifier;
use crate::inventory::ItemStack;

impl Content {
    /// « Sharpening », from `upgrade-<namespace>-<name>`.
    pub fn upgrade_name(&self, upgrade: &Identifier, language: &str) -> String {
        self.translations
            .text(language, &upgrade.translation_key("upgrade"))
    }

    /// « +20 % damage », one brick.
    pub fn effect_text(&self, effect: &Effect, language: &str) -> String {
        let tr = &self.translations;
        match effect {
            Effect::Stat { stat, op, value } => {
                let sign = if *value < 0.0 { "−" } else { "+" };
                let number = tr.short_decimal(language, f64::from(value.abs()));
                let args = [
                    ("value", format!("{sign}{number}")),
                    (
                        "stat",
                        tr.text(language, &stat.translation_key("item-stat")),
                    ),
                ];
                let key = match op {
                    StatOp::Add => "upgrade-effect-add",
                    StatOp::Percent => "upgrade-effect-percent",
                };
                tr.text_with(language, key, &args)
            }
            Effect::Area(width, height) => {
                let args = [("width", width.to_string()), ("height", height.to_string())];
                tr.text_with(language, "upgrade-effect-area", &args)
            }
            Effect::Magnet { radius, .. } => {
                let args = [("radius", tr.short_decimal(language, f64::from(*radius)))];
                tr.text_with(language, "upgrade-effect-magnet", &args)
            }
            Effect::Sort => tr.text(language, "upgrade-effect-sort"),
            Effect::Filter => tr.text(language, "upgrade-effect-filter"),
            Effect::Compress => tr.text(language, "upgrade-effect-compress"),
            Effect::Void => tr.text(language, "upgrade-effect-void"),
            Effect::Pickup => tr.text(language, "upgrade-effect-pickup"),
            Effect::Feed => tr.text(language, "upgrade-effect-feed"),
            Effect::Crafting => tr.text(language, "upgrade-effect-crafting"),
            Effect::Tank { buckets } => {
                let args = [("buckets", buckets.to_string())];
                tr.text_with(language, "upgrade-effect-tank", &args)
            }
        }
    }

    /// « Sharpening 2: +20 % damage, +20 % mining speed »; on `item`, only the bricks
    /// that change it, and none while its stats are not known.
    pub fn upgrade_level_text(
        &self,
        upgrade: &Identifier,
        level: u8,
        item: Option<ItemId>,
        language: &str,
    ) -> String {
        let entry = item.and_then(|item| self.items.get(item));
        let known = item.is_none_or(|item| self.stats_known(item));
        let effects: Vec<String> = self
            .upgrades
            .get(upgrade)
            .and_then(|u| u.levels.get(usize::from(level).checked_sub(1)?))
            .filter(|_| known)
            .map(|level| {
                let effects = level.effects.iter();
                effects
                    .filter(|e| entry.is_none_or(|entry| applies(e, entry)))
                    .map(|e| self.effect_text(e, language))
                    .collect()
            })
            .unwrap_or_default();
        let args = [
            ("name", self.upgrade_name(upgrade, language)),
            ("level", level.to_string()),
            ("effects", effects.join(", ")),
        ];
        self.translations
            .text_with(language, "upgrade-level-line", &args)
    }

    /// What an item that is an upgrade does once in a block's slot: « Furnace module:
    /// −25 % cooking time, −20 % cookings per fuel ».
    pub fn upgrade_item_line(&self, upgrade: &Identifier, language: &str) -> Option<String> {
        let upgrade = self.upgrades.get(upgrade)?;
        let level = upgrade.levels.first()?;
        let effects: Vec<String> = (level.effects.iter())
            .map(|effect| self.effect_text(effect, language))
            .collect();
        let tr = &self.translations;
        let args = [
            (
                "slot",
                tr.text(language, &upgrade.slot.translation_key("upgrade-slot")),
            ),
            ("effects", effects.join(", ")),
        ];
        Some(tr.text_with(language, "upgrade-item-line", &args))
    }

    /// The upgrades a stack holds, what its stats come to with them, and the hammer
    /// area chosen.
    pub fn stack_upgrade_lines(&self, stack: &ItemStack, language: &str) -> Vec<String> {
        let held = installed(stack);
        let item = Some(stack.item);
        let mut lines: Vec<String> = held
            .iter()
            .map(|(id, level)| self.upgrade_level_text(id, *level, item, language))
            .collect();
        let stats = stack_stats(stack, &self.items, &self.upgrades);
        if !held.is_empty() && self.stats_known(stack.item) {
            lines.extend(self.upgraded_stats(&stats, language));
        }
        if stats.areas.len() > 1 {
            let area = super::chosen_area(stack, &stats.areas);
            let args = [
                ("width", area.width.to_string()),
                ("height", area.height.to_string()),
            ];
            lines.push(
                self.translations
                    .text_with(language, "hud-hammer-area", &args),
            );
        }
        lines
    }

    /// « With its upgrades: 6.6 damage, speed ×6.6, 7 protection ».
    fn upgraded_stats(&self, stats: &super::ItemStats, language: &str) -> Option<String> {
        let tr = &self.translations;
        let number = |value: f32| tr.short_decimal(language, f64::from(value));
        let mut parts = Vec::new();
        if let Some(weapon) = &stats.weapon {
            let args = [("value", number(weapon.damage))];
            parts.push(tr.text_with(language, "upgrade-stat-damage", &args));
        }
        if let Some(tool) = &stats.tool {
            let args = [("value", number(tool.speed))];
            parts.push(tr.text_with(language, "upgrade-stat-speed", &args));
        }
        if let Some(armor) = &stats.armor {
            let args = [("value", armor.protection.to_string())];
            parts.push(tr.text_with(language, "upgrade-stat-protection", &args));
        }
        (!parts.is_empty()).then(|| {
            let args = [("stats", parts.join(", "))];
            tr.text_with(language, "upgrade-stats-line", &args)
        })
    }

    /// What the anvil offers: the level and its effects, its price, and why it is not
    /// possible yet.
    pub fn choice_tooltip(&self, choice: &Choice<'_>, item: ItemId, language: &str) -> String {
        let tr = &self.translations;
        let upgrade = &choice.upgrade.id;
        let mut lines = vec![self.upgrade_level_text(upgrade, choice.level, Some(item), language)];
        let material = choice.cost.material.items().first().copied();
        let args = [
            ("count", choice.cost.count.to_string()),
            ("material", self.ingredient_name(material, choice, language)),
            ("xp", choice.cost.xp.to_string()),
        ];
        lines.push(tr.text_with(language, "upgrade-cost-line", &args));
        if let Some(blocked) = &choice.blocked {
            lines.push(self.blocked_text(blocked, language));
        }
        lines.join("\n")
    }

    fn ingredient_name(
        &self,
        first: Option<ItemId>,
        choice: &Choice<'_>,
        language: &str,
    ) -> String {
        match (&choice.cost.material.tag, first) {
            (Some(tag), _) => self
                .translations
                .text(language, &tag.translation_key("tag")),
            (None, Some(item)) => self.item_name(item, language),
            (None, None) => String::new(),
        }
    }

    pub fn blocked_text(&self, blocked: &Blocked, language: &str) -> String {
        let tr = &self.translations;
        match blocked {
            Blocked::NoSlot => tr.text(language, "upgrade-blocked-slot"),
            Blocked::Requires(id) => tr.text_with(
                language,
                "upgrade-blocked-requires",
                &[("name", self.upgrade_name(id, language))],
            ),
            Blocked::Excludes(id) => tr.text_with(
                language,
                "upgrade-blocked-excludes",
                &[("name", self.upgrade_name(id, language))],
            ),
        }
    }
}
