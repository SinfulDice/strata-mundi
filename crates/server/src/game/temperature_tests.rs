#![allow(clippy::unwrap_used)]

use super::storage_tests::{item, survival};
use super::tests::{World, wait, world_with};
use super::vitals_tests::{health, hurts, id, needs, put_block};
use super::*;
use strata_core::inventory::{ACCESSORIES, ItemStack};
use strata_core::survival::temperature::{BodyTemperature, TemperatureZone};
use strata_core::survival::{EffectGrant, StatusEffectDef, SurvivalOptions};

/// A survival world whose only biome is `biome`, at `daylight`.
fn in_biome(biome: &str, daylight: f32) -> World {
    let mut w = survival();
    w.game.set_biomes(vec![id(biome)]);
    w.game.set_daylight(daylight);
    w
}

fn body(w: &World) -> &BodyTemperature {
    w.game.ecs.temperatures.get(w.game.player).unwrap()
}

fn felt(w: &World) -> f32 {
    body(w).felt.unwrap()
}

fn zone(w: &World) -> TemperatureZone {
    w.game.vitals_view().unwrap().temperature.unwrap().zone
}

fn wear_cloak(w: &mut World) {
    let cloak = ItemStack::new(item(w, "strata:wool_cloak"), 1, &w.game.rules);
    w.game.with_inventory(|inventory, _| {
        inventory.slots.slots[ACCESSORIES.end - 1] = Some(cloak);
    });
}

#[test]
fn the_biome_and_the_night_set_the_temperature() {
    let mut w = in_biome("strata:desert", 1.0);
    wait(&mut w, 1);
    assert_eq!(felt(&w), 40.0, "the first sample is taken at once");
    assert_eq!(zone(&w), TemperatureZone::Hot);
    let mut night = in_biome("strata:plains", 0.0);
    wait(&mut night, 1);
    assert_eq!(felt(&night), 12.0);
    assert_eq!(zone(&night), TemperatureZone::Comfortable);
}

#[test]
fn the_felt_temperature_follows_the_surroundings_slowly() {
    let mut w = in_biome("strata:plains", 1.0);
    wait(&mut w, 1);
    w.game.set_biomes(vec![id("strata:snowy_tundra")]);
    wait(&mut w, 40);
    let after = felt(&w);
    assert!(after < 20.0 && after > 10.0, "{after}");
    wait(&mut w, 2_400);
    assert!((felt(&w) + 3.0).abs() < 0.5, "{}", felt(&w));
}

#[test]
fn the_cold_slows_and_makes_hungry_then_freezes() {
    let mut w = in_biome("strata:snowy_tundra", 1.0);
    wait(&mut w, 2);
    assert_eq!(zone(&w), TemperatureZone::Cold);
    let view = w.game.vitals_view().unwrap();
    assert!(view.slowness > 0.1, "{}", view.slowness);
    assert!(w.game.movement().unwrap().speed < 0.9);
    let before = needs(&mut w).exhaustion - needs(&mut w).saturation * 4.0;
    wait(&mut w, 200);
    let after = needs(&mut w).exhaustion - needs(&mut w).saturation * 4.0;
    assert!(
        after - before > 0.5,
        "the cold eats saturation: {before} → {after}"
    );
    assert!(hurts(&mut w).is_empty(), "cold strains, does not hurt");
    let mut night = in_biome("strata:snowy_tundra", 0.0);
    wait(&mut night, 1);
    assert_eq!(zone(&night), TemperatureZone::Freezing);
    wait(&mut night, 160);
    let hits = hurts(&mut night);
    assert_eq!(hits.len(), 2, "{hits:?}");
    assert_eq!(hits[0].0, "strata:freezing");
}

#[test]
fn the_heat_makes_thirsty_and_tires() {
    let mut w = in_biome("strata:desert", 1.0);
    wait(&mut w, 1);
    let start = needs(&mut w).thirst_exhaustion;
    wait(&mut w, 100);
    let mut mild = in_biome("strata:plains", 1.0);
    wait(&mut mild, 101);
    let hot = needs(&mut w).thirst_exhaustion - start;
    let normal = needs(&mut mild).thirst_exhaustion;
    assert!(hot > normal * 2.0, "{hot} > 2 × {normal}");
    needs(&mut w).stamina = 0.0;
    needs(&mut mild).stamina = 0.0;
    wait(&mut w, 60);
    wait(&mut mild, 60);
    assert!(needs(&mut w).stamina < needs(&mut mild).stamina);
}

#[test]
fn a_fire_nearby_warms_and_lava_overheats() {
    let mut w = in_biome("strata:snowy_tundra", 0.0);
    put_block(&mut w, BlockPos::new(10, 64, 8), "strata:fire");
    wait(&mut w, 1);
    assert!(felt(&w) > -12.0 + 5.0, "{}", felt(&w));
    assert!(body(&w).heat > 0.0);
    let mut hot = in_biome("strata:desert", 1.0);
    put_block(&mut hot, BlockPos::new(10, 63, 8), "strata:lava");
    wait(&mut hot, 1);
    assert_eq!(zone(&hot), TemperatureZone::Scorching);
    wait(&mut hot, 80);
    assert!(
        hurts(&mut hot)
            .iter()
            .any(|(kind, _)| kind == "strata:overheating")
    );
}

#[test]
fn water_cools_and_the_body_dries_out_of_it() {
    let mut w = in_biome("strata:plains", 1.0);
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:water");
    wait(&mut w, 2);
    assert_eq!(body(&w).wetness, 1.0);
    assert!(w.game.vitals_view().unwrap().temperature.unwrap().wet);
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:air");
    wait(&mut w, 600);
    let wetness = body(&w).wetness;
    assert!(wetness > 0.4 && wetness < 0.6, "{wetness}");
    assert!(body(&w).target < 20.0, "still damp: {}", body(&w).target);
}

#[test]
fn a_wool_cloak_keeps_the_cold_out() {
    let mut w = in_biome("strata:snowy_tundra", 0.0);
    wear_cloak(&mut w);
    wait(&mut w, 1);
    assert_eq!(felt(&w), 0.0, "-12 + 4 points × 3");
    assert_eq!(zone(&w), TemperatureZone::Cold);
}

#[test]
fn cold_resistance_lessens_freezing() {
    let mut w = world_with(|content| {
        let warm = StatusEffectDef {
            harmful: false,
            heal: None,
            damage: None,
            ends_in_water: false,
            stats: vec![(id("strata:cold_resistance"), 100.0)],
        };
        content.survival.effects.insert(id("test:warm"), warm);
    });
    w.game.set_mode(strata_protocol::GameMode::Survival);
    w.game.set_biomes(vec![id("strata:snowy_tundra")]);
    w.game.set_daylight(0.0);
    let effects = w.game.ecs.effects.get_mut(w.game.player).unwrap();
    effects.give(&EffectGrant {
        effect: id("test:warm"),
        seconds: 60.0,
        strength: 1.0,
    });
    w.game.take_messages();
    wait(&mut w, 200);
    assert!(hurts(&mut w).is_empty());
    assert_eq!(health(&w), 20.0);
}

#[test]
fn the_option_off_and_creative_feel_nothing() {
    let mut w = in_biome("strata:snowy_tundra", 0.0);
    w.game.set_survival_options(SurvivalOptions {
        temperature: false,
        ..SurvivalOptions::default()
    });
    wait(&mut w, 200);
    assert!(hurts(&mut w).is_empty());
    let view = w.game.vitals_view().unwrap();
    assert!(view.temperature.is_none());
    assert_eq!(view.slowness, 0.0);
    assert!(body(&w).felt.is_none(), "not even sampled");
    let mut creative = in_biome("strata:snowy_tundra", 0.0);
    creative.game.set_mode(strata_protocol::GameMode::Creative);
    wait(&mut creative, 200);
    assert!(hurts(&mut creative).is_empty());
    assert!(creative.game.vitals_view().unwrap().temperature.is_none());
    assert_eq!(creative.game.movement().unwrap().speed, 1.0);
}

#[test]
fn the_temperature_and_wetness_are_saved() {
    let mut w = in_biome("strata:taiga", 0.0);
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:water");
    wait(&mut w, 2);
    put_block(&mut w, BlockPos::new(8, 64, 8), "strata:air");
    wait(&mut w, 100);
    let saved = w.game.saved_vitals().unwrap();
    let text = ron::to_string(&saved).unwrap();
    let mut back = in_biome("strata:desert", 1.0);
    back.game.restore_vitals(ron::from_str(&text).unwrap());
    assert_eq!(body(&back).felt, body(&w).felt);
    assert_eq!(body(&back).wetness, body(&w).wetness);
    assert!(body(&back).wetness > 0.0);
    wait(&mut back, 1);
    assert!(
        felt(&back) < 10.0,
        "no jump to the desert's heat: {}",
        felt(&back)
    );
}
