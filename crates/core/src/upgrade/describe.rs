//! What the interface tells about upgrades: their effects, what a stack holds, what the
//! anvil offers and what it costs.

use super::defs::Effect;
use super::{Blocked, Choice, installed, stack_stats};
use crate::content::Content;
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
        let number = |value: f32| tr.short_decimal(language, f64::from(value));
        let (key, args) = match *effect {
            Effect::Damage(p) => ("upgrade-effect-damage", vec![("value", number(p))]),
            Effect::MiningSpeed(p) => ("upgrade-effect-mining-speed", vec![("value", number(p))]),
            Effect::Durability(p) => ("upgrade-effect-durability", vec![("value", number(p))]),
            Effect::Protection(points) => (
                "upgrade-effect-protection",
                vec![("value", points.to_string())],
            ),
            Effect::Lighten(p) => ("upgrade-effect-lighten", vec![("value", number(p))]),
            Effect::Area(width, height) => (
                "upgrade-effect-area",
                vec![("width", width.to_string()), ("height", height.to_string())],
            ),
        };
        tr.text_with(language, key, &args)
    }

    /// « Sharpening 2: +20 % damage, +20 % mining speed »
    pub fn upgrade_level_text(&self, upgrade: &Identifier, level: u8, language: &str) -> String {
        let effects: Vec<String> = self
            .upgrades
            .get(upgrade)
            .and_then(|u| u.levels.get(usize::from(level).checked_sub(1)?))
            .map(|level| {
                let effects = level.effects.iter();
                effects.map(|e| self.effect_text(e, language)).collect()
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

    /// The upgrades a stack holds, what its stats come to with them, and the hammer
    /// area chosen.
    pub fn stack_upgrade_lines(&self, stack: &ItemStack, language: &str) -> Vec<String> {
        let held = installed(stack);
        let mut lines: Vec<String> = held
            .iter()
            .map(|(id, level)| self.upgrade_level_text(id, *level, language))
            .collect();
        let stats = stack_stats(stack, &self.items, &self.upgrades);
        if !held.is_empty() {
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
    pub fn choice_tooltip(&self, choice: &Choice<'_>, language: &str) -> String {
        let tr = &self.translations;
        let mut lines = vec![self.upgrade_level_text(&choice.upgrade.id, choice.level, language)];
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
        first: Option<crate::content::ItemId>,
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
