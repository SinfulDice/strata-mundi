#![allow(clippy::unwrap_used)]

use super::*;
use crate::rules::BlockRules;
use glam::DVec3;
use strata_core::coords::DEFAULT_WORLD_BORDER;
use strata_core::pack::PackSet;
use strata_core::physics::Body;
use strata_core::updates::{FireSettings, FluidSettings};

fn setup() -> (Game, WorldMeta, Content) {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, _) = Content::load(&packs, &PackSet::default());
    let body = Body::at(DVec3::new(0.5, 80.0, 0.5));
    let rules = BlockRules::new(&content);
    let game = Game::new(rules, DEFAULT_WORLD_BORDER, body, GameMode::Creative);
    let meta: WorldMeta = ron::from_str(
        r#"World(format: 1, game_version: "0.1.0", name: "t", seed: 7, seed_text: "",
            mode: Creative, border: 1000, created: 0, last_played: 0, play_ticks: 0)"#,
    )
    .unwrap();
    (game, meta, content)
}

#[test]
fn commands_change_the_world_and_answer() {
    let (mut game, mut meta, content) = setup();
    let mut console = Console {
        game: &mut game,
        meta: &mut meta,
        content: &content,
        cycle: DayCycle::default(),
    };
    assert_eq!(console.run("/tp ~ ~10 5").feedback.key, "command-tp-done");
    let given = console.run("give stone 100");
    assert!(!given.feedback.error);
    assert!(console.run("give stone zero").feedback.key == "command-usage");
    assert!(console.run("give strata:unobtainium").feedback.error);
    assert_eq!(console.run("time set noon").feedback.args[0].1, "6000");
    assert_eq!(console.run("seed").feedback.args[0].1, "7");
    assert_eq!(console.run("tp 1 2").feedback.key, "command-usage");
    assert_eq!(console.run("dance").feedback.key, "command-unknown");
    let feet = game.player_body().unwrap().position;
    assert_eq!((feet.y, feet.z), (90.0, 5.0));
    let stone = content.items.key(&"strata:stone".parse().unwrap()).unwrap();
    assert_eq!(
        game.inventory().slots.count(stone),
        100,
        "split into stacks of 64"
    );
    assert_eq!(meta.day_time, 6_000);
}

#[test]
fn fluids_turn_off_and_take_a_budget_saved_with_the_world() {
    let (mut game, mut meta, content) = setup();
    let mut console = Console {
        game: &mut game,
        meta: &mut meta,
        content: &content,
        cycle: DayCycle::default(),
    };
    let off = console.run("fluids off").feedback;
    assert_eq!(
        (off.key.as_str(), off.args[0].1.as_str()),
        ("command-fluids-now", "off")
    );
    assert_eq!(console.run("fluids 512").feedback.args[1].1, "512");
    assert!(console.run("fluids 0").feedback.error);
    assert_eq!(console.run("fluids lots").feedback.key, "command-usage");
    console.run("fluids on");
    assert_eq!(
        meta.fluids,
        FluidSettings {
            flow: true,
            updates_per_tick: 512
        }
    );
}

#[test]
fn fire_spread_and_destroy_are_saved_with_the_world() {
    let (mut game, mut meta, content) = setup();
    let mut console = Console {
        game: &mut game,
        meta: &mut meta,
        content: &content,
        cycle: DayCycle::default(),
    };
    let off = console.run("fire spread off").feedback;
    assert_eq!(off.key, "command-fire-now");
    assert_eq!(
        (off.args[0].1.as_str(), off.args[1].1.as_str()),
        ("off", "on")
    );
    assert_eq!(console.run("fire destroy off").feedback.args[1].1, "off");
    assert_eq!(console.run("fire query").feedback.args[0].1, "off");
    assert_eq!(
        console.run("fire destroy maybe").feedback.key,
        "command-usage"
    );
    assert_eq!(console.run("fire burn").feedback.key, "command-usage");
    console.run("fire spread on");
    assert_eq!(
        meta.fire,
        FireSettings {
            spread: true,
            destroy: false
        }
    );
}

#[test]
fn time_moves_forward_to_what_is_asked_and_counts_days() {
    let (mut game, mut meta, content) = setup();
    let mut console = Console {
        game: &mut game,
        meta: &mut meta,
        content: &content,
        cycle: DayCycle::default(),
    };
    let now = |console: &mut Console<'_>, line: &str| {
        let feedback = console.run(line).feedback;
        assert_eq!(feedback.key, "command-time-now", "{line}");
        (feedback.args[0].1.clone(), feedback.args[2].1.clone())
    };
    assert_eq!(
        now(&mut console, "time set midnight"),
        ("18000".into(), "1".into())
    );
    assert_eq!(
        now(&mut console, "time set noon"),
        ("6000".into(), "2".into())
    );
    assert_eq!(
        now(&mut console, "time set 7000"),
        ("7000".into(), "2".into())
    );
    assert_eq!(
        now(&mut console, "time add 24000"),
        ("7000".into(), "3".into())
    );
    assert_eq!(
        now(&mut console, "time set 31000"),
        ("7000".into(), "3".into())
    );
    assert_eq!(now(&mut console, "time"), ("7000".into(), "3".into()));
    assert_eq!(console.run("time add -5").feedback.key, "command-usage");
    assert_eq!(
        console.run("time set teatime").feedback.key,
        "command-usage"
    );
    let stopped = console.run("daycycle off").feedback;
    assert_eq!(stopped.args[0].1, "off");
    assert_eq!(meta.day_time, 2 * 24_000 + 7_000);
    assert!(!meta.day_cycle);
}

#[test]
fn coordinates_may_be_relative() {
    assert_eq!(coordinate("~", 10.0), Some(10.0));
    assert_eq!(coordinate("~-2.5", 10.0), Some(7.5));
    assert_eq!(coordinate("64", 10.0), Some(64.0));
    assert_eq!(coordinate("~x", 10.0), None);
    assert_eq!(coordinate("NaN", 10.0), None);
}

#[test]
fn names_and_times_are_understood() {
    assert_eq!(
        identifier("stone").map(|i| i.to_string()).as_deref(),
        Some("strata:stone")
    );
    assert_eq!(preset_time("noon"), Some(Moment::Noon));
    assert_eq!(preset_time("teatime"), None);
}
