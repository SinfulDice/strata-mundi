#![allow(clippy::unwrap_used)]

use super::storage_tests::{item, survival};
use super::tests::{World, block, hold, wait, world};
use super::*;
use strata_core::inventory::{ARMOR, ItemStack};
use strata_core::survival::{EffectGrant, SurvivalOptions, engine};
use strata_protocol::ServerMessage;

fn id(text: &str) -> strata_core::id::Identifier {
    text.parse().unwrap()
}

/// Runs `ticks` ticks, one `input` each.
fn walk(w: &mut World, input: MoveInput, ticks: u32) {
    for _ in 0..ticks {
        let sequence = w.game.last_sequence() + 1;
        w.game.queue_input(sequence, input);
        w.game.tick(&mut w.loader);
    }
}

fn health(w: &World) -> f32 {
    w.game.ecs.healths.get(w.game.player).unwrap().current
}

fn set_health(w: &mut World, value: f32) {
    w.game.ecs.healths.get_mut(w.game.player).unwrap().current = value;
}

fn needs(w: &mut World) -> &mut strata_core::survival::Needs {
    w.game.ecs.needs.get_mut(w.game.player).unwrap()
}

fn hurts(w: &mut World) -> Vec<(String, f32)> {
    w.game
        .take_messages()
        .into_iter()
        .filter_map(|message| match message {
            ServerMessage::Hurt(hurt) => Some((hurt.kind.to_string(), hurt.amount)),
            _ => None,
        })
        .collect()
}

fn put_block(w: &mut World, pos: BlockPos, name: &str) {
    let block = block(&w.content, name);
    w.loader.set_block(pos, block);
}

fn place_body(w: &mut World, at: DVec3) {
    let body = w.game.ecs.bodies.get_mut(w.game.player).unwrap();
    body.position = at;
    body.velocity = DVec3::ZERO;
    body.on_ground = false;
}

fn give_hand(w: &mut World, name: &str, count: u16) {
    let stack = ItemStack::new(item(w, name), count, &w.game.rules);
    w.game
        .with_inventory(|inventory, _| *inventory.selected_slot_mut() = Some(stack));
}

/// Holds the use button on the item in hand until it is consumed once.
fn consume_held(w: &mut World) {
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
    wait(&mut w, 2);
    let lava = hurts(&mut w);
    assert!(
        lava.iter().any(|(kind, _)| kind == "strata:lava"),
        "{lava:?}"
    );
    let effects = w.game.ecs.effects.get(w.game.player).unwrap();
    assert!(effects.has(&id("strata:burning")));

    let mut w = survival();
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:fire");
    wait(&mut w, 2);
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
    assert_eq!(w.game.hurt(&fall, 4.0), 4.0);
    assert_eq!(w.game.hurt(&fall, 3.0), 0.0);
    wait(&mut w, 11);
    assert_eq!(w.game.hurt(&fall, 3.0), 3.0);
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
    assert_eq!(w.game.hurt(&id("strata:lava"), 4.0), 2.0);
}

#[test]
fn at_zero_health_the_player_wakes_at_the_spawn_keeping_everything() {
    let mut w = survival();
    w.game.set_spawn(DVec3::new(2.5, 64.0, 2.5));
    give_hand(&mut w, "strata:stone", 5);
    place_body(&mut w, DVec3::new(8.5, 64.0, 8.5));
    w.game.hurt(&engine("fall"), 100.0);
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
fn food_is_eaten_by_holding_the_button_and_leaves_its_bowl() {
    let mut w = survival();
    needs(&mut w).food = 10.0;
    give_hand(&mut w, "strata:mushroom_stew", 1);
    let held = w.game.held_item().unwrap();
    w.game.start_use(held, 0);
    wait(&mut w, 10);
    w.game.stop_use();
    wait(&mut w, 40);
    assert_eq!(needs(&mut w).food, 10.0, "let go too soon");
    consume_held(&mut w);
    assert_eq!(needs(&mut w).food, 16.0);
    assert_eq!(w.game.held_item(), Some(item(&w, "strata:bowl")));
    let effects = w.game.ecs.effects.get(w.game.player).unwrap();
    assert!(effects.has(&id("strata:healing")));
    needs(&mut w).food = 20.0;
    give_hand(&mut w, "strata:apple", 3);
    consume_held(&mut w);
    assert_eq!(
        w.game.inventory().selected_stack().unwrap().count,
        3,
        "not hungry"
    );
}

#[test]
fn bandages_stop_bleeding_and_healing_never_stacks() {
    let mut w = survival();
    set_health(&mut w, 8.0);
    let effects = w.game.ecs.effects.get_mut(w.game.player).unwrap();
    effects.give(&EffectGrant {
        effect: id("strata:bleeding"),
        seconds: 60.0,
        strength: 1.0,
    });
    effects.give(&EffectGrant {
        effect: id("strata:healing"),
        seconds: 10.0,
        strength: 0.5,
    });
    give_hand(&mut w, "strata:bandage", 2);
    consume_held(&mut w);
    let effects = w.game.ecs.effects.get(w.game.player).unwrap();
    assert!(!effects.has(&id("strata:bleeding")));
    let healing: Vec<_> = (effects.0.iter())
        .filter(|e| e.effect == id("strata:healing"))
        .collect();
    assert_eq!(healing.len(), 1);
    assert_eq!(
        healing[0].strength, 1.0,
        "the bandage is stronger than the stew"
    );
    let before = health(&w);
    wait(&mut w, 60);
    assert!(health(&w) > before);
}

#[test]
fn bottles_fill_salty_by_the_sea_and_the_smoker_purifies() {
    let mut w = survival();
    let water = BlockPos::new(8, 63, 10);
    put_block(&mut w, water, "strata:water");
    give_hand(&mut w, "strata:glass_bottle", 2);
    let bottle = item(&w, "strata:glass_bottle");
    w.game.set_biomes(vec![id("strata:plains")]);
    assert!(w.game.fill_held(&w.loader, bottle, water));
    assert_eq!(w.game.inventory().selected_stack().unwrap().count, 1);
    wait(&mut w, 2);
    w.game.set_biomes(vec![id("strata:ocean")]);
    assert!(w.game.fill_held(&w.loader, bottle, water));
    assert_eq!(
        w.game.held_item(),
        Some(item(&w, "strata:salt_water_bottle"))
    );
    let fresh = item(&w, "strata:water_bottle");
    assert!(
        w.game
            .inventory()
            .slots
            .slots
            .iter()
            .flatten()
            .any(|s| s.item == fresh)
    );
    let salty = item(&w, "strata:salt_water_bottle");
    let smoker = [id("strata:food")];
    let recipe = w.game.rules.cooking.recipe_for(salty, &smoker).unwrap();
    assert_eq!(recipe.result, fresh);
    needs(&mut w).thirst = 10.0;
    consume_held(&mut w);
    assert_eq!(needs(&mut w).thirst, 7.0, "salt water dries one out");
    assert_eq!(w.game.held_item(), Some(bottle));
}

#[test]
fn a_waterskin_gives_four_sips_and_one_drinks_at_the_source_sneaking() {
    let mut w = survival();
    needs(&mut w).thirst = 0.0;
    give_hand(&mut w, "strata:waterskin_of_water", 1);
    for _ in 0..3 {
        consume_held(&mut w);
        assert_eq!(
            w.game.held_item(),
            Some(item(&w, "strata:waterskin_of_water"))
        );
    }
    consume_held(&mut w);
    assert_eq!(w.game.held_item(), Some(item(&w, "strata:waterskin")));
    assert_eq!(needs(&mut w).thirst, 20.0);
    needs(&mut w).thirst = 5.0;
    hold(&mut w, None);
    let water = BlockPos::new(8, 63, 10);
    put_block(&mut w, water, "strata:water");
    w.game.set_biomes(vec![id("strata:river")]);
    assert!(!w.game.drink_at(&w.loader, water), "standing up");
    let sneak = MoveInput {
        sneak: true,
        ..MoveInput::default()
    };
    walk(&mut w, sneak, 3);
    assert!(w.game.drink_at(&w.loader, water));
    assert_eq!(needs(&mut w).thirst, 7.0);
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
    assert_eq!(w.game.hurt(&engine("fall"), 10.0), 0.0);
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
