use super::{Content, ItemKind};

impl Content {
    /// Fluent keys every language must define for this content.
    pub fn translation_keys(&self) -> Vec<String> {
        let blocks = self
            .blocks
            .iter()
            .filter(|(key, _, block)| block.family(*key) == *key)
            .map(|(_, id, _)| id.translation_key("block"));
        let items = self
            .items
            .iter()
            .filter_map(|(_, id, item)| match item.kind {
                ItemKind::Flat(_) => Some(id.translation_key("item")),
                ItemKind::Block(_) => None,
            });
        let categories = self
            .blocks
            .iter()
            .filter_map(|(_, _, block)| block.category.as_ref())
            .chain(
                self.items
                    .iter()
                    .filter_map(|(_, _, item)| item.category.as_ref()),
            )
            .map(|category| category.translation_key("category"));
        let subtitles = self
            .cinematics
            .values()
            .flat_map(|shot| &shot.keyframes)
            .filter_map(|key| key.subtitle.clone());
        let sounds = self.sounds.subtitle_keys().cloned();
        let tools = self
            .tool_kinds()
            .into_iter()
            .map(|k| k.translation_key("tool"));
        let tags = self.tags.iter().map(|(id, _)| id.translation_key("tag"));
        let tiers = self.gear.tiers.keys().map(|id| id.translation_key("tier"));
        let weights = self
            .gear
            .weights
            .keys()
            .map(|id| id.translation_key("armor-weight"));
        let slots = self.gear.slot_families.keys();
        let slots = slots.map(|id| id.translation_key("upgrade-slot"));
        let upgrades = self.upgrades.list().iter();
        let upgrades = upgrades.map(|u| u.id.translation_key("upgrade"));
        let stats = self.upgrades.stats.keys();
        let stats = stats.map(|id| id.translation_key("item-stat"));
        let survival = &self.survival;
        let effects = survival
            .effects
            .keys()
            .map(|id| id.translation_key("effect"));
        let damage = survival
            .damage
            .keys()
            .map(|id| id.translation_key("damage"));
        let mut keys: Vec<String> = blocks
            .chain(tags)
            .chain(items)
            .chain(tools)
            .chain(tiers)
            .chain(weights)
            .chain(slots)
            .chain(upgrades)
            .chain(stats)
            .chain(categories)
            .chain(subtitles)
            .chain(sounds)
            .chain(effects)
            .chain(damage)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }
}
