//! What the interface tells about an item beyond its name: the tool a block needs, what a
//! tool mines.

use super::{BlockTool, Content, ItemId, ItemKind, ToolDef};
use crate::id::Identifier;

impl Content {
    /// The item's name, then one line per fact (tool needed, tool given), joined by `\n`.
    pub fn item_tooltip(&self, item: ItemId, language: &str) -> String {
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
            ("speed", format!("{}", tool.speed)),
        ];
        self.translations
            .text_with(language, "mining-tool-line", &args)
    }
}
