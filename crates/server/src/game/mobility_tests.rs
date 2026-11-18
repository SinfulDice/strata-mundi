#![allow(clippy::unwrap_used)]

use super::terrain::WorldTerrain;
use super::tests::{World, wait, world};
use super::*;
use strata_core::inventory::{ACCESSORIES, ARMOR, ItemStack};
use strata_core::physics::{MoveInput, step};
use strata_core::worn::inventory_worn_stats;

fn equipped() -> World {
    let mut w = world();
    w.game.set_mode(GameMode::Survival);
    let give = |w: &mut World, slot: usize, name: &str| {
        let item = w.content.items.key(&name.parse().unwrap()).unwrap();
        let stack = ItemStack::new(item, 1, &w.game.rules);
        w.game
            .with_inventory(|inventory, _| inventory.slots.slots[slot] = Some(stack));
    };
    give(&mut w, ARMOR.end - 1, "strata:leaping_boots");
    give(&mut w, ACCESSORIES.start + 3, "strata:dash_belt");
    wait(&mut w, 2);
    w
}

fn input(jump: bool, dash: bool) -> MoveInput {
    MoveInput {
        wish: [0.3, 1.0],
        jump,
        dash,
        ..MoveInput::default()
    }
}

/// Jumps, a second jump in the air, then dash pressed every tick.
fn inputs() -> Vec<MoveInput> {
    let mut list = vec![input(true, false)];
    list.extend((0..4).map(|_| input(false, false)));
    list.push(input(true, false));
    list.extend((0..40).map(|_| input(false, true)));
    list
}

/// The movement the client predicts with, from its copy of the inventory.
fn client_movement(w: &World) -> MovementSettings {
    let content = &w.content;
    let worn = inventory_worn_stats(
        &w.game.inventory(),
        &content.items,
        &content.upgrades,
        &content.worn,
    );
    MovementSettings {
        speed: f64::from(worn.burden.speed_factor()),
        mobility: content.worn.mobility.mobility(&worn),
        ..w.game.movement().unwrap()
    }
}

#[test]
fn the_client_s_prediction_matches_the_server_tick_after_tick() {
    let mut w = equipped();
    let movement = client_movement(&w);
    assert_eq!(movement, w.game.movement().unwrap());
    let mut predicted = *w.game.player_body().unwrap();
    let mut bodies = Vec::new();
    for (sequence, input) in inputs().into_iter().enumerate() {
        let terrain = WorldTerrain {
            loader: &w.loader,
            rules: &w.game.rules,
            border: w.game.border,
        };
        step(&mut predicted, &input, &movement, &terrain);
        bodies.push(predicted);
        w.game.queue_input(sequence as u32, input);
        wait(&mut w, 1);
        assert_eq!(*w.game.player_body().unwrap(), predicted, "tick {sequence}");
    }
    let from = 10;
    let mut replayed = bodies[from];
    let terrain = WorldTerrain {
        loader: &w.loader,
        rules: &w.game.rules,
        border: w.game.border,
    };
    for input in &inputs()[from + 1..] {
        step(&mut replayed, input, &movement, &terrain);
    }
    assert_eq!(replayed, predicted, "a reconcile replays without a gap");
}

#[test]
fn the_server_jumps_in_the_air_and_limits_dash_spam_by_the_cooldown() {
    let mut w = equipped();
    let cooldown = u32::from(w.content.worn.mobility.dash_cooldown);
    let mut dashes = 0;
    let mut air_jumps = 0;
    for (sequence, input) in inputs().into_iter().enumerate() {
        let before = w.game.player_body().unwrap().abilities;
        w.game.queue_input(sequence as u32, input);
        wait(&mut w, 1);
        let after = w.game.player_body().unwrap().abilities;
        dashes += u32::from(after.dash_cooldown > before.dash_cooldown);
        air_jumps = air_jumps.max(after.air_jumps_used);
    }
    assert_eq!(air_jumps, 1, "one jump in the air");
    let pressed: u32 = 40;
    assert_eq!(
        dashes,
        pressed.div_ceil(cooldown),
        "dash every {cooldown} ticks only"
    );
}
