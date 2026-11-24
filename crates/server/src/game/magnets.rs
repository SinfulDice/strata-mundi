//! Storage magnets: only chests and barrels with a magnet upgrade are listed; each works
//! at its period, spread over the ticks by its position, and looks only at the items on
//! the ground in the cells around it.

use super::Game;
use super::containers::block_centre;
use super::drops::DROP_SIZE;
use crate::ecs::Entity;
use crate::loader::ColumnLoader;
use glam::DVec3;
use std::collections::{BTreeMap, HashMap};
use strata_core::coords::BlockPos;
use strata_core::inventory::ItemStack;
use strata_core::storage::take_in;
use strata_protocol::ServerMessage;

/// Side, in blocks, of the cells items on the ground are sorted into for magnets.
const CELL: f64 = 8.0;

/// Chests and barrels with a magnet, and whether magnets work at all (world option).
#[derive(Debug, Clone, Default)]
pub struct Magnets {
    /// Radius and period of each storage with a magnet upgrade.
    pub at: BTreeMap<BlockPos, (f32, u16)>,
    pub disabled: bool,
}

/// Items on the ground that may be drawn, by cell.
type DropCells = HashMap<(i64, i64), Vec<Entity>>;

fn cell_of(x: f64, z: f64) -> (i64, i64) {
    ((x / CELL).floor() as i64, (z / CELL).floor() as i64)
}

/// Spreads the magnets of one period over its ticks.
fn offset(pos: BlockPos) -> u64 {
    let bytes = [
        pos.x.to_le_bytes(),
        pos.y.to_le_bytes(),
        pos.z.to_le_bytes(),
    ]
    .concat();
    strata_core::hash::fnv1a(&bytes)
}

impl Game {
    /// World option: storage magnets draw items lying around.
    pub fn set_storage_magnets(&mut self, enabled: bool) {
        self.magnets.disabled = !enabled;
    }

    /// The upgrade slots of the storage at `pos` may have changed: its magnet with them.
    pub(super) fn refresh_magnet(&mut self, pos: BlockPos) {
        match self.upkeep_of(&[pos]).magnet {
            Some(magnet) => self.magnets.at.insert(pos, magnet),
            None => self.magnets.at.remove(&pos),
        };
    }

    #[cfg(test)]
    pub(crate) fn has_magnet(&self, pos: BlockPos) -> bool {
        self.magnets.at.contains_key(&pos)
    }

    /// After loading: the storages with a magnet.
    pub(super) fn find_magnets(&mut self) {
        self.magnets.at.clear();
        let positions: Vec<BlockPos> = self.block_entities.keys().copied().collect();
        for pos in positions {
            self.refresh_magnet(pos);
        }
    }

    /// Each magnet due this tick draws the items lying within its radius (those the
    /// player could pick up) into its storage, through its filters.
    pub(super) fn magnets_tick(&mut self, loader: &ColumnLoader) {
        if self.magnets.disabled || self.magnets.at.is_empty() || self.ecs.drops.len() == 0 {
            return;
        }
        let tick = self.tick;
        let due: Vec<(BlockPos, f32)> = (self.magnets.at.iter())
            .filter(|(pos, (_, period))| {
                tick.wrapping_add(offset(**pos))
                    .is_multiple_of(u64::from((*period).max(1)))
            })
            .map(|(&pos, &(radius, _))| (pos, radius))
            .collect();
        if due.is_empty() {
            return;
        }
        let cells = self.drop_cells();
        for (pos, radius) in due {
            let present = loader
                .block(pos)
                .is_some_and(|block| self.rules.container(block).is_some());
            if present {
                let halves = self.halves_at(loader, pos);
                if self.draw_items(&halves, pos, f64::from(radius), &cells) {
                    self.storage_changed_on_its_own(&halves);
                }
            }
        }
    }

    pub(super) fn drop_cells(&self) -> DropCells {
        let mut cells: DropCells = HashMap::new();
        for (entity, drop) in self.ecs.drops.iter() {
            if drop.pickup_delay == 0 {
                let at = drop.body.position;
                cells.entry(cell_of(at.x, at.z)).or_default().push(entity);
            }
        }
        cells
    }

    /// The items within `radius` of `centre`, from the cells it touches.
    pub(super) fn near_drops(
        &self,
        centre: DVec3,
        radius: f64,
        cells: &DropCells,
    ) -> Vec<(Entity, ItemStack)> {
        let (min, max) = (
            cell_of(centre.x - radius, centre.z - radius),
            cell_of(centre.x + radius, centre.z + radius),
        );
        let keys = (min.0..=max.0).flat_map(|x| (min.1..=max.1).map(move |z| (x, z)));
        keys.filter_map(|key| cells.get(&key))
            .flatten()
            .filter_map(|&entity| {
                let drop = self.ecs.drops.get(entity)?;
                let middle = drop.body.position + DVec3::Y * (DROP_SIZE / 2.0);
                (middle.distance(centre) <= radius).then(|| (entity, drop.stack.clone()))
            })
            .collect()
    }

    fn draw_items(
        &mut self,
        halves: &[BlockPos],
        pos: BlockPos,
        radius: f64,
        cells: &DropCells,
    ) -> bool {
        let mut near = self.near_drops(block_centre(pos), radius, cells);
        if self.is_put_down_bag(pos) {
            near.retain(|(_, stack)| !self.rules.bags.is_bag(stack.item));
        }
        if near.is_empty() {
            return false;
        }
        let upkeep = self.upkeep_of(halves);
        let results: Vec<(Entity, ItemStack, Option<ItemStack>)> =
            self.with_storage(halves, |storage, game| {
                (near.into_iter())
                    .map(|(entity, stack)| {
                        let left = take_in(storage, stack.clone(), &upkeep, game.rules.items());
                        (entity, stack, left)
                    })
                    .collect()
            });
        let mut drew = false;
        for (entity, stack, left) in results {
            match left {
                None => self.remove_drop(entity),
                Some(left) if left != stack => {
                    if let Some(drop) = self.ecs.drops.get_mut(entity) {
                        drop.stack = left;
                    }
                }
                Some(_) => continue,
            }
            drew = true;
        }
        drew
    }

    /// The storage of these halves took items in on its own: compression, sorting unless
    /// its screen is open, and the screen follows.
    fn storage_changed_on_its_own(&mut self, halves: &[BlockPos]) {
        let window = self.ecs.windows.get(self.player).cloned();
        let open = window
            .as_ref()
            .is_some_and(|w| w.halves.iter().any(|h| halves.contains(h)));
        let now = self.storage_of(halves);
        self.storage_entered(halves, &now, !open);
        if let Some(window) = window.filter(|_| open) {
            let contents = self.window_contents(&window);
            self.messages
                .push(ServerMessage::ContainerContents(Box::new(contents)));
        }
    }
}
