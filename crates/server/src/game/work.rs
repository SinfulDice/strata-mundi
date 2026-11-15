//! Tools used beyond breaking: a hoe tills and a chisel carves the block aimed at, a
//! sickle sweeps the plants around the one cut; worn armour slows the player.

use super::Game;
use super::hammer::add_to_heap;
use crate::loader::ColumnLoader;
use strata_core::content::{BlockId, Face, ItemId};
use strata_core::coords::BlockPos;
use strata_core::gear::worn_burden;
use strata_core::inventory::ARMOR;
use strata_core::updates::BlockChange;

impl Game {
    /// Survival or creative: the held tool turns the block at `pos` into what its
    /// `worked` says for one of the tool's kinds; the tool wears.
    pub(super) fn work_block(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
    ) -> Vec<BlockChange> {
        let mut changes = Vec::new();
        let Some(block) = loader.block(pos) else {
            return changes;
        };
        let Some(worked) = self.rules.mining.worked(block, self.held_tool().as_ref()) else {
            return changes;
        };
        let covered =
            worked.open_above && loader.block(pos.neighbour(Face::Up)) != Some(BlockId::AIR);
        if covered || !pos.is_valid(self.border) || !self.in_reach(pos) || !self.take_action() {
            return changes;
        }
        self.set_block(loader, pos, worked.into, &mut changes);
        self.wear_held(1);
        changes
    }

    /// Survival: a sweeping tool (sickle) that broke `block` at `pos` also breaks the
    /// instant blocks of the same tool kind around it, one block up and down, for one use;
    /// what they give falls in one heap per item where the cut began.
    pub(super) fn sweep_around(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        block: BlockId,
        changes: &mut Vec<BlockChange>,
    ) {
        if self.is_creative() {
            return;
        }
        let tool = self.held_tool();
        let held = tool.as_ref();
        let reach = i32::from(self.rules.mining.sweep(block, held));
        let mining = std::sync::Arc::clone(&self.rules.mining);
        let Some(kind) = mining.tool_kind(block).filter(|_| reach > 0) else {
            return;
        };
        let mut heap: Vec<(ItemId, u16)> = Vec::new();
        let mut swept = false;
        for at in around(pos, reach) {
            let Some(other) = loader.block(at).filter(|&b| b != BlockId::AIR) else {
                continue;
            };
            let alike =
                mining.tool_kind(other) == Some(kind) && mining.break_ticks(other, held) == Some(0);
            if alike && at.is_valid(self.border) {
                for (item, count) in self.roll_loot(other, at) {
                    add_to_heap(&mut heap, item, count);
                }
                self.set_block(loader, at, BlockId::AIR, changes);
                swept = true;
            }
        }
        self.spawn_loot(heap, pos);
        if swept {
            self.wear_held(1);
        }
    }

    /// Each tick: the player walks as fast as their armour lets them.
    pub(super) fn apply_armour_weight(&mut self) {
        let Some(inventory) = self.ecs.inventories.get(self.player) else {
            return;
        };
        let worn = ARMOR.filter_map(|slot| inventory.slots.get(slot));
        let speed = worn_burden(worn, self.rules.items(), &self.rules.upgrades).speed_factor();
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.movement.speed = f64::from(speed);
        }
    }
}

/// The cells up to `reach` blocks around `pos` on x and z and one up and down, `pos` aside.
fn around(pos: BlockPos, reach: i32) -> impl Iterator<Item = BlockPos> {
    (-reach..=reach)
        .flat_map(move |dx| {
            (-1..=1).flat_map(move |dy| (-reach..=reach).map(move |dz| (dx, dy, dz)))
        })
        .filter(|&offset| offset != (0, 0, 0))
        .map(move |(dx, dy, dz)| BlockPos::new(pos.x + dx, pos.y + dy, pos.z + dz))
}
