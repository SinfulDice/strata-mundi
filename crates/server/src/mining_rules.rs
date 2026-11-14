//! How long each block takes to break with the held item, and what it gives.

use strata_core::audio::Rng;
use strata_core::content::{
    Block, BlockId, Content, ItemId, LootContext, LootTable, Registry, ToolDef,
};

#[derive(Debug, Clone, Default)]
pub struct MiningRules {
    /// Hardness and tool of each block, by `BlockId`.
    blocks: Vec<Block>,
    /// Loot of each block, by `BlockId` (the block itself when its pack names none).
    loot: Vec<LootTable>,
    /// Tool of each item, by `ItemId`.
    tools: Vec<Option<ToolDef>>,
}

impl MiningRules {
    pub fn new(content: &Content) -> Self {
        Self {
            blocks: content.blocks.iter().map(|(_, _, b)| b.clone()).collect(),
            loot: content
                .blocks
                .iter()
                .map(|(id, _, _)| {
                    content
                        .block_loot(id)
                        .cloned()
                        .unwrap_or_else(LootTable::itself)
                })
                .collect(),
            tools: tools(&content.items),
        }
    }

    pub fn tool(&self, item: Option<ItemId>) -> Option<&ToolDef> {
        self.tools.get(usize::from(item?.0))?.as_ref()
    }

    /// Ticks of hitting that break `block` with `held` in hand; `None` = unbreakable.
    pub fn break_ticks(&self, block: BlockId, held: Option<ItemId>) -> Option<u32> {
        self.blocks
            .get(usize::from(block.0))?
            .break_ticks(self.tool(held))
    }

    /// Items given by breaking `block` with `held`: nothing without the tool it requires.
    pub fn loot(
        &self,
        block: BlockId,
        held: Option<ItemId>,
        itself: Option<ItemId>,
        rng: &mut Rng,
    ) -> Vec<(ItemId, u16)> {
        let tool = self.tool(held);
        let index = usize::from(block.0);
        let harvested = self.blocks.get(index).is_some_and(|b| b.harvests(tool));
        match (harvested, self.loot.get(index)) {
            (true, Some(table)) => table.roll(&LootContext { tool, itself }, rng),
            _ => Vec::new(),
        }
    }
}

fn tools(items: &Registry<ItemId, strata_core::content::Item>) -> Vec<Option<ToolDef>> {
    items.iter().map(|(_, _, item)| item.tool.clone()).collect()
}
