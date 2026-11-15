use super::*;

fn settings() -> MovementSettings {
    MovementSettings {
        can_fly: true,
        step_up: true,
        auto_jump: false,
        speed: 1.0,
        ..MovementSettings::default()
    }
}

/// Flat floor at y = 0 and a wall at x = 5.
fn world(x: i32, y: i32, _z: i32) -> bool {
    y < 0 || x == 5
}

fn run(body: &mut Body, input: MoveInput, ticks: u32) {
    for _ in 0..ticks {
        step(body, &input, &settings(), &world);
    }
}

#[test]
fn falls_and_lands_on_the_floor() {
    let mut body = Body::at(DVec3::new(0.5, 10.0, 0.5));
    run(&mut body, MoveInput::default(), 60);
    assert!(body.on_ground);
    assert!(body.position.y.abs() < 1e-6, "{}", body.position.y);
}

#[test]
fn never_goes_through_a_wall_even_very_fast() {
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    body.velocity.x = 500.0;
    body.flying = true;
    step(&mut body, &MoveInput::default(), &settings(), &world);
    assert!(
        body.position.x <= 5.0 - WIDTH / 2.0 + 1e-6,
        "{}",
        body.position.x
    );
    assert_eq!(body.velocity.x, 0.0);
}

#[test]
fn walking_east_stops_at_the_wall() {
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    let east = MoveInput {
        wish: [0.0, 1.0],
        yaw: std::f32::consts::FRAC_PI_2,
        ..MoveInput::default()
    };
    run(&mut body, east, 100);
    assert!((body.position.x - (5.0 - WIDTH / 2.0)).abs() < 1e-6);
}

fn walk(speed: f64) -> f64 {
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    let north = MoveInput {
        wish: [0.0, 1.0],
        ..MoveInput::default()
    };
    let settings = MovementSettings {
        speed,
        ..settings()
    };
    for _ in 0..40 {
        step(&mut body, &north, &settings, &|x, y, _| y < 0 || x == 50);
    }
    0.5 - body.position.z
}

#[test]
fn a_slower_body_walks_less_far() {
    let ratio = walk(0.9) / walk(1.0);
    assert!((ratio - 0.9).abs() < 0.01, "{ratio}");
}

#[test]
fn the_speed_factor_is_bounded() {
    assert_eq!(walk(10.0), walk(MAX_SPEED_FACTOR));
    assert_eq!(walk(-1.0), 0.0);
}

#[test]
fn jumps_about_a_block_and_a_quarter() {
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    run(&mut body, MoveInput::default(), 2);
    let jump = MoveInput {
        jump: true,
        ..MoveInput::default()
    };
    step(&mut body, &jump, &settings(), &world);
    let mut highest: f64 = 0.0;
    for _ in 0..20 {
        step(&mut body, &MoveInput::default(), &settings(), &world);
        highest = highest.max(body.position.y);
    }
    assert!((1.1..1.4).contains(&highest), "{highest}");
}

#[test]
fn sneaking_does_not_fall_off_an_edge() {
    let ledge = |x: i32, y: i32, _z: i32| y < 0 && x < 1;
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    let input = MoveInput {
        wish: [0.0, 1.0],
        yaw: std::f32::consts::FRAC_PI_2,
        sneak: true,
        ..MoveInput::default()
    };
    for _ in 0..100 {
        step(&mut body, &input, &settings(), &ledge);
    }
    assert!(body.on_ground && body.position.y.abs() < 1e-6);
    assert!(body.position.x < 1.0 + WIDTH / 2.0);
}

#[test]
fn auto_jump_climbs_one_block() {
    let step_world = |x: i32, y: i32, _z: i32| y < 0 || (x >= 3 && y < 1);
    let mut auto = settings();
    auto.auto_jump = true;
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    let east = MoveInput {
        wish: [0.0, 1.0],
        yaw: std::f32::consts::FRAC_PI_2,
        ..MoveInput::default()
    };
    for _ in 0..60 {
        step(&mut body, &east, &auto, &step_world);
    }
    assert!(
        body.position.x > 4.0 && (body.position.y - 1.0).abs() < 1e-6,
        "{:?}",
        body.position
    );
}

#[test]
fn auto_jump_ignores_walls_and_sneaking() {
    let wall = |x: i32, y: i32, _z: i32| y < 0 || (x >= 3 && y < 2);
    let mut auto = settings();
    auto.auto_jump = true;
    let east = MoveInput {
        wish: [0.0, 1.0],
        yaw: std::f32::consts::FRAC_PI_2,
        ..MoveInput::default()
    };
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    for _ in 0..40 {
        step(&mut body, &east, &auto, &wall);
        assert!(body.position.y < 0.1, "jumped against a wall");
    }
    let step_world = |x: i32, y: i32, _z: i32| y < 0 || (x >= 3 && y < 1);
    let sneaking = MoveInput {
        sneak: true,
        ..east
    };
    let mut body = Body::at(DVec3::new(0.5, 0.0, 0.5));
    for _ in 0..80 {
        step(&mut body, &sneaking, &auto, &step_world);
    }
    assert!(body.position.y < 0.1);
}

#[test]
fn landing_ends_flight() {
    let mut body = Body::at(DVec3::new(0.5, 3.0, 0.5));
    body.flying = true;
    run(
        &mut body,
        MoveInput {
            sneak: true,
            ..MoveInput::default()
        },
        40,
    );
    assert!(!body.flying && body.on_ground);
}
