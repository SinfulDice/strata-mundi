//! What the interface tells about an item beyond its name: the tool a block needs, what a
//! tool mines.

use super::{BlockTool, Content, ItemId, ItemKind, ToolDef};
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
        }
        lines.join("\n")
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
