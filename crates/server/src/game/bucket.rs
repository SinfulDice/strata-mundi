//! Items used on the world: buckets pour and take liquid sources, boats are put down.

use super::Game;
use crate::loader::ColumnLoader;
use strata_core::content::{BlockId, Face, ItemAction, ItemId};
use strata_core::coords::BlockPos;
use strata_core::updates::BlockChange;

impl Game {
    /// Validated use of `item`, held in the hotbar, on `face` of the block at `against`.
    /// Creative: buckets stay as they are.
    pub fn use_item(
        &mut self,
        loader: &mut ColumnLoader,
        item: ItemId,
        against: BlockPos,
        face: Face,
    ) -> Vec<BlockChange> {
        let held = self.hotbar().slots.contains(&Some(item));
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
            ItemAction::PlaceEntity(kind) => {
                self.place_entity(loader, kind, against, face);
                return Vec::new();
            }
        };
        let mut changes = Vec::new();
        if self.in_reach(target) && self.take_action() {
            self.set_block(loader, target, block, &mut changes);
        }
        changes
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::super::tests::{World, block, wait, world};
    use super::*;
    use strata_protocol::HotbarState;

    fn item(w: &World, id: &str) -> ItemId {
        w.content.items.key(&id.parse().unwrap()).unwrap()
    }

    fn hold(w: &mut World, items: &[&str]) {
        let slots = items.iter().map(|id| Some(item(w, id))).collect();
        assert!(w.game.set_hotbar(HotbarState { slots, selected: 0 }));
    }

    #[test]
    fn a_bucket_pours_a_source_that_flows_then_takes_it_back() {
        let mut w = world();
        hold(&mut w, &["strata:water_bucket", "strata:bucket"]);
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
    fn buckets_must_be_held_and_take_only_sources() {
        let mut w = world();
        let empty = item(&w, "strata:bucket");
        let stone = BlockPos::new(11, 63, 8);
        assert!(
            w.game
                .use_item(&mut w.loader, empty, stone, Face::Up)
                .is_empty(),
            "not in the hotbar"
        );
        hold(&mut w, &["strata:bucket", "strata:lava_bucket"]);
        assert!(
            w.game
                .use_item(&mut w.loader, empty, stone, Face::Up)
                .is_empty(),
            "stone is no liquid"
        );
        let lava = item(&w, "strata:lava_bucket");
        let poured = w.game.use_item(&mut w.loader, lava, stone, Face::Up);
        assert_eq!(poured.len(), 1);
        wait(&mut w, 2);
        let flowing = BlockPos::new(12, 64, 8);
        assert!(
            w.game
                .use_item(&mut w.loader, empty, flowing, Face::Up)
                .is_empty(),
            "no source here"
        );
    }
}
