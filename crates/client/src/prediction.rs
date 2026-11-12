//! Block clicks shown before the server answers. A predicted cell keeps its predicted
//! block until the server acknowledges the click; meanwhile the server's changes for that
//! cell are kept aside, and the latest one replaces the prediction on acknowledgement.
//! The server stays the only judge: this is only what the client draws.

use crate::interaction::BlockInfo;
use crate::world::ClientWorld;
use std::collections::HashMap;
use strata_core::content::{BlockId, Face};
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::physics::{Aabb, Body};
use strata_core::updates::Cell;
use strata_protocol::{ActionId, MAX_PENDING_ACTIONS, WorldAction};

struct Pending {
    /// The latest click that predicted this cell.
    action: ActionId,
    /// The cell as the server last said it was.
    server: BlockId,
}

#[derive(Default)]
pub struct BlockPredictions {
    last_id: u32,
    /// The server answers in the order of the clicks: this one and all before it.
    last_answered: u32,
    pending: HashMap<BlockPos, Pending>,
}

impl BlockPredictions {
    pub fn next_id(&mut self) -> ActionId {
        self.last_id = self.last_id.wrapping_add(1);
        ActionId(self.last_id)
    }

    /// Whether click `id` may be predicted: past `MAX_PENDING_ACTIONS` clicks without an
    /// answer, the server refuses them, so they are not shown before it answers.
    pub fn may_predict(&self, id: ActionId) -> bool {
        let waiting = id.0.wrapping_sub(self.last_answered);
        waiting as usize <= MAX_PENDING_ACTIONS
    }

    /// Click `action` shows a new block at `pos`, which the server says holds `server`.
    pub fn predict(&mut self, action: ActionId, pos: BlockPos, server: BlockId) {
        self.pending
            .entry(pos)
            .and_modify(|pending| pending.action = action)
            .or_insert(Pending { action, server });
    }

    /// The server changed `pos`: `true` to show it now, `false` when a prediction covers
    /// the cell (the change is shown when the click is acknowledged).
    pub fn server_block(&mut self, pos: BlockPos, block: BlockId) -> bool {
        match self.pending.get_mut(&pos) {
            Some(pending) => {
                pending.server = block;
                false
            }
            None => true,
        }
    }

    /// The server finished `action`: the cells it was the last to predict get the
    /// server's block back (a cell predicted again waits for the later answer).
    pub fn acknowledge(&mut self, action: ActionId) -> Vec<(BlockPos, BlockId)> {
        self.last_answered = action.0;
        let mut settled = Vec::new();
        self.pending.retain(|&pos, pending| {
            let done = pending.action == action;
            if done {
                settled.push((pos, pending.server));
            }
            !done
        });
        settled
    }

    /// A column came again (or left): its blocks are the server's, whatever was predicted.
    pub fn forget_column(&mut self, column: ColumnPos) {
        self.pending.retain(|pos, _| pos.column() != column);
    }
}

/// What the client expects the click to change, with the same rules as the server
/// (reach aside); `None` when it expects a refusal or cannot tell.
pub fn predicted_change(
    info: &BlockInfo,
    world: &ClientWorld,
    action: WorldAction,
    player: Option<&Body>,
    creative: bool,
) -> Option<(BlockPos, BlockId)> {
    match action {
        WorldAction::Break { pos } => {
            let current = world.block_at(pos);
            let breakable = creative || info.content.blocks.get(current)?.hardness >= 0.0;
            (world.is_loaded(pos) && current != BlockId::AIR && breakable)
                .then_some((pos, BlockId::AIR))
        }
        WorldAction::Place {
            against,
            face,
            block,
        } => predicted_placement(info, world, against, face, block, player),
        WorldAction::Use { .. } | WorldAction::Interact(_) | WorldAction::Attack(_) => None,
    }
}

fn predicted_placement(
    info: &BlockInfo,
    world: &ClientWorld,
    against: BlockPos,
    face: Face,
    block: BlockId,
    player: Option<&Body>,
) -> Option<(BlockPos, BlockId)> {
    let blocks = &info.content.blocks;
    let aimed = world.block_at(against);
    let target = if blocks.get(aimed)?.replaceable {
        against
    } else {
        let target = against.neighbour(face);
        (info.is_solid(aimed) && !info.is_solid(world.block_at(target))).then_some(target)?
    };
    let placed = blocks.get(block)?;
    let cell_at = |pos: BlockPos| match world.is_loaded(pos) {
        true => Cell::Block(world.block_at(pos)),
        false => Cell::Unloaded,
    };
    let supported = info.updates.is_supported(
        block,
        cell_at(target.neighbour(Face::Down)),
        cell_at(target.neighbour(Face::Up)),
    );
    let in_player = info.is_solid(block) && player.is_some_and(|body| overlaps(body, target));
    let allowed = placed.has_item
        && block != BlockId::AIR
        && supported == Some(true)
        && world.is_loaded(target)
        && !in_player;
    let variant = match placed.orientation {
        Some(orientation) => orientation.along(face.axis()),
        None => block,
    };
    allowed.then_some((target, variant))
}

fn overlaps(body: &Body, pos: BlockPos) -> bool {
    let min = glam::DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
    body.aabb().overlaps(&Aabb {
        min,
        max: min + 1.0,
    })
}

#[cfg(test)]
mod tests;
