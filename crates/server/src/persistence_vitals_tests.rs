#![allow(clippy::unwrap_used)]

use super::tests::{content, new_world, settings};
use super::*;
use crate::loader::tests::loader;
use crate::rules::BlockRules;
use strata_core::survival::{EffectGrant, SurvivalOptions};
use strata_protocol::GameMode;

#[test]
fn the_gauges_and_the_survival_options_come_back() {
    let (saves, folder, mut meta) = new_world("vitals");
    assert_eq!(meta.survival, SurvivalOptions::default());
    meta.survival = SurvivalOptions {
        hunger: true,
        thirst: false,
        stamina: true,
        rate: 50,
    };
    folder.write_meta(&meta).unwrap();
    let content = content();
    let dimension = DimensionId("strata:terra".parse().unwrap());
    let meta = folder.read_meta().unwrap();
    let mut persistence = Persistence::new(settings(&folder, meta), Arc::clone(&content));
    let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
    let mut game = Game::new(BlockRules::new(&content), 5_000, body, GameMode::Survival);
    let mut saved = game.saved_vitals().unwrap();
    saved.health = 9.0;
    saved.needs.food = 11.0;
    saved.needs.thirst = 3.0;
    saved.effects.give(&EffectGrant {
        effect: "strata:bleeding".parse().unwrap(),
        seconds: 20.0,
        strength: 1.0,
    });
    game.restore_vitals(saved.clone());
    persistence.save(&game, &mut loader(), &dimension, None);

    let reopened = Persistence::new(settings(&folder, folder.read_meta().unwrap()), content);
    assert_eq!(reopened.meta.survival.rate, 50);
    assert!(!reopened.meta.survival.thirst);
    let start = reopened.player_start(&dimension, || DVec3::ZERO);
    assert_eq!(start.vitals, Some(saved));
    std::fs::remove_dir_all(saves).unwrap();
}
