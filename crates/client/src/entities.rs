//! Entities other than the local player, as the server last described them, drawn
//! between their last two known poses so they glide at any frame rate.

use crate::models::{ModelKind, ModelPose};
use glam::DVec3;
use std::collections::HashMap;
use strata_core::content::{EntityTypeId, ItemId};
use strata_core::physics::TICK;
use strata_protocol::{EntityState, NetEntity};

/// Turns per second of an item on the ground, and how high it bobs (blocks).
const ITEM_SPIN: f64 = 0.4;
const ITEM_BOB: f64 = 0.06;

#[derive(Debug, Clone)]
struct ClientEntity {
    kind: EntityTypeId,
    /// What an item on the ground shows.
    item: Option<ItemId>,
    /// Seconds since it appeared (items turn and bob).
    age: f64,
    previous: ModelPose,
    current: ModelPose,
    /// Seconds since `current` arrived.
    elapsed: f64,
}

impl ClientEntity {
    fn pose(&self) -> ModelPose {
        let alpha = (self.elapsed / TICK).clamp(0.0, 1.0);
        let turn = shortest_turn(self.previous.yaw, self.current.yaw);
        let position = self.previous.position.lerp(self.current.position, alpha);
        if self.item.is_some() {
            let phase = self.age * ITEM_SPIN * std::f64::consts::TAU;
            let bob = (phase.sin() + 1.0) * ITEM_BOB;
            return ModelPose {
                position: position + DVec3::Y * bob,
                yaw: phase as f32,
            };
        }
        ModelPose {
            position,
            yaw: self.previous.yaw + turn * alpha as f32,
        }
    }

    fn model(&self) -> ModelKind {
        self.item
            .map_or(ModelKind::Entity(self.kind), ModelKind::Item)
    }
}

/// From `from` to `to` the short way round, in radians.
fn shortest_turn(from: f32, to: f32) -> f32 {
    use std::f32::consts::{PI, TAU};
    (to - from + PI).rem_euclid(TAU) - PI
}

#[derive(Debug, Clone, Default)]
pub struct ClientEntities {
    entities: HashMap<NetEntity, ClientEntity>,
}

impl ClientEntities {
    pub fn spawn(&mut self, state: EntityState) {
        let pose = ModelPose {
            position: DVec3::from_array(state.position),
            yaw: state.yaw,
        };
        let entity = ClientEntity {
            kind: state.kind,
            item: state.item,
            age: 0.0,
            previous: pose,
            current: pose,
            elapsed: 0.0,
        };
        self.entities.insert(state.id, entity);
    }

    /// The entity glides from where it is drawn now to the new pose over one tick.
    pub fn moved(&mut self, id: NetEntity, position: [f64; 3], yaw: f32) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.previous = entity.pose();
            entity.current = ModelPose {
                position: DVec3::from_array(position),
                yaw,
            };
            entity.elapsed = 0.0;
        }
    }

    pub fn remove(&mut self, id: NetEntity) {
        self.entities.remove(&id);
    }

    pub fn advance(&mut self, seconds: f64) {
        for entity in self.entities.values_mut() {
            entity.elapsed += seconds;
            entity.age += seconds;
        }
    }

    /// Where the entity is drawn now.
    pub fn pose(&self, id: NetEntity) -> Option<ModelPose> {
        self.entities.get(&id).map(ClientEntity::pose)
    }

    pub fn kind(&self, id: NetEntity) -> Option<EntityTypeId> {
        self.entities.get(&id).map(|entity| entity.kind)
    }

    /// What each entity looks like now: its model and pose.
    pub fn models(&self) -> impl Iterator<Item = (ModelKind, ModelPose)> + '_ {
        self.entities
            .values()
            .map(|entity| (entity.model(), entity.pose()))
    }

    /// Every entity with its type and current pose.
    pub fn iter(&self) -> impl Iterator<Item = (NetEntity, EntityTypeId, ModelPose)> + '_ {
        self.entities
            .iter()
            .map(|(&id, entity)| (id, entity.kind, entity.pose()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state(x: f64, yaw: f32) -> EntityState {
        EntityState {
            id: NetEntity(1),
            kind: EntityTypeId(0),
            position: [x, 64.0, 0.0],
            yaw,
            item: None,
        }
    }

    #[test]
    fn moves_are_spread_over_one_tick() {
        let mut entities = ClientEntities::default();
        entities.spawn(state(0.0, 0.0));
        entities.moved(NetEntity(1), [1.0, 64.0, 0.0], 0.5);
        entities.advance(TICK / 2.0);
        let halfway = entities.pose(NetEntity(1)).unwrap();
        assert!((halfway.position.x - 0.5).abs() < 1e-9);
        assert!((halfway.yaw - 0.25).abs() < 1e-6);
        entities.advance(TICK);
        assert_eq!(entities.pose(NetEntity(1)).unwrap().position.x, 1.0);
        entities.remove(NetEntity(1));
        assert!(entities.pose(NetEntity(1)).is_none());
    }

    #[test]
    fn turning_past_north_takes_the_short_way() {
        let turn = shortest_turn(6.2, 0.1);
        assert!((turn - (0.1 + std::f32::consts::TAU - 6.2)).abs() < 1e-5);
    }
}
