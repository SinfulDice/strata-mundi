//! Survival breaking takes time. The server counts the ticks from `StartBreak` and lets
//! `Break` through only once the block has been hit long enough, with a little slack for
//! the network. Changing block or held item ends the break. The cracks it reports let
//! every client draw the work in progress.

use super::Game;
use crate::ecs::Mining;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::audio::Rng;
use strata_core::content::{BlockId, crack_stage};
use strata_core::coords::BlockPos;
use strata_core::inventory::ItemStack;
use strata_protocol::{CRACK_STAGES, NetEntity, ServerMessage};

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
    /// Survival: the player starts hitting the block at `pos`. Creative breaks at once.
    pub(super) fn start_break(&mut self, loader: &ColumnLoader, pos: BlockPos) {
        self.stop_break();
        if self.is_creative() || !pos.is_valid(self.border) || !self.in_reach(pos) {
            return;
        }
        let Some(block) = loader.block(pos).filter(|&b| b != BlockId::AIR) else {
            return;
        };
        let item = self.held_item();
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

    /// The player stopped hitting (or broke the block): the cracks go away.
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

    /// Each tick: the break ends if the block or the held item changed, or went out of
    /// reach; otherwise its cracks are reported when they grow.
    pub(super) fn mining_tick(&mut self, loader: &ColumnLoader) {
        let Some(mining) = self.ecs.controls.get(self.player).and_then(|c| c.mining) else {
            return;
        };
        let still = loader.block(mining.pos) == Some(mining.block)
            && self.held_item() == mining.item
            && self.in_reach(mining.pos);
        if !still {
            self.stop_break();
            return;
        }
        let elapsed = self.tick.saturating_sub(mining.started) as f32;
        let stage = crack_stage(elapsed, mining.needed as f32, CRACK_STAGES);
        if mining.stage == Some(stage) {
            return;
        }
        if let Some(Some(m)) = self
            .ecs
            .controls
            .get_mut(self.player)
            .map(|c| &mut c.mining)
        {
            m.stage = Some(stage);
        }
        self.report_cracks(mining.pos, Some(stage));
    }

    fn report_cracks(&mut self, pos: BlockPos, stage: Option<u8>) {
        let breaker = NetEntity(self.player.to_bits());
        self.messages.push(ServerMessage::Cracking {
            breaker,
            pos,
            stage,
        });
    }

    /// Survival: a broken block leaves its loot on the ground, and the tool wears.
    pub(super) fn drop_broken(&mut self, block: BlockId, pos: BlockPos) {
        if self.is_creative() {
            return;
        }
        let held = self.held_item();
        let itself = self.rules.block_item(block);
        let mut rng = Rng::new(self.loot_seed(pos));
        let loot = self.rules.mining.loot(block, held, itself, &mut rng);
        for (item, count) in loot {
            let stack = ItemStack::new(item, count, &self.rules);
            let velocity = DVec3::new(0.0, 0.1, 0.0);
            let at = super::containers::block_centre(pos) - DVec3::Y * 0.25;
            self.spawn_drop(stack, at, velocity);
        }
        let worn = self.rules.mining.tool(held).is_some()
            && self.rules.mining.break_ticks(block, None) != Some(0);
        if worn {
            self.wear_held(1);
        }
    }

    /// Same world, block and tick, same loot.
    fn loot_seed(&self, pos: BlockPos) -> u64 {
        let mix = |value: i32, factor: u64| u64::from(value as u32).wrapping_mul(factor);
        self.loot_seed
            ^ mix(pos.x, 0x9E37_79B9_7F4A_7C15)
            ^ mix(pos.y, 0xC2B2_AE3D_27D4_EB4F)
            ^ mix(pos.z, 0x1656_67B1_9E37_79F9)
            ^ self.tick.wrapping_mul(0xFF51_AFD7_ED55_8CCD)
    }
}
