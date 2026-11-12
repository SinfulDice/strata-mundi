//! Block clicks: run at once when the rate limit allows, otherwise queued and run one per
//! allowed tick, so no click is lost. Every click is answered with `ActionDone`; a
//! refused one first gets the actual state of the cells it aimed at, so the client can
//! undo its prediction.

use super::{ACTION_TICKS, Game};
use crate::loader::ColumnLoader;
use strata_core::content::Face;
use strata_core::coords::BlockPos;
use strata_core::updates::BlockChange;
use strata_protocol::{ActionId, BlockAction, ServerMessage};

/// Clicks waiting for their turn; more are refused (a client spamming clicks).
pub const MAX_QUEUED_ACTIONS: usize = 8;

impl Game {
    /// Runs the click now if nothing waits and the rate limit allows, queues it otherwise.
    /// The answer (`ActionDone`) goes with the game messages.
    pub fn request_action(
        &mut self,
        loader: &mut ColumnLoader,
        id: ActionId,
        action: BlockAction,
    ) -> Vec<BlockChange> {
        let ready = self.action_ready();
        let Some(control) = self.ecs.controls.get_mut(self.player) else {
            return Vec::new();
        };
        if ready && control.actions.is_empty() {
            return self.run_action(loader, id, action);
        }
        if control.actions.len() < MAX_QUEUED_ACTIONS {
            control.actions.push_back((id, action));
        } else {
            self.answer(loader, id, action, &[]);
        }
        Vec::new()
    }

    /// The oldest waiting click, if its turn has come.
    pub(super) fn run_queued_action(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        if !self.action_ready() {
            return Vec::new();
        }
        let next = self
            .ecs
            .controls
            .get_mut(self.player)
            .and_then(|control| control.actions.pop_front());
        match next {
            Some((id, action)) => self.run_action(loader, id, action),
            None => Vec::new(),
        }
    }

    fn run_action(
        &mut self,
        loader: &mut ColumnLoader,
        id: ActionId,
        action: BlockAction,
    ) -> Vec<BlockChange> {
        let changes = match action {
            BlockAction::Break { pos } => self.break_block(loader, pos),
            BlockAction::Place {
                against,
                face,
                block,
            } => self.place_block(loader, against, face, block),
            BlockAction::Use {
                item,
                against,
                face,
            } => self.use_item(loader, item, against, face),
        };
        self.answer(loader, id, action, &changes);
        changes
    }

    /// Whether the rate limit lets the player act this tick.
    fn action_ready(&self) -> bool {
        let tick = self.tick;
        self.ecs
            .controls
            .get(self.player)
            .is_some_and(|c| c.last_action.is_none_or(|last| tick >= last + ACTION_TICKS))
    }

    /// A refused action (nothing changed) sends the actual state of the aimed cells, which
    /// the client may have predicted otherwise; then the action is acknowledged.
    fn answer(
        &mut self,
        loader: &ColumnLoader,
        id: ActionId,
        action: BlockAction,
        changes: &[BlockChange],
    ) {
        if changes.is_empty() {
            for pos in aimed_cells(action) {
                if let Some(block) = loader.block(pos) {
                    self.messages
                        .push(ServerMessage::BlockChanged { pos, block });
                }
            }
        }
        self.messages.push(ServerMessage::ActionDone(id));
    }
}

/// Every cell the action may have changed, as the client could predict it.
fn aimed_cells(action: BlockAction) -> Vec<BlockPos> {
    let (against, face): (BlockPos, Option<Face>) = match action {
        BlockAction::Break { pos } => (pos, None),
        BlockAction::Place { against, face, .. } | BlockAction::Use { against, face, .. } => {
            (against, Some(face))
        }
    };
    let mut cells = vec![against];
    cells.extend(face.map(|face| against.neighbour(face)));
    cells
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
