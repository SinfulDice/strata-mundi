//! Items on the ground: thrown or left by a broken block, they fall, join the equal
//! stacks around, are picked up when the player walks over them and disappear after the
//! lifetime of the pack's dropped-item entity.

use super::Game;
use super::terrain::WorldTerrain;
use crate::clock::TICKS_PER_SECOND;
use crate::ecs::{DroppedItem, Entity};
use crate::loader::ColumnLoader;
use glam::DVec3;
use glam::IVec3;
use std::collections::{HashMap, HashSet};

/// The cell of an item and the 26 around it.
const NEIGHBOURS: [IVec3; 27] = {
    let mut cells = [IVec3::ZERO; 27];
    let mut i = 0;
    while i < 27 {
        cells[i] = IVec3::new(i as i32 % 3 - 1, i as i32 / 3 % 3 - 1, i as i32 / 9 - 1);
        i += 1;
    }
    cells
};
use strata_core::content::ItemId;
use strata_core::coords::{BlockPos, WORLD_HEIGHT};
use strata_core::inventory::ItemStack;
use strata_core::physics::{ItemBody, step_item};
use strata_protocol::{EntityState, NetEntity, ServerMessage};

const THROW_SPEED: f64 = 0.3;
/// Half-width of a dropped item's box, in blocks.
pub(super) const DROP_SIZE: f64 = 0.25;

impl Game {
    /// Puts `stack` on the ground at `position`; nothing happens without a pack entity
    /// for dropped items.
    pub fn spawn_drop(&mut self, stack: ItemStack, position: DVec3, velocity: DVec3) {
        self.spawn_drop_aged(stack, ItemBody::at(position, velocity), 0, None);
    }

    /// A drop restored from a save, or thrown by the player (picked up after a delay).
    pub(super) fn spawn_drop_aged(
        &mut self,
        stack: ItemStack,
        body: ItemBody,
        age: u32,
        pickup_delay: Option<u32>,
    ) -> Option<Entity> {
        let (kind, def) = self.rules.dropped_item_entity()?;
        if stack.count == 0 || !body.position.is_finite() || !body.velocity.is_finite() {
            return None;
        }
        if self.ecs.drops.len() >= def.max_count as usize {
            self.remove_oldest_drop();
        }
        let entity = self.ecs.entities.spawn();
        let item = stack.item;
        self.ecs.drops.insert(
            entity,
            DroppedItem {
                stack,
                body,
                age,
                pickup_delay: pickup_delay.unwrap_or(0).min(def.pickup_delay),
                sent: body.position,
            },
        );
        self.messages
            .push(ServerMessage::EntitySpawned(EntityState {
                id: NetEntity(entity.to_bits()),
                kind,
                position: body.position.to_array(),
                yaw: 0.0,
                item: Some(item),
            }));
        Some(entity)
    }

    /// Throws stacks from the player's eyes, where they look.
    pub fn throw_all(&mut self, stacks: impl IntoIterator<Item = ItemStack>) {
        let Some(body) = self.player_body() else {
            return;
        };
        let [yaw, pitch] = self.look().map(f64::from);
        let direction = DVec3::new(
            -yaw.sin() * pitch.cos(),
            -pitch.sin(),
            -yaw.cos() * pitch.cos(),
        );
        let start = body.eyes() - DVec3::Y * 0.3;
        let delay = self
            .rules
            .dropped_item_entity()
            .map(|(_, def)| def.pickup_delay);
        for stack in stacks {
            let thrown = ItemBody::at(start, direction * THROW_SPEED);
            self.spawn_drop_aged(stack, thrown, 0, delay);
        }
    }

    /// One tick of every item on the ground: ageing, motion, joining, pickup.
    pub(super) fn drops_tick(&mut self, loader: &ColumnLoader) {
        let Some((_, def)) = self.rules.dropped_item_entity() else {
            return;
        };
        let lifetime = u64::from(def.lifetime) * u64::from(TICKS_PER_SECOND);
        let terrain = WorldTerrain {
            loader,
            rules: &self.rules,
            border: self.border,
        };
        let mut expired = Vec::new();
        let mut moved = Vec::new();
        for (entity, drop) in self.ecs.drops.iter_mut() {
            drop.age = drop.age.saturating_add(1);
            drop.pickup_delay = drop.pickup_delay.saturating_sub(1);
            if lifetime > 0 && u64::from(drop.age) >= lifetime {
                expired.push(entity);
                continue;
            }
            let loaded = column_loaded(&terrain, drop.body.position);
            if loaded && step_item(&mut drop.body, DROP_SIZE, &terrain) {
                moved.push(entity);
            }
        }
        for entity in expired {
            self.remove_drop(entity);
        }
        self.report_drop_moves(&moved);
        if self.tick.is_multiple_of(u64::from(def.merge_period)) {
            self.merge_drops(def.merge_radius);
        }
        self.pick_up(def.pickup_radius);
    }

    fn report_drop_moves(&mut self, moved: &[Entity]) {
        for &entity in moved {
            let Some(drop) = self.ecs.drops.get_mut(entity) else {
                continue;
            };
            if drop.sent.distance_squared(drop.body.position) < 1e-6 {
                continue;
            }
            drop.sent = drop.body.position;
            self.messages.push(ServerMessage::EntityMoved {
                id: NetEntity(entity.to_bits()),
                position: drop.body.position.to_array(),
                yaw: 0.0,
            });
        }
    }

    /// Equal stacks close to each other join, the older ones taking the younger. Items
    /// are sorted into a grid of `radius`-wide cells: only neighbouring cells are compared.
    fn merge_drops(&mut self, radius: f64) {
        let cell = radius.max(0.1);
        let key = |position: DVec3| (position / cell).floor().as_ivec3();
        let mut grid: HashMap<(ItemId, IVec3), Vec<Entity>> = HashMap::new();
        let mut oldest_first: Vec<(u32, Entity)> = Vec::new();
        for (entity, drop) in self.ecs.drops.iter() {
            let at = (drop.stack.item, key(drop.body.position));
            grid.entry(at).or_default().push(entity);
            oldest_first.push((drop.age, entity));
        }
        oldest_first.sort_by_key(|&(age, entity)| (std::cmp::Reverse(age), entity.to_bits()));
        let (mut emptied, mut removed) = (HashSet::new(), Vec::new());
        for (_, into) in oldest_first {
            let Some(drop) = self
                .ecs
                .drops
                .get(into)
                .filter(|_| !emptied.contains(&into))
            else {
                continue;
            };
            let (item, centre) = (drop.stack.item, key(drop.body.position));
            for offset in NEIGHBOURS {
                for &from in grid.get(&(item, centre + offset)).into_iter().flatten() {
                    if from != into
                        && !emptied.contains(&from)
                        && self.merge_pair(into, from, radius)
                    {
                        emptied.insert(from);
                        removed.push(from);
                    }
                }
            }
        }
        for entity in removed {
            self.remove_drop(entity);
        }
    }

    /// Moves what fits of `from` into `into` if they are close and equal; `true` when
    /// `from` is left empty.
    fn merge_pair(&mut self, into: Entity, from: Entity, radius: f64) -> bool {
        let (Some(a), Some(b)) = (self.ecs.drops.get(into), self.ecs.drops.get(from)) else {
            return false;
        };
        if a.body.position.distance(b.body.position) > radius || !a.stack.stacks_with(&b.stack) {
            return false;
        }
        let mut taken = b.stack.clone();
        if let Some(target) = self.ecs.drops.get_mut(into) {
            target.stack.absorb(&mut taken, &self.rules);
        }
        if let Some(rest) = self.ecs.drops.get_mut(from) {
            rest.stack.count = taken.count;
        }
        taken.count == 0
    }

    /// Items within reach of the player's body go into the inventory (what fits).
    fn pick_up(&mut self, radius: f64) {
        let Some(body) = self.player_body().copied() else {
            return;
        };
        let reach = body.aabb();
        let near: Vec<Entity> = self
            .ecs
            .drops
            .iter()
            .filter(|(_, drop)| drop.pickup_delay == 0)
            .filter(|(_, drop)| {
                let centre = drop.body.position + DVec3::Y * (DROP_SIZE / 2.0);
                centre.clamp(reach.min, reach.max).distance(centre) <= radius
            })
            .map(|(entity, _)| entity)
            .collect();
        for entity in near {
            let Some(stack) = self.ecs.drops.get(entity).map(|d| d.stack.clone()) else {
                continue;
            };
            match self.give(stack.clone()) {
                None => self.remove_drop(entity),
                Some(left) if left != stack => {
                    if let Some(drop) = self.ecs.drops.get_mut(entity) {
                        drop.stack = left;
                    }
                }
                Some(_) => {}
            }
        }
    }

    fn remove_oldest_drop(&mut self) {
        let oldest = self
            .ecs
            .drops
            .iter()
            .max_by_key(|(_, drop)| drop.age)
            .map(|(entity, _)| entity);
        if let Some(entity) = oldest {
            self.remove_drop(entity);
        }
    }

    pub(super) fn remove_drop(&mut self, entity: Entity) {
        self.ecs.despawn(entity);
        self.messages
            .push(ServerMessage::EntityRemoved(NetEntity(entity.to_bits())));
    }

    /// Items on the ground to save, with their age.
    pub fn saved_drops(&self) -> Vec<(ItemStack, ItemBody, u32)> {
        self.ecs
            .drops
            .iter()
            .map(|(_, drop)| (drop.stack.clone(), drop.body, drop.age))
            .collect()
    }

    pub fn restore_drops(&mut self, drops: Vec<(ItemStack, ItemBody, u32)>) {
        for (stack, body, age) in drops {
            if stack.is_valid(&self.rules) {
                self.spawn_drop_aged(stack, body, age, None);
            }
        }
    }
}

/// Items in unloaded columns wait (they would fall through the missing ground).
fn column_loaded(terrain: &WorldTerrain<'_>, position: DVec3) -> bool {
    let y = (position.y.floor() as i32).clamp(0, WORLD_HEIGHT - 1);
    let pos = BlockPos::new(position.x.floor() as i32, y, position.z.floor() as i32);
    terrain.loader.block(pos).is_some()
}
