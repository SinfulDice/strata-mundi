#![allow(clippy::unwrap_used)]

use super::*;
use crate::game::tests::{World, block, wait, world};
use crate::systems::SLEEP_TICKS;
use strata_core::content::{BlockId, ItemId};
use strata_core::physics::MoveInput;
use strata_protocol::HotbarState;

/// Water (surface 63.875) over x 10-15, a shallow strip at x = 9 (stone up to y = 62),
/// stone to y = 63 for x ≤ 8, where the player stands.
fn harbour() -> World {
    let mut w = world();
    let water = block(&w.content, "strata:water");
    for z in 0..16 {
        assert!(w.loader.set_block(BlockPos::new(9, 63, z), BlockId::AIR));
        for (x, y) in (10..16).flat_map(|x| (61..64).map(move |y| (x, y))) {
            assert!(w.loader.set_block(BlockPos::new(x, y, z), water));
        }
    }
    let boat = w.content.items.key(&"strata:oak_boat".parse().unwrap());
    let slots = vec![boat];
    assert!(w.game.set_hotbar(HotbarState { slots, selected: 0 }));
    w
}

fn boat_item(w: &World) -> ItemId {
    w.content
        .items
        .key(&"strata:oak_boat".parse().unwrap())
        .unwrap()
}

fn put_boat(w: &mut World) -> Entity {
    let item = boat_item(w);
    let against = BlockPos::new(10, 63, 8);
    w.game.use_item(&mut w.loader, item, against, Face::Up);
    let (entity, _) = w.game.ecs.boats.iter().next().expect("a boat was put down");
    entity
}

fn net(entity: Entity) -> NetEntity {
    NetEntity(entity.to_bits())
}

fn drive(w: &mut World, input: MoveInput, ticks: u32) {
    for sequence in 0..ticks {
        w.game.queue_input(sequence, input);
        w.game.tick(&mut w.loader);
    }
}

#[test]
fn a_boat_put_on_water_floats_at_the_surface() {
    let mut w = harbour();
    let boat = put_boat(&mut w);
    let messages = w.game.take_messages();
    assert!(matches!(messages[..], [ServerMessage::EntitySpawned(_)]));
    wait(&mut w, 60);
    let body = w.game.ecs.boats.get(boat).unwrap().body;
    let draft = 0.1875;
    assert!(
        (body.position.y - (63.875 - draft)).abs() < 0.01,
        "{body:?}"
    );
    assert!(
        w.game
            .take_messages()
            .iter()
            .all(|m| matches!(m, ServerMessage::EntityMoved { .. }))
    );
}

#[test]
fn the_pilot_steers_the_boat_and_gets_off_onto_the_shore() {
    let mut w = harbour();
    let boat = put_boat(&mut w);
    wait(&mut w, 40);
    assert!(w.game.interact_entity(net(boat)));
    assert!(
        w.game
            .take_messages()
            .contains(&ServerMessage::Mounted(Some(net(boat))))
    );
    let start = w.game.ecs.boats.get(boat).unwrap().body.position;
    let north = MoveInput {
        wish: [0.0, 1.0],
        ..MoveInput::default()
    };
    drive(&mut w, north, 10);
    let moved = w.game.ecs.boats.get(boat).unwrap().body.position;
    assert!(start.z - moved.z > 1.0, "{start} → {moved}");
    assert!((moved.x - start.x).abs() < 1e-3);
    let rider = w.game.player_body().unwrap().position;
    assert!(
        (rider - moved - DVec3::Y * -0.3).length() < 1e-9,
        "sits in it"
    );
    drive(&mut w, MoveInput::default(), 20);
    let sneak = MoveInput {
        sneak: true,
        ..MoveInput::default()
    };
    drive(&mut w, sneak, 1);
    assert!(w.game.ecs.mounts.get(w.game.player).is_none());
    assert!(
        w.game
            .take_messages()
            .contains(&ServerMessage::Mounted(None))
    );
    let feet = w.game.player_body().unwrap().position;
    assert!(feet.x < 10.0 && (feet.y - 63.0).abs() < 1e-9, "{feet}");
}

#[test]
fn a_held_sneak_does_not_throw_the_player_out_and_boats_take_one_rider() {
    let mut w = harbour();
    let boat = put_boat(&mut w);
    let sneak = MoveInput {
        sneak: true,
        ..MoveInput::default()
    };
    drive(&mut w, sneak, 1);
    wait(&mut w, 2);
    assert!(w.game.interact_entity(net(boat)));
    drive(&mut w, sneak, 5);
    assert!(w.game.ecs.mounts.get(w.game.player).is_some());
    wait(&mut w, 2);
    assert!(!w.game.interact_entity(net(boat)), "already riding");
}

#[test]
fn creative_players_take_boats_away_and_forged_ids_find_nothing() {
    let mut w = harbour();
    let boat = put_boat(&mut w);
    wait(&mut w, 2);
    assert!(!w.game.interact_entity(NetEntity(u64::MAX)));
    assert!(w.game.interact_entity(net(boat)));
    wait(&mut w, 2);
    assert!(w.game.attack_entity(&w.loader, net(boat)));
    assert!(w.game.ecs.boats.get(boat).is_none());
    assert!(w.game.ecs.mounts.get(w.game.player).is_none());
    let messages = w.game.take_messages();
    assert!(messages.contains(&ServerMessage::EntityRemoved(net(boat))));
}

#[test]
fn boats_are_not_put_inside_blocks() {
    let mut w = harbour();
    let item = boat_item(&w);
    let wall = BlockPos::new(8, 63, 8);
    w.game.use_item(&mut w.loader, item, wall, Face::East);
    assert!(
        w.game.ecs.boats.iter().next().is_none(),
        "would overlap the stone"
    );
}

#[test]
fn an_empty_still_boat_sleeps_until_a_block_beside_it_changes_or_it_is_boarded() {
    let mut w = harbour();
    let boat = put_boat(&mut w);
    wait(&mut w, 80);
    let asleep = |w: &World| w.game.ecs.boats.get(boat).unwrap().still_ticks >= SLEEP_TICKS;
    assert!(asleep(&w), "floating still without a pilot");
    let body = w.game.ecs.boats.get(boat).unwrap().body;
    w.game.ecs.boats.get_mut(boat).unwrap().body.velocity = DVec3::X;
    wait(&mut w, 5);
    assert_eq!(
        w.game.ecs.boats.get(boat).unwrap().body.position,
        body.position,
        "a sleeping boat is not simulated"
    );
    w.game.ecs.boats.get_mut(boat).unwrap().body.velocity = DVec3::ZERO;
    let far = BlockPos::new(8, 63, 4);
    assert!(!w.game.break_block(&mut w.loader, far).is_empty());
    assert!(asleep(&w), "the change is too far away");
    wait(&mut w, 2);
    let beside = BlockPos::new(11, 63, 8);
    assert!(!w.game.break_block(&mut w.loader, beside).is_empty());
    assert!(!asleep(&w), "the water beside it changed");
    wait(&mut w, 120);
    assert!(asleep(&w), "settled again");
    assert!(w.game.interact_entity(net(boat)));
    wait(&mut w, 1);
    assert!(!asleep(&w), "a pilot keeps it awake");
}
