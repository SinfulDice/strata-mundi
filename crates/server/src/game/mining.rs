//! Survival breaking takes time. The server counts the ticks from `StartBreak` and
//! breaks the block itself once it has been hit long enough. The client's `Break` only
//! comes first when the client is ahead (accepted with a little slack for the network).
//! Changing block or held item ends the break. The progress it reports lets every client
//! draw the cracks.

use super::Game;
use crate::ecs::Mining;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::content::{BlockId, ItemId, crack_progress, crack_stage};
use strata_core::coords::BlockPos;
use strata_core::inventory::{ItemRules, ItemStack};
use strata_core::rng::Rng;
use strata_core::updates::BlockChange;
use strata_protocol::{NetEntity, ServerMessage};

/// A break may come this many ticks early (network jitter)…
const EARLY_TICKS: u32 = 2;
/// … or this share of the break time early, whichever is more.
const EARLY_DIVISOR: u32 = 10;

/// Whether `elapsed` ticks of hitting are enough for a block taking `needed`.
pub fn long_enough(elapsed: u64, needed: u32) -> bool {
    let slack = EARLY_TICKS.max(needed / EARLY_DIVISOR);
    elapsed + u64::from(slack) >= u64::from(needed)
}

impl Game {
    /// Survival: the player starts hitting the block at `pos` (a start on the block
    /// already being hit, with the same item, changes nothing). Creative breaks at once.
    pub(super) fn start_break(&mut self, loader: &ColumnLoader, pos: BlockPos) {
        let block = loader.block(pos).filter(|&b| b != BlockId::AIR);
        let item = self.held_item();
        let current = self.ecs.controls.get(self.player).and_then(|c| c.mining);
        if current.is_some_and(|m| m.pos == pos && Some(m.block) == block && m.item == item) {
            return;
        }
        self.stop_break();
        if self.is_creative() || !pos.is_valid(self.border) || !self.in_reach(pos) {
            return;
        }
        let Some(block) = block else {
            return;
        };
        let Some(needed) = self.rules.mining.break_ticks(block, item) else {
            return;
        };
        let started = self.tick;
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.mining = Some(Mining {
                pos,
                block,
                item,
                started,
                needed,
                stage: None,
            });
        }
    }

    /// The player stopped hitting (or the block broke): the cracks go away.
    pub(super) fn stop_break(&mut self) {
        let Some(control) = self.ecs.controls.get_mut(self.player) else {
            return;
        };
        if let Some(Mining {
            pos,
            stage: Some(_),
            ..
        }) = control.mining.take()
        {
            self.report_cracks(pos, None);
        }
    }

    /// Survival: whether `block` at `pos` may break now, given how long it was hit with
    /// the item in hand. Instant blocks need no start.
    pub(super) fn hit_long_enough(&self, pos: BlockPos, block: BlockId) -> bool {
        let item = self.held_item();
        let Some(needed) = self.rules.mining.break_ticks(block, item) else {
            return false;
        };
        if needed == 0 {
            return true;
        }
        let mining = self.ecs.controls.get(self.player).and_then(|c| c.mining);
        mining.is_some_and(|m| {
            m.pos == pos
                && m.block == block
                && m.item == item
                && long_enough(self.tick.saturating_sub(m.started), needed)
        })
    }

    /// Each tick: the break ends in creative, or if the block or the held item changed,
    /// or went out of reach; it breaks the block once its time is up; otherwise its
    /// progress is reported when the crack stage changes.
    pub(super) fn mining_tick(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        let Some(mining) = self.ecs.controls.get(self.player).and_then(|c| c.mining) else {
            return Vec::new();
        };
        let still = !self.is_creative()
            && loader.block(mining.pos) == Some(mining.block)
            && self.held_item() == mining.item
            && self.in_reach(mining.pos);
        if !still {
            self.stop_break();
            return Vec::new();
        }
        let elapsed = self.tick.saturating_sub(mining.started);
        if elapsed >= u64::from(mining.needed) {
            return self.finish_break(loader, mining.pos, mining.block);
        }
        let progress = crack_progress(elapsed as f32, mining.needed as f32);
        let stage = crack_stage(progress, self.rules.mining.crack_stages);
        if mining.stage != Some(stage) {
            if let Some(Some(m)) = self
                .ecs
                .controls
                .get_mut(self.player)
                .map(|c| &mut c.mining)
            {
                m.stage = Some(stage);
            }
            self.report_cracks(mining.pos, Some(progress));
        }
        Vec::new()
    }

    /// Breaks `block` at `pos`: its loot falls, the cracks go, neighbours react.
    pub(super) fn finish_break(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        block: BlockId,
    ) -> Vec<BlockChange> {
        let mut changes = Vec::new();
        self.drop_broken(block, pos);
        self.stop_break();
        self.set_block(loader, pos, BlockId::AIR, &mut changes);
        self.sweep_around(loader, pos, block, &mut changes);
        changes
    }

    fn report_cracks(&mut self, pos: BlockPos, progress: Option<u8>) {
        let breaker = NetEntity(self.player.to_bits());
        self.messages.push(ServerMessage::Cracking {
            breaker,
            pos,
            progress,
        });
    }

    /// Survival: a broken block leaves its loot on the ground, in stacks no bigger than
    /// the item's, and the tool wears.
    pub(super) fn drop_broken(&mut self, block: BlockId, pos: BlockPos) {
        if self.is_creative() {
            return;
        }
        let loot = self.roll_loot(block, pos);
        self.spawn_loot(loot, pos);
        let held = self.held_item();
        let worn = self.rules.mining.tool(held).is_some()
            && self.rules.mining.break_ticks(block, None) != Some(0);
        if worn {
            self.wear_held(1);
        }
    }

    /// What breaking `block` at `pos` with the held item gives.
    pub(super) fn roll_loot(&self, block: BlockId, pos: BlockPos) -> Vec<(ItemId, u16)> {
        let itself = self.rules.block_item(block);
        let mut rng = Rng::new(self.loot_seed(pos));
        let held = self.held_item();
        self.rules.mining.loot(block, held, itself, &mut rng)
    }

    /// Items laid on the ground at `pos`, in stacks no bigger than the item's.
    pub(super) fn spawn_loot(&mut self, loot: Vec<(ItemId, u16)>, pos: BlockPos) {
        let at = super::containers::block_centre(pos) - DVec3::Y * 0.25;
        for (item, count) in loot {
            let max = self.rules.max_stack(item).max(1);
            for size in split(count, max) {
                let stack = ItemStack::new(item, size, &self.rules);
                self.spawn_drop(stack, at, DVec3::new(0.0, 0.1, 0.0));
            }
        }
    }

    /// Same world, dimension, block and tick, same loot.
    fn loot_seed(&self, pos: BlockPos) -> u64 {
        let mix = |value: i32, factor: u64| u64::from(value as u32).wrapping_mul(factor);
        self.loot_seed
            ^ mix(pos.x, 0x9E37_79B9_7F4A_7C15)
            ^ mix(pos.y, 0xC2B2_AE3D_27D4_EB4F)
            ^ mix(pos.z, 0x1656_67B1_9E37_79F9)
            ^ self.tick.wrapping_mul(0xFF51_AFD7_ED55_8CCD)
    }
}

/// `count` in piles of `max` at most: 20 by 16 → 16, 4.
pub fn split(count: u16, max: u16) -> impl Iterator<Item = u16> {
    let max = max.max(1);
    (0..count.div_ceil(max)).map(move |i| (count - i * max).min(max))
}
