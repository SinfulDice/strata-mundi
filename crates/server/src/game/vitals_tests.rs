#![allow(clippy::unwrap_used)]

use super::storage_tests::{item, survival};
use super::tests::{World, block, wait, world};
use super::*;
use strata_core::inventory::{ARMOR, ItemStack};
use strata_core::physics::MoveInput;
use strata_core::survival::{EffectGrant, SurvivalOptions, engine};
use strata_protocol::ServerMessage;

pub(super) fn id(text: &str) -> strata_core::id::Identifier {
    text.parse().unwrap()
}

/// Runs `ticks` ticks, one `input` each.
pub(super) fn walk(w: &mut World, input: MoveInput, ticks: u32) {
    for _ in 0..ticks {
        let sequence = w.game.last_sequence() + 1;
        w.game.queue_input(sequence, input);
        w.game.tick(&mut w.loader);
    }
}

pub(super) fn health(w: &World) -> f32 {
    w.game.ecs.healths.get(w.game.player).unwrap().current
}

pub(super) fn set_health(w: &mut World, value: f32) {
    w.game.ecs.healths.get_mut(w.game.player).unwrap().current = value;
}

pub(super) fn needs(w: &mut World) -> &mut strata_core::survival::Needs {
    w.game.ecs.needs.get_mut(w.game.player).unwrap()
}

pub(super) fn hurts(w: &mut World) -> Vec<(String, f32)> {
    w.game
        .take_messages()
        .into_iter()
        .filter_map(|message| match message {
            ServerMessage::Hurt(hurt) => Some((hurt.kind.to_string(), hurt.amount)),
            _ => None,
        })
        .collect()
}

pub(super) fn put_block(w: &mut World, pos: BlockPos, name: &str) {
    let block = block(&w.content, name);
    w.loader.set_block(pos, block);
}

pub(super) fn place_body(w: &mut World, at: DVec3) {
    let body = w.game.ecs.bodies.get_mut(w.game.player).unwrap();
    body.position = at;
    body.velocity = DVec3::ZERO;
    body.on_ground = false;
}

pub(super) fn give_hand(w: &mut World, name: &str, count: u16) {
    let stack = ItemStack::new(item(w, name), count, &w.game.rules);
    w.game
        .with_inventory(|inventory, _| *inventory.selected_slot_mut() = Some(stack));
}

/// Holds the use button on the item in hand until it is consumed once.
pub(super) fn consume_held(w: &mut World) {
    let held = w.game.held_item().unwrap();
    let ticks = w.game.rules.survival.consumables[&held].ticks;
    w.game.start_use(held, 0);
    wait(w, u32::from(ticks));
    w.game.stop_use();
}

#[test]
fn a_fed_player_heals_and_effort_makes_hungry() {
    let mut w = survival();
    set_health(&mut w, 10.0);
    wait(&mut w, 20);
    assert!(health(&w) > 10.0, "full food and saturation heal");
    let sprint = MoveInput {
        wish: [1.0, 0.0],
        sprint: true,
        yaw: 0.0,
        ..MoveInput::default()
    };
    place_body(&mut w, DVec3::new(1.5, 64.0, 8.5));
    let before = needs(&mut w).clone();
    walk(&mut w, sprint, 20);
    let after = needs(&mut w).clone();
    assert!(after.stamina < before.stamina, "sprinting tires");
    let spent = (after.saturation - before.saturation).abs() + after.exhaustion;
    assert!(spent > before.exhaustion, "sprinting makes hungry");
}

#[test]
fn heavy_armour_lowers_the_stamina_and_hunger_stops_the_sprint() {
    let mut w = survival();
    let full = w.game.vitals_view().unwrap().max_stamina;
    for (slot, piece) in ARMOR.zip([
        "strata:iron_plate_helmet",
        "strata:iron_plate_chestplate",
        "strata:iron_plate_leggings",
        "strata:iron_plate_boots",
    ]) {
        let stack = ItemStack::new(item(&w, piece), 1, &w.game.rules);
        w.game
            .with_inventory(|inventory, _| inventory.slots.slots[slot] = Some(stack));
    }
    wait(&mut w, 1);
    let view = w.game.vitals_view().unwrap();
    assert!(view.max_stamina < full, "{} < {full}", view.max_stamina);
    assert!(view.can_sprint);
    needs(&mut w).food = 6.0;
    wait(&mut w, 1);
    assert!(!w.game.vitals_view().unwrap().can_sprint);
    assert!(!w.game.movement().unwrap().can_sprint);
}

#[test]
fn falls_hurt_and_armour_softens_them() {
    let mut w = survival();
    place_body(&mut w, DVec3::new(8.5, 80.0, 8.5));
    walk(&mut w, MoveInput::default(), 60);
    let fall = health(&w);
    assert!(fall < 20.0 && fall > 0.0, "{fall}");
    let mut armoured = survival();
    let stack = ItemStack::new(
        item(&armoured, "strata:iron_plate_chestplate"),
        1,
        &armoured.game.rules,
    );
    armoured
        .game
        .with_inventory(|inventory, _| inventory.slots.slots[ARMOR.start + 1] = Some(stack));
    place_body(&mut armoured, DVec3::new(8.5, 80.0, 8.5));
    walk(&mut armoured, MoveInput::default(), 60);
    assert!(health(&armoured) > fall);
    let mut low = survival();
    place_body(&mut low, DVec3::new(8.5, 66.5, 8.5));
    walk(&mut low, MoveInput::default(), 30);
    assert_eq!(health(&low), 20.0, "a short drop does not hurt");
}

#[test]
fn water_drowns_lava_and_fire_burn_and_blocks_suffocate() {
    let mut w = survival();
    for y in 64..68 {
        put_block(&mut w, BlockPos::new(8, y, 8), "strata:water");
    }
    place_body(&mut w, DVec3::new(8.5, 64.0, 8.5));
    let breath = w.game.vitals_view().unwrap().max_breath;
    wait(&mut w, u32::from(breath) + 25);
    assert!(
        hurts(&mut w)
            .iter()
            .any(|(kind, _)| kind == "strata:drowning")
    );

    let mut w = survival();
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:lava");
    wait(&mut w, 10);
    let lava = hurts(&mut w);
    assert!(
        lava.iter().any(|(kind, _)| kind == "strata:lava"),
        "{lava:?}"
    );
    let effects = w.game.ecs.effects.get(w.game.player).unwrap();
    assert!(effects.has(&id("strata:burning")));

    let mut w = survival();
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:fire");
    wait(&mut w, 10);
    assert!(hurts(&mut w).iter().any(|(kind, _)| kind == "strata:fire"));

    let mut w = survival();
    put_block(&mut w, BlockPos::new(8, 65, 8), "strata:stone");
    wait(&mut w, 20);
    assert!(
        hurts(&mut w)
            .iter()
            .any(|(kind, _)| kind == "strata:suffocation")
    );
}

#[test]
fn starving_and_thirst_hurt_slowly_and_never_kill_alone() {
    let mut w = survival();
    needs(&mut w).food = 0.0;
    needs(&mut w).saturation = 0.0;
    needs(&mut w).thirst = 0.0;
    wait(&mut w, 80);
    let kinds: Vec<String> = hurts(&mut w).into_iter().map(|(kind, _)| kind).collect();
    assert!(kinds.contains(&"strata:starvation".to_owned()));
    assert!(kinds.contains(&"strata:dehydration".to_owned()));
    wait(&mut w, 80 * 40);
    assert!(health(&w) >= 1.0);
}

#[test]
fn invulnerability_holds_back_a_second_hit() {
    let mut w = survival();
    let fall = engine("fall");
    assert_eq!(w.game.hurt(w.game.player, &fall, 4.0), 4.0);
    assert_eq!(w.game.hurt(w.game.player, &fall, 3.0), 0.0);
    wait(&mut w, 11);
    assert_eq!(w.game.hurt(w.game.player, &fall, 3.0), 3.0);
}

#[test]
fn fire_resistance_from_an_effect_lessens_burns() {
    let mut w = survival();
    let mut content_survival = (*w.game.rules.survival).clone();
    content_survival.effects.insert(
        id("strata:test_resistance"),
        strata_core::survival::StatusEffectDef {
            harmful: false,
            heal: None,
            damage: None,
            ends_in_water: false,
            stats: vec![(id("strata:fire_resistance"), 50.0)],
        },
    );
    w.game.rules.survival = std::sync::Arc::new(content_survival);
    w.game
        .ecs
        .effects
        .get_mut(w.game.player)
        .unwrap()
        .give(&EffectGrant {
            effect: id("strata:test_resistance"),
            seconds: 10.0,
            strength: 1.0,
        });
    assert_eq!(w.game.hurt(w.game.player, &id("strata:lava"), 4.0), 2.0);
}

#[test]
fn at_zero_health_the_player_wakes_at_the_spawn_keeping_everything() {
    let mut w = survival();
    w.game.set_spawn(DVec3::new(2.5, 64.0, 2.5));
    give_hand(&mut w, "strata:stone", 5);
    place_body(&mut w, DVec3::new(8.5, 64.0, 8.5));
    w.game.hurt(w.game.player, &engine("fall"), 100.0);
    wait(&mut w, 1);
    assert_eq!(health(&w), 20.0);
    assert_eq!(
        w.game.player_body().unwrap().position,
        DVec3::new(2.5, 64.0, 2.5)
    );
    assert_eq!(w.game.held_item(), Some(item(&w, "strata:stone")));
    assert!(w.game.take_messages().contains(&ServerMessage::Fainted));
}

#[test]
fn creative_players_never_hunger_nor_get_hurt() {
    let mut w = world();
    needs(&mut w).food = 10.0;
    let sprint = MoveInput {
        wish: [1.0, 0.0],
        sprint: true,
        ..MoveInput::default()
    };
    walk(&mut w, sprint, 40);
    assert_eq!(needs(&mut w).food, 10.0);
    assert_eq!(w.game.hurt(w.game.player, &engine("fall"), 10.0), 0.0);
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:lava");
    wait(&mut w, 20);
    assert_eq!(health(&w), 20.0);
    assert!(
        w.game.vitals_view().unwrap().food.is_none(),
        "hidden in creative"
    );
}

#[test]
fn world_options_turn_hunger_and_thirst_off() {
    let mut w = survival();
    w.game.set_survival_options(SurvivalOptions {
        hunger: false,
        thirst: false,
        stamina: true,
        rate: 100,
    });
    needs(&mut w).food = 3.0;
    needs(&mut w).thirst = 0.0;
    wait(&mut w, 400);
    assert!(hurts(&mut w).is_empty());
    let view = w.game.vitals_view().unwrap();
    assert!(view.food.is_none() && view.thirst.is_none());
    assert!(view.can_sprint, "hunger off: sprinting whatever the food");
}

#[test]
fn the_gauges_are_saved_and_come_back_bounded() {
    let mut w = survival();
    set_health(&mut w, 7.0);
    needs(&mut w).food = 9.0;
    needs(&mut w).thirst = 4.0;
    w.game
        .ecs
        .effects
        .get_mut(w.game.player)
        .unwrap()
        .give(&EffectGrant {
            effect: id("strata:bleeding"),
            seconds: 30.0,
            strength: 1.0,
        });
    let saved = w.game.saved_vitals().unwrap();
    let text = ron::to_string(&saved).unwrap();
    let mut back = survival();
    back.game.restore_vitals(ron::from_str(&text).unwrap());
    assert_eq!(health(&back), 7.0);
    assert_eq!(needs(&mut back).food, 9.0);
    assert_eq!(needs(&mut back).thirst, 4.0);
    assert!(
        back.game
            .ecs
            .effects
            .get(back.game.player)
            .unwrap()
            .has(&id("strata:bleeding"))
    );
    let mut broken = saved;
    broken.health = f32::NAN;
    broken.needs.food = 400.0;
    back.game.restore_vitals(broken);
    assert_eq!(health(&back), 20.0);
    assert_eq!(needs(&mut back).food, 20.0);
}

#[test]
fn breaking_blocks_is_effort() {
    let mut w = survival();
    let before = needs(&mut w).exhaustion;
    let pos = BlockPos::new(9, 63, 8);
    super::tests::mine(&mut w, pos);
    let after = needs(&mut w).exhaustion + needs(&mut w).saturation;
    assert!(after > before, "{after} > {before}");
}

#[test]
fn touching_fire_hurts_at_its_pace_not_every_tick() {
    let mut w = survival();
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:fire");
    wait(&mut w, 20);
    let taken = 20.0 - health(&w);
    assert!(taken > 0.0 && taken <= 4.0, "{taken} in one second");
}
