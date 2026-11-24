//! Kits: used on a furnace, a chest or a barrel, they make it the next tier in place,
//! contents, fire and upgrades kept. Both halves of a double chest go up together, one
//! kit each, so the pair keeps one tier; a kit that would part a pair is refused.

use super::Game;
use super::containers::block_centre;
use crate::loader::ColumnLoader;
use glam::DVec3;
use strata_core::content::BlockId;
use strata_core::cooking::{FurnaceState, KitTarget};
use strata_core::coords::BlockPos;
use strata_core::updates::BlockChange;
use strata_protocol::ServerMessage;
use strata_protocol::commands::Feedback;

impl Game {
    /// Right click on a block: a kit in hand upgrades the block it is used on;
    /// otherwise the block opens (container, station, furnace).
    pub(super) fn use_block(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
    ) -> Vec<BlockChange> {
        if let Some(changes) = self.upgrade_with_kit(loader, pos) {
            return changes;
        }
        self.open_block(loader, pos);
        Vec::new()
    }

    /// `None` when the hand holds no kit or the block is not of the kit's kind.
    fn upgrade_with_kit(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
    ) -> Option<Vec<BlockChange>> {
        let item = self.held_item()?;
        let kit = self.rules.items().get(item)?.kit.clone()?;
        let block = loader.block(pos)?;
        let fits_kind = match kit.on {
            KitTarget::Furnace => self.rules.furnace(block).is_some(),
            KitTarget::Storage => self.rules.container(block)?.upgradable.is_some(),
        };
        if !fits_kind {
            return None;
        }
        let mut changes = Vec::new();
        if !self.in_reach(pos) || !self.take_action() {
            return Some(changes);
        }
        let halves = match self.partner(loader, pos) {
            Some(other) => vec![pos, other],
            None => vec![pos],
        };
        let Some(upgraded) = self.rules.upgraded_by(block, &kit) else {
            self.kit_feedback(kit.on, KitNews::WrongTier);
            return Some(changes);
        };
        if !self.holds_kits(halves.len()) {
            self.kit_feedback(kit.on, KitNews::NeedsTwo);
            return Some(changes);
        }
        if self.would_part_a_pair(loader, &halves, upgraded) {
            self.kit_feedback(kit.on, KitNews::PartsAPair);
            return Some(changes);
        }
        for &half in &halves {
            self.raise_tier(loader, half, block, upgraded, &mut changes);
        }
        self.kit_feedback(kit.on, KitNews::Done);
        Some(changes)
    }

    /// Survival: the hand holds a kit for each block (creative needs one).
    fn holds_kits(&self, needed: usize) -> bool {
        let held = self
            .ecs
            .inventories
            .get(self.player)
            .and_then(|inventory| inventory.selected_stack())
            .map_or(0, |stack| usize::from(stack.count));
        self.is_creative() || held >= needed
    }

    /// The block at `pos` becomes `upgraded`: its entity stays, its slots grow (what a
    /// pack's smaller tier cuts off falls), and one kit is used.
    fn raise_tier(
        &mut self,
        loader: &mut ColumnLoader,
        pos: BlockPos,
        block: BlockId,
        upgraded: BlockId,
        changes: &mut Vec<BlockChange>,
    ) {
        let Some(layout) = self.rules.layout(upgraded).cloned() else {
            return;
        };
        let furnace = self.rules.furnace(block).is_some();
        let entity = self.block_entities.entry(pos).or_default();
        if furnace {
            entity.furnace.get_or_insert_with(FurnaceState::default);
        }
        let mut cut = entity.container.resize(layout.storage);
        cut.extend(entity.upgrades.resize(layout.upgrades));
        for stack in cut {
            self.spawn_drop(stack, block_centre(pos), DVec3::ZERO);
        }
        self.follow_upgraded_window(pos, upgraded);
        self.set_block(loader, pos, upgraded, changes);
        self.consume_held();
        if furnace {
            self.wake_furnace(pos);
        }
    }

    /// The open screen of an upgraded block shows its new tier and slots.
    fn follow_upgraded_window(&mut self, pos: BlockPos, block: BlockId) {
        let Some(window) = self.ecs.windows.get_mut(self.player) else {
            return;
        };
        if !window.halves.contains(&pos) {
            return;
        }
        window.block = block;
        let window = window.clone();
        let upgrades_at = self.upgrades_at(block, window.halves.len());
        let contents = self.window_contents(&window);
        self.messages
            .push(ServerMessage::ContainerUpgraded { block, upgrades_at });
        self.messages
            .push(ServerMessage::ContainerContents(Box::new(contents)));
    }

    fn kit_feedback(&mut self, on: KitTarget, news: KitNews) {
        let key = match (on, news) {
            (KitTarget::Furnace, KitNews::Done) => "feedback-kit-furnace-done",
            (KitTarget::Furnace, _) => "feedback-kit-furnace-wrong-tier",
            (KitTarget::Storage, KitNews::WrongTier) => "feedback-kit-storage-wrong-tier",
            (KitTarget::Storage, KitNews::NeedsTwo) => "feedback-kit-storage-needs-two",
            (KitTarget::Storage, KitNews::PartsAPair) => "feedback-kit-storage-parts-a-pair",
            (KitTarget::Storage, KitNews::Done) => "feedback-kit-storage-done",
        };
        let feedback = match news {
            KitNews::Done => Feedback::ok(key, &[]),
            _ => Feedback::error(key, &[]),
        };
        self.messages.push(ServerMessage::Feedback(feedback));
    }
}

/// What a kit's player is told.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum KitNews {
    WrongTier,
    /// A double chest needs a kit for each half.
    NeedsTwo,
    /// A neighbour of the new tier would take a half of a double chest.
    PartsAPair,
    Done,
}
