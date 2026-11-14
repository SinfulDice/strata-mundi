//! Items used on the world: buckets pour and take liquid sources, flint and steel lights
//! fires, boats are put down.

use super::Game;
use crate::loader::{Bounded, ColumnLoader};
use strata_core::content::{BlockId, Face, ItemAction, ItemId};
use strata_core::coords::BlockPos;
use strata_core::updates::BlockChange;

impl Game {
    /// Validated use of `item`, in the selected hotbar slot, on `face` of the block at
    /// `against`. In survival a bucket is filled or emptied, flint and steel wears and a
    /// boat is used up.
    pub fn use_item(
        &mut self,
        loader: &mut ColumnLoader,
        item: ItemId,
        against: BlockPos,
        face: Face,
    ) -> Vec<BlockChange> {
        let held = self.held_item() == Some(item);
        let (Some(action), true) = (self.rules.item_action(item), held) else {
            return Vec::new();
        };
        let (target, block) = match action {
            ItemAction::PlaceFluid(source) => {
                let Some(target) = self.placement_target(loader, against, face) else {
                    return Vec::new();
                };
                if loader.block(target) == Some(source) {
                    return Vec::new();
                }
                (target, source)
            }
            ItemAction::TakeFluid => {
                let is_source = loader
                    .block(against)
                    .is_some_and(|block| self.rules.is_fluid_source(block));
                if !is_source || !against.is_valid(self.border) {
                    return Vec::new();
                }
                (against, BlockId::AIR)
            }
            ItemAction::Ignite(fire) => {
                let Some(target) = self.fire_target(loader, against, face) else {
                    return Vec::new();
                };
                (target, fire)
            }
            ItemAction::PlaceEntity(kind) => {
                if self.place_entity(loader, kind, against, face).is_some() {
                    self.consume_held();
                }
                return Vec::new();
            }
        };
        let mut changes = Vec::new();
        if self.in_reach(target) && self.take_action() {
            let replaced = loader.block(target);
            self.set_block(loader, target, block, &mut changes);
            self.after_use(action, replaced);
        }
        changes
    }

    /// Survival: what using the item did to it.
    fn after_use(&mut self, action: ItemAction, replaced: Option<BlockId>) {
        let exchanged = match action {
            ItemAction::PlaceFluid(_) => self.rules.item_with(ItemAction::TakeFluid),
            ItemAction::TakeFluid => {
                replaced.and_then(|source| self.rules.item_with(ItemAction::PlaceFluid(source)))
            }
            ItemAction::Ignite(_) => {
                self.wear_held(1);
                None
            }
            ItemAction::PlaceEntity(_) => None,
        };
        if let Some(item) = exchanged {
            self.exchange_held(item);
        }
    }

    /// Where flint and steel used on `face` of `against` lights a fire: in the air (or a
    /// plant) in front, if a fire may burn there.
    fn fire_target(
        &self,
        loader: &mut ColumnLoader,
        against: BlockPos,
        face: Face,
    ) -> Option<BlockPos> {
        let target = self.placement_target(loader, against, face)?;
        let current = loader.block(target)?;
        let free = current == BlockId::AIR
            || (self.rules.is_replaceable(current) && self.rules.updates().fire(current).is_none());
        let world = Bounded {
            loader,
            border: self.border,
        };
        (free && self.updates.fire_can_stay(&world, target) == Some(true)).then_some(target)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::tests::{World, block, wait, world};
    use super::*;
    use strata_core::inventory::ItemStack;

    fn item(w: &World, id: &str) -> ItemId {
        w.content.items.key(&id.parse().unwrap()).unwrap()
    }

    fn hold(w: &mut World, items: &[&str], selected: u8) {
        let stacks: Vec<_> = items
            .iter()
            .map(|id| Some(ItemStack::new(item(w, id), 1, &w.game.rules)))
            .collect();
        w.game.with_inventory(|inventory| {
            inventory.slots.slots.fill(None);
            inventory.slots.slots[..stacks.len()].clone_from_slice(&stacks);
            inventory.selected = selected;
        });
    }

    #[test]
    fn a_bucket_pours_a_source_that_flows_then_takes_it_back() {
        let mut w = world();
        hold(&mut w, &["strata:water_bucket", "strata:bucket"], 0);
        let (water, empty) = (item(&w, "strata:water_bucket"), item(&w, "strata:bucket"));
        let trench: Vec<BlockPos> = (9..=13).map(|x| BlockPos::new(x, 63, 8)).collect();
        for &pos in &trench {
            assert!(w.loader.set_block(pos, BlockId::AIR));
        }
        let source = trench[0];
        let poured = w
            .game
            .use_item(&mut w.loader, water, source.neighbour(Face::Down), Face::Up);
        assert_eq!(poured, vec![(source, block(&w.content, "strata:water"))]);
        wait(&mut w, 40);
        hold(&mut w, &["strata:water_bucket", "strata:bucket"], 1);
        let flowing = block(&w.content, "strata:water__1");
        assert_eq!(w.loader.block(trench[1]), Some(flowing));
        assert_ne!(w.loader.block(trench[4]), Some(BlockId::AIR));
        let taken = w.game.use_item(&mut w.loader, empty, source, Face::Up);
        assert_eq!(taken, vec![(source, BlockId::AIR)]);
        wait(&mut w, 200);
        for pos in trench {
            assert_eq!(
                w.loader.block(pos),
                Some(BlockId::AIR),
                "the flow dries up without its source"
            );
        }
    }

    #[test]
    fn flint_and_steel_lights_a_fire_that_water_puts_out() {
        let mut w = world();
        hold(&mut w, &["strata:flint_and_steel"], 0);
        let flint = item(&w, "strata:flint_and_steel");
        let fire = block(&w.content, "strata:fire");
        let ground = BlockPos::new(11, 63, 8);
        let lit = w.game.use_item(&mut w.loader, flint, ground, Face::Up);
        assert_eq!(lit, vec![(BlockPos::new(11, 64, 8), fire)]);
        wait(&mut w, 3);
        assert!(
            w.game
                .use_item(&mut w.loader, flint, ground, Face::Up)
                .is_empty(),
            "already burning"
        );
        let water = block(&w.content, "strata:water");
        assert!(w.loader.set_block(BlockPos::new(12, 64, 8), water));
        let wet = BlockPos::new(13, 63, 8);
        assert!(
            w.game
                .use_item(&mut w.loader, flint, wet, Face::Up)
                .is_empty(),
            "no fire next to water"
        );
        wait(&mut w, 100);
        assert_ne!(
            w.loader.block(BlockPos::new(11, 64, 8)),
            Some(fire),
            "put out by the water next to it"
        );
    }

    #[test]
    fn buckets_must_be_selected_and_take_only_sources() {
        let mut w = world();
        let (empty, lava) = (item(&w, "strata:bucket"), item(&w, "strata:lava_bucket"));
        let stone = BlockPos::new(11, 63, 8);
        assert!(
            w.game
                .use_item(&mut w.loader, empty, stone, Face::Up)
                .is_empty(),
            "not in the hotbar"
        );
        hold(&mut w, &["strata:bucket", "strata:lava_bucket"], 0);
        assert!(
            w.game
                .use_item(&mut w.loader, lava, stone, Face::Up)
                .is_empty(),
            "in the hotbar but not in the selected slot"
        );
        assert!(
            w.game
                .use_item(&mut w.loader, empty, stone, Face::Up)
                .is_empty(),
            "stone is no liquid"
        );
        hold(&mut w, &["strata:bucket", "strata:lava_bucket"], 1);
        let poured = w.game.use_item(&mut w.loader, lava, stone, Face::Up);
        assert_eq!(poured.len(), 1);
        wait(&mut w, 2);
        hold(&mut w, &["strata:bucket", "strata:lava_bucket"], 0);
        let flowing = BlockPos::new(12, 64, 8);
        assert!(
            w.game
                .use_item(&mut w.loader, empty, flowing, Face::Up)
                .is_empty(),
            "no source here"
        );
    }
}
