//! Placing blocks: where a block goes, which way it turns, and who stands in the way.

use super::Game;
use crate::loader::{Bounded, ColumnLoader};
use glam::DVec3;
use strata_core::content::{BlockId, Face, Facing};
use strata_core::coords::BlockPos;
use strata_core::physics::Aabb;
use strata_core::updates::{BlockChange, cell};

impl Game {
    /// Validated placement into a free (non-solid) cell that no player stands in: next to
    /// a solid block, or in place of a replaceable one (plants). Turned blocks (logs)
    /// follow the clicked face, attached ones (torches) lean on the clicked block; plants
    /// and torches need their support.
    pub fn place_block(
        &mut self,
        loader: &mut ColumnLoader,
        against: BlockPos,
        face: Face,
        block: BlockId,
    ) -> Vec<BlockChange> {
        let Some(target) = self.placement_target(loader, against, face) else {
            return Vec::new();
        };
        let world = Bounded {
            loader,
            border: self.border,
        };
        let variant = self
            .rules
            .updates()
            .placed_variant(block, face, |side| cell(&world, target.neighbour(side)))
            .map(|variant| self.facing_player(variant));
        let bag = self.holds_bag_of(block);
        let held = self
            .held_item()
            .is_some_and(|item| self.rules.block_item(block) == Some(item));
        let allowed = ((self.rules.is_placeable(block) && held) || bag)
            && self.in_reach(target)
            && !self.blocks_a_player(target, block)
            && variant.is_some()
            && self.take_action();
        let mut changes = Vec::new();
        if let (true, Some(variant)) = (allowed, variant) {
            self.set_block(loader, target, variant, &mut changes);
            if bag {
                self.put_down_bag(target);
            } else {
                self.consume_held();
            }
        }
        changes
    }

    /// The cell a block placed against `against` goes to, if any.
    pub(super) fn placement_target(
        &self,
        loader: &ColumnLoader,
        against: BlockPos,
        face: Face,
    ) -> Option<BlockPos> {
        if !against.is_valid(self.border) {
            return None;
        }
        let aimed = loader.block(against)?;
        if self.rules.is_replaceable(aimed) {
            return Some(against);
        }
        let target = against.neighbour(face);
        let current = loader.block(target)?;
        (self.rules.is_solid(aimed)
            && !self.rules.is_solid(current)
            && target.is_valid(self.border))
        .then_some(target)
    }

    /// A block with a front (furnace) turns it toward the player placing it.
    fn facing_player(&self, block: BlockId) -> BlockId {
        let Some(facing) = self.rules.facing(block) else {
            return block;
        };
        facing.variant(Facing::front_toward_viewer(self.look()[0]), false)
    }

    fn blocks_a_player(&self, pos: BlockPos, block: BlockId) -> bool {
        if !self.rules.is_solid(block) {
            return false;
        }
        let min = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z));
        let cell = Aabb {
            min,
            max: min + 1.0,
        };
        self.ecs
            .bodies
            .iter()
            .any(|(_, body)| body.aabb().overlaps(&cell))
    }
}
