use super::*;
use crate::physics::{MovementSettings, step};

fn settings(mobility: Mobility) -> MovementSettings {
    MovementSettings {
        auto_jump: false,
        mobility,
        ..MovementSettings::default()
    }
}

/// Flat floor at y = 0 and a high wall at z = -3 (north).
fn world(_x: i32, y: i32, z: i32) -> bool {
    y < 0 || z == -3
}

fn input(jump: bool, forward: bool, dash: bool) -> MoveInput {
    MoveInput {
        wish: [0.0, if forward { 1.0 } else { 0.0 }],
        jump,
        dash,
        ..MoveInput::default()
    }
}

fn run(body: &mut Body, input: MoveInput, settings: &MovementSettings, ticks: u32) {
    for _ in 0..ticks {
        step(body, &input, settings, &world);
    }
}

fn highest_after_jumps(mobility: Mobility) -> f64 {
    let settings = settings(mobility);
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    run(&mut body, input(false, false, false), &settings, 2);
    run(&mut body, input(true, false, false), &settings, 1);
    run(&mut body, input(false, false, false), &settings, 6);
    run(&mut body, input(true, false, false), &settings, 1);
    let mut top: f64 = 0.0;
    for _ in 0..30 {
        run(&mut body, input(false, false, false), &settings, 1);
        top = top.max(body.position.y);
    }
    top
}

#[test]
fn a_second_press_in_the_air_jumps_again_only_with_air_jumps() {
    let plain = highest_after_jumps(Mobility::default());
    let double = highest_after_jumps(Mobility {
        air_jumps: 1,
        air_jump_speed: 7.6,
        ..Mobility::default()
    });
    assert!(plain < 1.3, "one jump: {plain}");
    assert!(double > plain + 0.5, "double jump: {double} vs {plain}");
}

#[test]
fn holding_jump_does_not_spend_the_air_jump() {
    let settings = settings(Mobility {
        air_jumps: 1,
        air_jump_speed: 7.6,
        ..Mobility::default()
    });
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    run(&mut body, input(false, false, false), &settings, 2);
    run(&mut body, input(true, false, false), &settings, 8);
    assert_eq!(body.abilities.air_jumps_used, 0);
}

fn dash() -> Mobility {
    Mobility {
        dash: Some(Dash {
            speed: 14.0,
            ticks: 4,
            cooldown: 30,
        }),
        ..Mobility::default()
    }
}

#[test]
fn a_dash_goes_further_than_walking_then_cools_down() {
    let walk_settings = settings(Mobility::default());
    let mut walker = Body::at(DVec3::new(0.5, 0.0, 10.5));
    run(&mut walker, input(false, true, true), &walk_settings, 6);
    let settings = settings(dash());
    let mut dasher = Body::at(DVec3::new(0.5, 0.0, 10.5));
    run(&mut dasher, input(false, true, true), &settings, 6);
    assert!(
        dasher.position.z < walker.position.z - 1.5,
        "dash goes north"
    );
    assert!(dasher.abilities.dash_cooldown > 0);
    let before = dasher.abilities.dash_cooldown;
    run(&mut dasher, input(false, false, true), &settings, 1);
    assert_eq!(
        dasher.abilities.dash_left, 0,
        "no new dash while cooling down"
    );
    assert_eq!(dasher.abilities.dash_cooldown, before - 1);
}

#[test]
fn walking_into_a_wall_climbs_it_only_with_climbing() {
    let mut walker = Body::at(DVec3::new(0.5, 0.0, -1.5));
    run(
        &mut walker,
        input(false, true, false),
        &settings(Mobility::default()),
        20,
    );
    assert!(walker.position.y < 0.01);
    let climbing = settings(Mobility {
        climb: Some(2.4),
        ..Mobility::default()
    });
    let mut climber = Body::at(DVec3::new(0.5, 0.0, -1.5));
    run(&mut climber, input(false, true, false), &climbing, 20);
    assert!(
        climber.position.y > 1.5,
        "climbed to {}",
        climber.position.y
    );
    let height = climber.position.y;
    let hold = MoveInput {
        sneak: true,
        ..input(false, true, false)
    };
    run(&mut climber, hold, &climbing, 10);
    assert!(
        (climber.position.y - height).abs() < 0.2,
        "sneaking holds on"
    );
}

#[test]
fn a_dash_without_movement_goes_where_the_body_looks() {
    let settings = settings(dash());
    for (yaw, along) in [
        (0.0_f32, DVec3::NEG_Z),
        (std::f32::consts::FRAC_PI_2, DVec3::X),
    ] {
        let mut body = Body::at(DVec3::new(0.5, 0.0, 10.5));
        let input = MoveInput {
            yaw,
            ..input(false, false, true)
        };
        run(&mut body, input, &settings, 3);
        let moved = body.position - DVec3::new(0.5, 0.0, 10.5);
        assert!(moved.dot(along) > 1.0, "yaw {yaw}: {moved}");
        assert!((moved - along * moved.dot(along)).length() < 1e-6);
    }
}

#[test]
fn climbing_does_not_give_the_air_jumps_back() {
    let climbing = settings(Mobility {
        air_jumps: 1,
        air_jump_speed: 7.6,
        climb: Some(2.4),
        ..Mobility::default()
    });
    let mut body = Body::at(DVec3::new(0.5, 0.0, -1.5));
    run(&mut body, input(false, false, false), &climbing, 2);
    run(&mut body, input(true, false, false), &climbing, 1);
    run(&mut body, input(false, false, false), &climbing, 4);
    run(&mut body, input(true, false, false), &climbing, 1);
    assert_eq!(body.abilities.air_jumps_used, 1);
    run(&mut body, input(false, true, false), &climbing, 5);
    assert!(!body.on_ground);
    assert_eq!(
        body.abilities.air_jumps_used, 1,
        "only the ground gives them back"
    );
}

#[test]
fn swimming_or_flying_ends_a_dash() {
    let mut body = Body::at(DVec3::new(0.5, 0.0, 10.5));
    body.abilities.dash_left = 3;
    body.flying = true;
    let settings = MovementSettings {
        can_fly: true,
        ..settings(dash())
    };
    run(&mut body, input(false, false, false), &settings, 1);
    assert_eq!(body.abilities.dash_left, 0);
}
