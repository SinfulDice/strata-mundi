//! Clicks in the world: run at once when the rate limit allows, otherwise queued and run
//! one per allowed tick, so no click is lost. Every click is answered with `ActionDone`,
//! in the order of the clicks; a refused one first gets the actual state of the cells it
//! aimed at (those the player sees), so the client can undo its prediction.

use super::{ACTION_TICKS, Game};
use crate::ecs::Queued;
use crate::loader::ColumnLoader;
use strata_core::content::Face;
use strata_core::coords::BlockPos;
use strata_core::updates::BlockChange;
use strata_protocol::{ActionId, MAX_PENDING_ACTIONS, ServerMessage, WorldAction, in_view_disc};

/// Refused clicks wait too (to keep the answers in order), up to this many entries.
const MAX_QUEUE: usize = 2 * MAX_PENDING_ACTIONS;

impl Game {
    /// Runs the click now if nothing waits and the rate limit allows, queues it otherwise.
    /// The answer (`ActionDone`) goes with the game messages.
    pub fn request_action(
        &mut self,
        loader: &mut ColumnLoader,
        id: ActionId,
        action: WorldAction,
    ) -> Vec<BlockChange> {
        let ready = self.action_ready();
        let Some(control) = self.ecs.controls.get_mut(self.player) else {
            return Vec::new();
        };
        if ready && control.actions.is_empty() {
            return self.run_action(loader, id, action);
        }
        let queue = &mut control.actions;
        let waiting = queue
            .iter()
            .filter(|q| matches!(q, Queued::Run(..)))
            .count();
        if waiting < MAX_PENDING_ACTIONS {
            queue.push_back(Queued::Run(id, action));
        } else if queue.len() < MAX_QUEUE {
            queue.push_back(Queued::Refuse(id, action));
        } else {
            // Only a client ignoring `MAX_PENDING_ACTIONS` gets here: answered out of turn.
            self.answer(loader, id, action, &[]);
        }
        Vec::new()
    }

    /// Answers the waiting clicks in order: refusals at once (they take no turn), runs
    /// while the rate limit allows.
    pub(super) fn run_queued_actions(&mut self, loader: &mut ColumnLoader) -> Vec<BlockChange> {
        let mut changes = Vec::new();
        loop {
            let ready = self.action_ready();
            let Some(queue) = self
                .ecs
                .controls
                .get_mut(self.player)
                .map(|c| &mut c.actions)
            else {
                break;
            };
            match queue.front().copied() {
                Some(Queued::Refuse(id, action)) => {
                    queue.pop_front();
                    self.answer(loader, id, action, &[]);
                }
                Some(Queued::Run(id, action)) if ready => {
                    queue.pop_front();
                    changes.extend(self.run_action(loader, id, action));
                }
                _ => break,
            }
        }
        changes
    }

    fn run_action(
        &mut self,
        loader: &mut ColumnLoader,
        id: ActionId,
        action: WorldAction,
    ) -> Vec<BlockChange> {
        let changes = match action {
            WorldAction::Break { pos } => self.break_block(loader, pos),
            WorldAction::Place {
                against,
                face,
                block,
            } => self.place_block(loader, against, face, block),
            WorldAction::Use {
                item,
                against,
                face,
            } => self.use_item(loader, item, against, face),
            WorldAction::Interact(entity) => {
                self.interact_entity(entity);
                Vec::new()
            }
            WorldAction::Attack(entity) => {
                self.attack_entity(loader, entity);
                Vec::new()
            }
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

    /// A refused action (nothing changed) sends the actual state of the aimed cells the
    /// player sees, which the client may have predicted otherwise; then the action is
    /// acknowledged.
    fn answer(
        &mut self,
        loader: &ColumnLoader,
        id: ActionId,
        action: WorldAction,
        changes: &[BlockChange],
    ) {
        if changes.is_empty() {
            for pos in aimed_cells(action) {
                if let (true, Some(block)) = (self.sees(pos), loader.block(pos)) {
                    self.messages
                        .push(ServerMessage::BlockChanged { pos, block });
                }
            }
        }
        self.messages.push(ServerMessage::ActionDone(id));
    }

    /// Whether the column of `pos` lies in the player's view disc (what their client has).
    fn sees(&self, pos: BlockPos) -> bool {
        self.player_body().is_some_and(|body| {
            let column = pos.column();
            let (x, z) = (
                body.position.x.floor() as i32,
                body.position.z.floor() as i32,
            );
            let player = BlockPos::new(x, 0, z).column();
            in_view_disc(column.x - player.x, column.z - player.z, self.view_distance)
        })
    }
}

/// Every cell the action may have changed, as the client could predict it.
fn aimed_cells(action: WorldAction) -> Vec<BlockPos> {
    let (against, face): (BlockPos, Option<Face>) = match action {
        WorldAction::Break { pos } => (pos, None),
        WorldAction::Place { against, face, .. } | WorldAction::Use { against, face, .. } => {
            (against, Some(face))
        }
        WorldAction::Interact(_) | WorldAction::Attack(_) => return Vec::new(),
    };
    let mut cells = vec![against];
    cells.extend(face.map(|face| against.neighbour(face)));
    cells
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests;
