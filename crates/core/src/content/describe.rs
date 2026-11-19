//! What the interface tells about an item beyond its name: the tool a block needs, what a
//! tool mines.

use super::{BlockTool, Content, Item, ItemAction, ItemId, ItemKind, ToolDef};
use crate::gear::count_slots;
use crate::id::Identifier;
use crate::lang::LANGUAGES;
use std::collections::HashMap;

/// Tooltips of every item in every language, written once when the content loads.
pub type TooltipCache = HashMap<(ItemId, &'static str), String>;

impl Content {
    /// The item's name, then one line per fact (tool needed, tool given), joined by `\n`;
    /// from the cache for the game's languages.
    pub fn item_tooltip(&self, item: ItemId, language: &str) -> String {
        let cached = LANGUAGES
            .iter()
            .find(|&&known| known == language)
            .and_then(|&known| self.tooltips.get(&(item, known)));
        match cached {
            Some(text) => text.clone(),
            None => self.write_tooltip(item, language),
        }
    }

    /// Fills `tooltips` (after loading, once translations are known).
    pub(super) fn cache_tooltips(&mut self) {
        let items: Vec<ItemId> = self.items.iter().map(|(key, _, _)| key).collect();
        let tooltips = items
            .into_iter()
            .flat_map(|item| LANGUAGES.iter().map(move |&language| (item, language)))
            .map(|(item, language)| ((item, language), self.write_tooltip(item, language)))
            .collect();
        self.tooltips = tooltips;
    }

    fn write_tooltip(&self, item: ItemId, language: &str) -> String {
        let mut lines = vec![self.item_name(item, language)];
        if let Some(entry) = self.items.get(item) {
            let block = match entry.kind {
                ItemKind::Block(block) => self.blocks.get(block),
                ItemKind::Flat(_) => None,
            };
            if let Some(tool) = block.and_then(|block| block.tool.as_ref()) {
                lines.push(self.block_tool_text(tool, language));
            }
            if let Some(tool) = &entry.tool {
                lines.push(self.tool_text(tool, language));
            }
            let upgrade = entry.upgrade.as_ref();
            lines.extend(upgrade.and_then(|u| self.upgrade_item_line(u, language)));
            if self.stats_known(item) {
                lines.extend(self.gear_lines(entry, language));
            }
            lines.extend(self.made_at_line(item, language));
        }
        lines.join("\n")
    }

    /// « Made at: Forge » for what benches make. Bench recipes are all known until the
    /// knowledge of milestone 13 decides here too.
    fn made_at_line(&self, item: ItemId, language: &str) -> Option<String> {
        let mut stations: Vec<String> = (self.benches.list().iter())
            .filter(|recipe| recipe.result == item)
            .filter_map(|recipe| self.bench_name(&recipe.bench, language))
            .collect();
        stations.sort();
        stations.dedup();
        let args = [("stations", stations.join(", "))];
        (!stations.is_empty()).then(|| self.translations.text_with(language, "gear-made-at", &args))
    }

    /// The name of the block that opens `bench`.
    pub fn bench_name(&self, bench: &Identifier, language: &str) -> Option<String> {
        let opens = |block: &super::Block| matches!(&block.station, Some(super::StationDef::Bench(b)) if b == bench);
        let (_, id, _) = self.blocks.iter().find(|(_, _, block)| opens(block))?;
        Some(
            self.translations
                .text(language, &id.translation_key("block")),
        )
    }

    /// Sweep, use on blocks, weapon, armour and upgrade slots of an item.
    fn gear_lines(&self, item: &Item, language: &str) -> Vec<String> {
        let tr = &self.translations;
        let number = |value: f32| tr.short_decimal(language, f64::from(value));
        let mut lines = Vec::new();
        if let Some(sweep) = item.tool.as_ref().map(|t| t.sweep).filter(|&s| s > 0) {
            let blocks = [("blocks", sweep.to_string())];
            lines.push(tr.text_with(language, "mining-tool-sweep", &blocks));
        }
        if item.action == Some(ItemAction::Work) {
            lines.push(tr.text(language, "gear-works-line"));
        }
        if let Some(weapon) = &item.gear.weapon {
            let args = [
                ("damage", number(weapon.damage)),
                ("speed", number(weapon.attack_speed)),
            ];
            lines.push(tr.text_with(language, "gear-weapon-line", &args));
        }
        if let Some(armor) = &item.gear.armor {
            let weight = tr.text(language, &armor.weight.translation_key("armor-weight"));
            let args = [("points", armor.protection.to_string()), ("weight", weight)];
            lines.push(tr.text_with(language, "gear-armor-line", &args));
        }
        if !item.gear.worn.is_empty() {
            let effects: Vec<String> = (item.gear.worn.iter())
                .map(|effect| self.effect_text(effect, language))
                .collect();
            let args = [("effects", effects.join(", "))];
            lines.push(tr.text_with(language, "gear-worn-line", &args));
        }
        if !item.gear.upgrade_slots.is_empty() {
            let slots = [(
                "slots",
                self.slot_counts(&item.gear.upgrade_slots, language),
            )];
            lines.push(tr.text_with(language, "gear-upgrade-line", &slots));
        }
        lines
    }

    /// The single check of whether an item's stats (damage, protection, slots) are
    /// known: all are until the knowledge system of milestone 13 answers here.
    pub fn stats_known(&self, _item: ItemId) -> bool {
        true
    }

    /// « artisanal ×2 », one family after the other.
    pub fn slot_counts(&self, slots: &[Identifier], language: &str) -> String {
        let tr = &self.translations;
        let counts: Vec<String> = count_slots(slots)
            .into_iter()
            .map(|(family, count)| {
                let name = tr.text(language, &family.translation_key("upgrade-slot"));
                let args = [("family", name), ("count", count.to_string())];
                tr.text_with(language, "upgrade-slot-count", &args)
            })
            .collect();
        counts.join(", ")
    }

    /// Translated name of a tool kind (`strata:pickaxe` → « pioche »).
    pub fn tool_name(&self, kind: &Identifier, language: &str) -> String {
        self.translations
            .text(language, &kind.translation_key("tool"))
    }

    /// « Needs a pickaxe of level 2 or more », or « Faster with: axe ».
    pub fn block_tool_text(&self, tool: &BlockTool, language: &str) -> String {
        let name = ("tool", self.tool_name(&tool.kind, language));
        let level = ("level", tool.level.to_string());
        let (key, args) = match (tool.required, tool.level) {
            (true, 0) => ("mining-needs-tool", vec![name]),
            (true, _) => ("mining-needs-tool-level", vec![name, level]),
            (false, _) => ("mining-best-tool", vec![name]),
        };
        self.translations.text_with(language, key, &args)
    }

    fn tool_text(&self, tool: &ToolDef, language: &str) -> String {
        let kinds: Vec<String> = tool
            .kinds
            .iter()
            .map(|kind| self.tool_name(kind, language))
            .collect();
        let args = [
            ("tools", kinds.join(", ")),
            ("level", tool.level.to_string()),
            (
                "speed",
                self.translations
                    .short_decimal(language, f64::from(tool.speed)),
            ),
        ];
        self.translations
            .text_with(language, "mining-tool-line", &args)
    }
}
