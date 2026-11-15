//! What the held stack does once its upgrades count, and the miner's hammer: the area is
//! worked out here from the face hit and the player's facing, broken with the block hit,
//! each block wearing the hammer, their loot in one heap per item.

use super::Game;
use crate::ecs::Swing;
use crate::loader::ColumnLoader;
use std::sync::Arc;
use strata_core::content::{BlockId, ItemId, ToolDef};
use strata_core::coords::BlockPos;
use strata_core::inventory::ItemStack;
use strata_core::updates::BlockChange;
use strata_core::upgrade::{AreaSize, ItemStats, chosen_area, swing_cells, swing_ticks, with_area};

impl Game {
    /// The selected stack, unless broken (then no better than a hand).
    pub(super) fn held_stack(&self) -> Option<&ItemStack> {
        let inventory = self.ecs.inventories.get(self.player)?;
        inventory
            .selected_stack()
            .filter(|stack| !stack.is_broken())
    }

    /// The stats of the stack in slot `slot`, upgrades counted (cached).
    pub(super) fn slot_stats(&self, slot: usize, stack: &ItemStack) -> Arc<ItemStats> {
        let (items, upgrades) = (self.rules.items(), &self.rules.upgrades);
        self.stats_cache.get(slot, stack, items, upgrades)
    }

    fn held_stats(&self) -> Option<(&ItemStack, Arc<ItemStats>)> {
        let inventory = self.ecs.inventories.get(self.player)?;
        let stack = self.held_stack()?;
        Some((stack, self.slot_stats(inventory.selected_index(), stack)))
    }

    /// The tool in hand, its upgrades counted.
    pub(super) fn held_tool(&self) -> Option<ToolDef> {
        self.held_stats()?.1.tool.clone()
    }

    /// The hammer area chosen for the stack in hand (1×1 for any other item).
    pub(super) fn held_area(&self) -> AreaSize {
        self.held_stats()
            .map_or(AreaSize::SINGLE, |(stack, stats)| {
                chosen_area(stack, &stats.areas)
            })
    }

    /// A smaller (or larger) area unlocked on the hammer in hand.
    pub(super) fn choose_area(&mut self, index: u8) {
        let Some((stack, stats)) = self.held_stats() else {
            return;
        };
        if usize::from(index) >= stats.areas.len() {
            return;
        }
        let chosen = with_area(stack, usize::from(index));
        self.with_inventory(|inventory, _| {
            *inventory.selected_slot_mut() = Some(chosen);
        });
    }

    /// The cells a hit on `pos` breaks besides it (a hammer's area).
    pub(super) fn swing_cells(
        &self,
        loader: &ColumnLoader,
        pos: BlockPos,
        swing: Swing,
        tool: Option<&ToolDef>,
    ) -> Vec<BlockPos> {
        let Some(tool) = tool else {
            return Vec::new();
        };
        let blocks = |at: BlockPos| {
            let block = loader.block(at).filter(|&b| b != BlockId::AIR)?;
            self.rules.mining.block(block)
        };
        swing_cells(pos, swing.face, (swing.area, swing.yaw), tool, blocks)
            .into_iter()
            .filter(|at| at.is_valid(self.border))
            .collect()
    }

    /// Ticks the hit takes: those of the hardest block it breaks.
    pub(super) fn swing_ticks(
        &self,
        loader: &ColumnLoader,
        cells: &[BlockPos],
        tool: Option<&ToolDef>,
        single: u32,
    ) -> u32 {
        let Some(tool) = tool else {
            return single;
        };
        let blocks = |at: BlockPos| self.rules.mining.block(loader.block(at)?);
        swing_ticks(cells, single, tool, blocks)
    }

    /// Breaks the other cells of a hammer's area with `tool`: one use each (none for
    /// an instant block), while the hammer lasts; their loot joins the heap of the block
    /// hit.
    pub(super) fn break_area(
        &mut self,
        loader: &mut ColumnLoader,
        (cells, tool): (Vec<BlockPos>, &ToolDef),
        heap: &mut Vec<(ItemId, u16)>,
        changes: &mut Vec<BlockChange>,
    ) {
        let hammer = self.held_item();
        for at in cells {
            if hammer.is_none() || self.held_item() != hammer {
                break;
            }
            let Some(block) = loader.block(at).filter(|&b| b != BlockId::AIR) else {
                continue;
            };
            for (item, count) in self.roll_loot_with(block, at, Some(tool)) {
                add_to_heap(heap, item, count);
            }
            self.set_block(loader, at, BlockId::AIR, changes);
            if self.rules.mining.break_ticks(block, None) != Some(0) {
                self.wear_held(1);
            }
        }
    }
}

pub(super) fn add_to_heap(heap: &mut Vec<(ItemId, u16)>, item: ItemId, count: u16) {
    match heap.iter_mut().find(|(kept, _)| *kept == item) {
        Some((_, total)) => *total = total.saturating_add(count),
        None => heap.push((item, count)),
    }
}
