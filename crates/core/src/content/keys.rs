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
        let mut keys: Vec<String> = blocks
            .chain(tags)
            .chain(items)
            .chain(tools)
            .chain(tiers)
            .chain(weights)
            .chain(categories)
            .chain(subtitles)
            .chain(sounds)
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }
}
