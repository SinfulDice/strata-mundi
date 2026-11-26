#![allow(clippy::unwrap_used)]

use super::storage_tests::{item, survival};
use super::tests::{hold, wait};
use super::vitals_tests::{
    consume_held, give_hand, health, id, needs, put_block, set_health, walk,
};
use super::*;
use strata_core::physics::MoveInput;
use strata_core::survival::EffectGrant;

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
fn care_is_not_wasted_on_the_healthy() {
    let mut w = survival();
    give_hand(&mut w, "strata:bandage", 2);
    consume_held(&mut w);
    assert_eq!(w.game.inventory().selected_stack().unwrap().count, 2);
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
    consume_held(&mut w);
    assert_eq!(
        w.game.inventory().selected_stack().unwrap().count,
        1,
        "bleeding"
    );
}

#[test]
fn in_creative_a_bottle_fills_without_being_used_up() {
    let mut w = super::tests::world();
    let water = BlockPos::new(8, 63, 10);
    put_block(&mut w, water, "strata:water");
    give_hand(&mut w, "strata:glass_bottle", 1);
    let bottle = item(&w, "strata:glass_bottle");
    assert!(w.game.fill_held(&w.loader, bottle, water));
    assert_eq!(w.game.held_item(), Some(bottle));
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
}
