//! Survival breaking as the player sees it: the time counted while the attack button is
//! held on one block with one item, the cracks of the local break and those the server
//! reports for every breaker. The server keeps its own count and decides.

use crate::interaction::BlockInfo;
use crate::render::CrackDraw;
use crate::world::ClientWorld;
use glam::DVec3;
use std::collections::HashMap;
use strata_core::content::{BlockId, Face, ItemId, TICKS_PER_SECOND, crack_stage};
use strata_core::coords::BlockPos;
use strata_core::raycast::RayHit;
use strata_protocol::{CRACK_STAGES, NetEntity, WorldAction};

/// A hit sound and a fragment come this often while breaking (seconds), like Minecraft.
const HIT_PERIOD: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq)]
struct Break {
    pos: BlockPos,
    block: BlockId,
    item: Option<ItemId>,
    elapsed: f32,
    /// Seconds the block takes with that item.
    needed: f32,
    since_hit: f32,
}

#[derive(Debug, Default)]
pub struct Mining {
    current: Option<Break>,
    /// Cracks reported by the server, by breaker.
    reported: HashMap<NetEntity, (BlockPos, u8)>,
    /// Blocks hit since the last frame (sound and fragments), with the face hit.
    pub hits: Vec<(BlockPos, Face)>,
}

/// What `Mining::update` knows of the frame.
pub struct MiningInput<'a> {
    pub world: &'a ClientWorld,
    pub info: &'a BlockInfo,
    /// The aimed block, if the attack button is held on one.
    pub target: Option<RayHit>,
    pub held: Option<ItemId>,
    /// The pause after a break is over: a new block may be started.
    pub ready: bool,
}

impl Mining {
    /// Counts the time on the aimed block; returns what to tell the server: start on a
    /// new block, stop when the button or the aim leaves, break once the time is up.
    pub fn update(&mut self, dt: f32, input: &MiningInput<'_>) -> Option<WorldAction> {
        let Some(hit) = input.target else {
            return self.current.take().map(|_| WorldAction::StopBreak);
        };
        let block = input.world.block_at(hit.block);
        let same = self.current.as_ref().is_some_and(|current| {
            current.pos == hit.block && current.block == block && current.item == input.held
        });
        if same {
            return self.advance(dt, hit);
        }
        if !input.ready {
            return self.current.take().map(|_| WorldAction::StopBreak);
        }
        let ticks = input.info.break_ticks(block, input.held);
        let Some(ticks) = ticks.filter(|_| block != BlockId::AIR) else {
            return self.current.take().map(|_| WorldAction::StopBreak);
        };
        if ticks == 0 {
            self.current = None;
            return Some(WorldAction::Break { pos: hit.block });
        }
        self.current = Some(Break {
            pos: hit.block,
            block,
            item: input.held,
            elapsed: 0.0,
            needed: ticks as f32 / TICKS_PER_SECOND as f32,
            since_hit: HIT_PERIOD,
        });
        Some(WorldAction::StartBreak { pos: hit.block })
    }

    fn advance(&mut self, dt: f32, hit: RayHit) -> Option<WorldAction> {
        let current = self.current.as_mut()?;
        current.elapsed += dt;
        current.since_hit += dt;
        if current.since_hit >= HIT_PERIOD {
            current.since_hit = 0.0;
            self.hits.push((hit.block, hit.face));
        }
        // The margin keeps fifteen frames of 0.05 s at 0.75 s despite rounding.
        if current.elapsed + 1e-4 < current.needed {
            return None;
        }
        self.current = None;
        Some(WorldAction::Break { pos: hit.block })
    }

    /// Creative, menus, cinematics: any break in progress is dropped.
    pub fn cancel(&mut self) -> Option<WorldAction> {
        self.current.take().map(|_| WorldAction::StopBreak)
    }

    /// The server's news about someone's cracks.
    pub fn report(&mut self, breaker: NetEntity, pos: BlockPos, stage: Option<u8>) {
        match stage {
            Some(stage) => self.reported.insert(breaker, (pos, stage)),
            None => self.reported.remove(&breaker),
        };
    }

    /// Cracks to draw: the local break and the reported ones, the furthest stage per
    /// block, never on air.
    pub fn cracks(&self, world: &ClientWorld) -> Vec<(BlockPos, u8)> {
        let local = self.current.map(|current| {
            let stage = crack_stage(current.elapsed, current.needed, CRACK_STAGES);
            (current.pos, stage)
        });
        let mut cracks: Vec<(BlockPos, u8)> = Vec::new();
        for (pos, stage) in local.into_iter().chain(self.reported.values().copied()) {
            if world.block_at(pos) == BlockId::AIR {
                continue;
            }
            match cracks.iter_mut().find(|(p, _)| *p == pos) {
                Some(entry) => entry.1 = entry.1.max(stage),
                None => cracks.push((pos, stage)),
            }
        }
        cracks
    }

    /// The cracks as the renderer draws them: on each block's hit box, relative to the
    /// camera at `eyes`.
    pub fn draws(&self, world: &ClientWorld, info: &BlockInfo, eyes: DVec3) -> Vec<CrackDraw> {
        self.cracks(world)
            .into_iter()
            .filter_map(|(pos, stage)| {
                let shape = info.hit_box(world.block_at(pos))?;
                Some(CrackDraw::on(pos, shape, stage, eyes))
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "mining_tests.rs"]
mod tests;
