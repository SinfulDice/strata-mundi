//! Boats: putting one down, getting in and out, taking it away (creative), and the
//! messages that tell clients about them.

use super::terrain::WorldTerrain;
use super::{Game, MAX_REACH};
use crate::ecs::{Boat, Entity};
use crate::loader::ColumnLoader;
use glam::{DVec2, DVec3};
use strata_core::content::{EntityTypeId, Face};
use strata_core::coords::BlockPos;
use strata_core::physics::{Aabb, BoatBody, HEIGHT, Terrain, WIDTH};
use strata_protocol::{EntityState, NetEntity, ServerMessage};

/// More boats than this in a world is surely a mistake (or a hostile client).
const MAX_BOATS: usize = 512;
/// Space kept between the boat and the player getting off beside it.
const EXIT_GAP: f64 = 0.1;

impl Game {
    /// A new boat of type `kind`; `None` if the type is no boat or there are too many.
    pub fn spawn_boat(&mut self, kind: EntityTypeId, body: BoatBody) -> Option<Entity> {
        self.rules.entity(kind)?.boat?;
        if self.ecs.boats.len() >= MAX_BOATS || !body.position.is_finite() {
            return None;
        }
        let entity = self.ecs.entities.spawn();
        let boat = Boat::new(kind, body);
        self.messages
            .push(ServerMessage::EntitySpawned(state(entity, &boat)));
        self.ecs.boats.insert(entity, boat);
        Some(entity)
    }

    /// Validated use of a boat item: on the surface of the aimed liquid, or on the aimed
    /// face of a block, where the boat fits, facing where the player looks.
    pub(super) fn place_entity(
        &mut self,
        loader: &ColumnLoader,
        kind: EntityTypeId,
        against: BlockPos,
        face: Face,
    ) -> Option<Entity> {
        let size = self.rules.entity(kind)?.size;
        let aimed = loader.block(against)?;
        let (cell, height) = match self.rules.liquid(aimed) {
            Some(liquid) => (against, liquid.surface),
            None => (self.placement_target(loader, against, face)?, 0.0),
        };
        let centre = DVec3::new(f64::from(cell.x), f64::from(cell.y), f64::from(cell.z));
        let position = centre + DVec3::new(0.5, height, 0.5);
        let body = BoatBody::at(position, self.look()[0]);
        let terrain = self.terrain(loader);
        let fits = !body.aabb(size).collides(&|x, y, z| terrain.solid(x, y, z));
        let allowed = fits && cell.is_valid(self.border) && self.in_reach(cell);
        if !allowed || !self.take_action() {
            return None;
        }
        self.spawn_boat(kind, body)
    }

    /// Right click on an entity: gets into a free boat within reach.
    pub fn interact_entity(&mut self, id: NetEntity) -> bool {
        let entity = Entity::from_bits(id.0);
        let free = self
            .ecs
            .boats
            .get(entity)
            .is_some_and(|boat| boat.pilot.is_none());
        let reachable = self.boat_box(entity).is_some_and(|b| self.reaches(&b));
        let walking = self.ecs.mounts.get(self.player).is_none();
        if !(free && reachable && walking) || !self.take_action() {
            return false;
        }
        self.mount(entity);
        true
    }

    /// Left click on an entity, creative only: the boat goes away (its rider gets off).
    pub fn attack_entity(&mut self, loader: &ColumnLoader, id: NetEntity) -> bool {
        let entity = Entity::from_bits(id.0);
        let reachable = self.boat_box(entity).is_some_and(|b| self.reaches(&b));
        if !(reachable && self.is_creative()) || !self.take_action() {
            return false;
        }
        if let Some(pilot) = self.ecs.boats.get(entity).and_then(|boat| boat.pilot) {
            self.dismount(loader, pilot);
        }
        self.ecs.despawn(entity);
        self.messages
            .push(ServerMessage::EntityRemoved(NetEntity(entity.to_bits())));
        true
    }

    /// Sits the player in `boat` (already checked to be free).
    pub(super) fn mount(&mut self, boat: Entity) {
        let player = self.player;
        let Some(seat) = self.ecs.boats.get_mut(boat) else {
            return;
        };
        seat.pilot = Some(player);
        self.ecs.mounts.insert(player, boat);
        if let Some(body) = self.ecs.bodies.get_mut(player) {
            body.flying = false;
            body.velocity = DVec3::ZERO;
        }
        self.messages
            .push(ServerMessage::Mounted(Some(NetEntity(boat.to_bits()))));
    }

    /// Unlinks the rider from their boat, leaving the body where it is.
    pub(super) fn leave_boat(&mut self, rider: Entity) -> Option<Entity> {
        let boat = self.ecs.mounts.remove(rider)?;
        if let Some(seat) = self.ecs.boats.get_mut(boat) {
            seat.pilot = None;
        }
        self.messages.push(ServerMessage::Mounted(None));
        Some(boat)
    }

    /// Gets off onto firm ground beside the boat if there is some, on top of it otherwise.
    pub(super) fn dismount(&mut self, loader: &ColumnLoader, rider: Entity) {
        let Some(boat) = self.leave_boat(rider) else {
            return;
        };
        let Some(spot) = self.exit_spot(loader, boat) else {
            return;
        };
        if let Some(body) = self.ecs.bodies.get_mut(rider) {
            body.position = spot;
            body.velocity = DVec3::ZERO;
        }
    }

    /// Standing ground beside the boat (at its water line or a block higher), the sides
    /// first, then behind and in front.
    fn exit_spot(&self, loader: &ColumnLoader, boat: Entity) -> Option<DVec3> {
        let seat = self.ecs.boats.get(boat)?;
        let size = self.rules.entity(seat.kind)?.size;
        let terrain = self.terrain(loader);
        let solid = |x, y, z| terrain.solid(x, y, z);
        let forward = seat.body.forward();
        let right = DVec2::new(-forward.y, forward.x);
        let reach = size.width / 2.0 + WIDTH / 2.0 + EXIT_GAP;
        let sides = [right, -right, -forward, forward].map(|side| side * reach);
        let ground = seat.body.position.y.floor();
        let beside = sides.into_iter().flat_map(|side| {
            [0.0, 1.0].map(|up| {
                let spot = seat.body.position + DVec3::new(side.x, 0.0, side.y);
                DVec3::new(spot.x, ground + up, spot.z)
            })
        });
        let on_top = seat.body.position + DVec3::Y * size.height;
        let standing = |feet: &DVec3| {
            let player = Aabb::standing(*feet, WIDTH, HEIGHT);
            !player.collides(&solid) && player.supported(0.6, &solid)
        };
        Some(beside.clone().find(standing).unwrap_or(on_top))
    }

    /// Spawn, move, removal and riding messages since the last call.
    pub fn take_messages(&mut self) -> Vec<ServerMessage> {
        std::mem::take(&mut self.messages)
    }

    /// Boats to save, and which one the player sits in (index in the list).
    pub fn saved_boats(&self) -> (Vec<(EntityTypeId, BoatBody)>, Option<usize>) {
        let mount = self.ecs.mounts.get(self.player).copied();
        let boats: Vec<_> = self.ecs.boats.iter().collect();
        let seat = boats.iter().position(|(entity, _)| Some(*entity) == mount);
        let list = boats
            .into_iter()
            .map(|(_, boat)| (boat.kind, boat.body))
            .collect();
        (list, seat)
    }

    /// Boats read from a save (`None` for one that could not be read back); the player
    /// gets back into the one they were in.
    pub fn restore_boats(
        &mut self,
        boats: Vec<Option<(EntityTypeId, BoatBody)>>,
        seat: Option<usize>,
    ) {
        let spawned: Vec<Option<Entity>> = boats
            .into_iter()
            .map(|boat| boat.and_then(|(kind, body)| self.spawn_boat(kind, body)))
            .collect();
        if let Some(boat) = seat.and_then(|index| spawned.get(index).copied().flatten()) {
            self.mount(boat);
        }
    }

    /// Sends the boats that moved this tick.
    pub(super) fn report_moves(&mut self, moved: &[Entity]) {
        for &entity in moved {
            if let Some(boat) = self.ecs.boats.get(entity) {
                self.messages.push(ServerMessage::EntityMoved {
                    id: NetEntity(entity.to_bits()),
                    position: boat.body.position.to_array(),
                    yaw: boat.body.yaw,
                });
            }
        }
    }

    fn boat_box(&self, entity: Entity) -> Option<Aabb> {
        let boat = self.ecs.boats.get(entity)?;
        Some(boat.body.aabb(self.rules.entity(boat.kind)?.size))
    }

    /// Whether the closest point of `target` is within reach of the eyes.
    fn reaches(&self, target: &Aabb) -> bool {
        self.player_body().is_some_and(|body| {
            let eyes = body.eyes();
            eyes.clamp(target.min, target.max).distance(eyes) <= MAX_REACH
        })
    }

    pub(super) fn terrain<'a>(&'a self, loader: &'a ColumnLoader) -> WorldTerrain<'a> {
        WorldTerrain {
            loader,
            rules: &self.rules,
            border: self.border,
        }
    }
}

fn state(entity: Entity, boat: &Boat) -> EntityState {
    EntityState {
        id: NetEntity(entity.to_bits()),
        kind: boat.kind,
        position: boat.body.position.to_array(),
        yaw: boat.body.yaw,
    }
}

#[cfg(test)]
mod tests;
