//! How long each block takes to break with the held item, and what it gives.

use strata_core::content::{
    Block, BlockId, Content, ItemId, LootContext, LootTable, Registry, ToolDef, Worked,
};
use strata_core::rng::Rng;

#[derive(Debug, Clone, Default)]
pub struct MiningRules {
    /// Hardness and tool of each block, by `BlockId`.
    blocks: Vec<Block>,
    /// Loot of each block, by `BlockId` (the block itself when its pack names none).
    loot: Vec<LootTable>,
    /// Tool of each item, by `ItemId`.
    tools: Vec<Option<ToolDef>>,
    /// Name of each block, by `BlockId` (a fire that works blocks, by its name).
    names: Vec<strata_core::id::Identifier>,
    /// Crack stages of the packs: progress is reported when the stage changes.
    pub crack_stages: usize,
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
            names: content
                .blocks
                .iter()
                .map(|(_, name, _)| name.clone())
                .collect(),
            crack_stages: content.crack_stages.len().max(1),
        }
    }

    pub fn tool(&self, item: Option<ItemId>) -> Option<&ToolDef> {
        self.tools.get(usize::from(item?.0))?.as_ref()
    }

    pub fn block(&self, block: BlockId) -> Option<&Block> {
        self.blocks.get(usize::from(block.0))
    }

    /// Ticks of hitting that break `block` with the `held` tool (a hand when `None`);
    /// `None` = unbreakable.
    pub fn break_ticks(&self, block: BlockId, held: Option<&ToolDef>) -> Option<u32> {
        self.block(block)?.break_ticks(held)
    }

    /// What using the `held` tool on `block` turns it into (a hoe on dirt).
    pub fn worked(&self, block: BlockId, held: Option<&ToolDef>) -> Option<Worked> {
        self.block(block)?.worked_by(&held?.kinds)
    }

    /// What lighting `fire` on `block` turns it into (an unlit campfire lit again).
    pub fn lit(&self, block: BlockId, fire: BlockId) -> Option<Worked> {
        let fire = self.names.get(usize::from(fire.0))?;
        self.block(block)?.worked_by([fire])
    }

    /// How far around `block` breaking it with `held` also breaks its like (a sickle).
    pub fn sweep(&self, block: BlockId, held: Option<&ToolDef>) -> u8 {
        match (held, self.block(block)) {
            (Some(tool), Some(block)) => tool.sweep_on(block),
            _ => 0,
        }
    }

    /// The tool kind `block` is best broken with.
    pub fn tool_kind(&self, block: BlockId) -> Option<&strata_core::id::Identifier> {
        Some(&self.blocks.get(usize::from(block.0))?.tool.as_ref()?.kind)
    }

    /// Items given by breaking `block` with `held`: nothing without the tool it requires.
    pub fn loot(
        &self,
        block: BlockId,
        tool: Option<&ToolDef>,
        itself: Option<ItemId>,
        rng: &mut Rng,
    ) -> Vec<(ItemId, u16)> {
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
