//! The local player, predicted with the same physics as the server: the client moves at
//! once, the server answers with the authoritative state, and inputs it has not applied
//! yet are replayed on top of it, each on the terrain the server had for it. Rendering
//! interpolates between the last two ticks; a small correction fades out instead of
//! jumping.

use glam::DVec3;
use std::collections::VecDeque;
use strata_core::physics::{Body, MoveInput, MovementSettings, TICK, Terrain, step};

/// Two jump presses closer than this toggle flight (creative), like Minecraft.
const DOUBLE_TAP_SECONDS: f64 = 0.3;
/// Unacknowledged inputs kept for replay; beyond that the server is far behind anyway.
const MAX_PENDING: usize = 64;
/// Corrections shorter than this (blocks) fade out; longer ones are teleports.
const SNAP_DISTANCE: f64 = 2.0;
/// Time constant of the fading (seconds).
const SMOOTHING_SECONDS: f64 = 0.1;

pub struct LocalPlayer {
    pub body: Body,
    previous: Body,
    pub movement: MovementSettings,
    sequence: u32,
    pending: VecDeque<(u32, MoveInput)>,
    since_tick: f64,
    since_jump_press: f64,
    jump_was_held: bool,
    /// A double tap waiting for the next tick (frames usually outnumber ticks).
    toggle_pending: bool,
    /// A dash press waiting for the next tick.
    dash_pending: bool,
    /// Sitting in a boat: the server moves the body, inputs only steer.
    pub riding: bool,
    /// Drawn offset left by a small correction, fading to zero.
    smoothing: DVec3,
}

impl LocalPlayer {
    pub fn new(body: Body, movement: MovementSettings) -> Self {
        Self {
            body,
            previous: body,
            movement,
            sequence: 0,
            pending: VecDeque::new(),
            since_tick: 0.0,
            since_jump_press: f64::INFINITY,
            jump_was_held: false,
            toggle_pending: false,
            dash_pending: false,
            riding: false,
            smoothing: DVec3::ZERO,
        }
    }

    /// The sequence of the last input sent.
    pub fn last_input(&self) -> u32 {
        self.sequence
    }

    /// Runs the fixed ticks due after `dt` seconds; returns the inputs to send.
    pub fn advance(
        &mut self,
        dt: f64,
        mut input: MoveInput,
        terrain: &impl Terrain,
    ) -> Vec<(u32, MoveInput)> {
        self.since_tick += dt.min(0.25);
        self.since_jump_press += dt;
        self.smoothing *= (-dt / SMOOTHING_SECONDS).exp();
        if input.jump && !self.jump_was_held {
            let double_tap = self.since_jump_press < DOUBLE_TAP_SECONDS;
            self.toggle_pending |= double_tap;
            self.since_jump_press = if double_tap { f64::INFINITY } else { 0.0 };
        }
        self.jump_was_held = input.jump;
        self.dash_pending |= input.dash;
        let mut sent = Vec::new();
        while self.since_tick >= TICK {
            self.since_tick -= TICK;
            self.sequence = self.sequence.wrapping_add(1);
            input.toggle_flight = std::mem::take(&mut self.toggle_pending);
            input.dash = std::mem::take(&mut self.dash_pending);
            self.previous = self.body;
            if !self.riding {
                step(&mut self.body, &input, &self.movement, terrain);
            }
            self.pending.push_back((self.sequence, input));
            if self.pending.len() > MAX_PENDING {
                self.pending.pop_front();
            }
            sent.push((self.sequence, input));
        }
        sent
    }

    /// Server state after input `sequence`: adopt it and replay the newer inputs, each on
    /// `terrain_at(input)`.
    pub fn reconcile<T: Terrain>(
        &mut self,
        sequence: u32,
        server: Body,
        terrain_at: impl Fn(u32) -> T,
    ) {
        while self
            .pending
            .front()
            .is_some_and(|(s, _)| s.wrapping_sub(sequence) as i32 <= 0)
        {
            self.pending.pop_front();
        }
        let mut body = server;
        for (input_sequence, input) in self.pending.iter().filter(|_| !self.riding) {
            step(
                &mut body,
                input,
                &self.movement,
                &terrain_at(*input_sequence),
            );
        }
        self.correct(body);
    }

    /// Takes the replayed body; a small gap is drawn away gradually, a large one at once.
    fn correct(&mut self, body: Body) {
        let shift = body.position - self.body.position;
        let error = shift.length();
        if error > 1e-3 {
            tracing::debug!(error, "prediction corrected");
            if error < SNAP_DISTANCE {
                self.previous.position += shift;
                self.smoothing -= shift;
            } else {
                self.previous = body;
                self.smoothing = DVec3::ZERO;
            }
        }
        self.body = body;
    }

    /// Eye position between the last two ticks, for smooth rendering at any frame rate.
    pub fn eyes(&self) -> DVec3 {
        let alpha = (self.since_tick / TICK).clamp(0.0, 1.0);
        self.previous.eyes().lerp(self.body.eyes(), alpha) + self.smoothing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor(_: i32, y: i32, _: i32) -> bool {
        y < 0
    }

    fn settings() -> MovementSettings {
        MovementSettings {
            can_fly: true,
            step_up: true,
            auto_jump: false,
            speed: 1.0,
            ..MovementSettings::default()
        }
    }

    fn forward() -> MoveInput {
        MoveInput {
            wish: [0.0, 1.0],
            ..MoveInput::default()
        }
    }

    #[test]
    fn prediction_matches_a_server_running_the_same_inputs() {
        let start = Body::at(DVec3::new(0.5, 0.0, 0.5));
        let mut client = LocalPlayer::new(start, settings());
        let mut sent = client.advance(0.25, forward(), &floor);
        sent.extend(client.advance(0.25, forward(), &floor));
        assert_eq!(sent.len(), 10);
        let mut server = start;
        for (_, input) in &sent[..6] {
            step(&mut server, input, &settings(), &floor);
        }
        let predicted = client.body;
        client.reconcile(sent[5].0, server, |_| floor);
        assert!(client.body.position.distance(predicted.position) < 1e-9);
    }

    #[test]
    fn server_corrections_win() {
        let mut client = LocalPlayer::new(Body::at(DVec3::ZERO), settings());
        let sent = client.advance(0.2, forward(), &floor);
        let mut forced = Body::at(DVec3::new(100.0, 0.0, 0.0));
        forced.on_ground = true;
        client.reconcile(sent.last().unwrap().0, forced, |_| floor);
        assert_eq!(client.body.position, forced.position);
    }

    #[test]
    fn replayed_inputs_see_the_block_before_the_click() {
        let start = Body::at(DVec3::new(0.5, 0.0, 0.5));
        let hole = |x: i32, y: i32, z: i32| y < 0 && (x, y, z) != (0, -1, 0);
        let mut client = LocalPlayer::new(start, settings());
        let before = client.advance(0.1, MoveInput::default(), &floor);
        let click = client.last_input();
        client.advance(0.1, MoveInput::default(), &hole);
        let predicted = client.body;
        assert!(predicted.position.y < 0.0, "falls into the hole");
        let mut server = start;
        step(&mut server, &before[0].1, &settings(), &floor);
        let terrain_at = |input: u32| {
            move |x, y, z| match input <= click {
                true => floor(x, y, z),
                false => hole(x, y, z),
            }
        };
        client.reconcile(before[0].0, server, terrain_at);
        assert!(client.body.position.distance(predicted.position) < 1e-9);
    }

    #[test]
    fn small_corrections_fade_out_large_ones_snap() {
        let mut client = LocalPlayer::new(Body::at(DVec3::ZERO), settings());
        let sent = client.advance(0.05, MoveInput::default(), &floor);
        let drawn = client.eyes();
        let mut server = client.body;
        server.position.x += 0.5;
        client.reconcile(sent[0].0, server, |_| floor);
        assert!(client.eyes().distance(drawn) < 1e-9, "no jump on screen");
        client.advance(0.5, MoveInput::default(), &floor);
        assert!(
            client.eyes().distance(client.body.eyes()) < 0.01,
            "faded out"
        );
        server = client.body;
        server.position.x += 50.0;
        client.reconcile(client.last_input(), server, |_| floor);
        assert_eq!(client.eyes(), server.eyes(), "a teleport is not smoothed");
    }

    #[test]
    fn a_rider_follows_the_server_without_walking() {
        let mut client = LocalPlayer::new(Body::at(DVec3::ZERO), settings());
        client.riding = true;
        let sent = client.advance(0.2, forward(), &floor);
        assert_eq!(client.body.position, DVec3::ZERO, "inputs only steer");
        let seat = Body::at(DVec3::new(3.0, 1.0, 0.0));
        client.reconcile(sent[0].0, seat, |_| floor);
        assert_eq!(
            client.body.position, seat.position,
            "no replay while riding"
        );
    }

    #[test]
    fn double_jump_toggles_flight() {
        let mut client = LocalPlayer::new(Body::at(DVec3::ZERO), settings());
        let jump = MoveInput {
            jump: true,
            ..MoveInput::default()
        };
        client.advance(0.05, jump, &floor);
        client.advance(0.05, MoveInput::default(), &floor);
        let sent = client.advance(0.05, jump, &floor);
        assert!(sent.iter().any(|(_, i)| i.toggle_flight));
        assert!(client.body.flying);
    }

    #[test]
    fn double_jump_survives_frames_without_a_tick() {
        let mut client = LocalPlayer::new(Body::at(DVec3::ZERO), settings());
        let jump = MoveInput {
            jump: true,
            ..MoveInput::default()
        };
        let frame = 1.0 / 144.0;
        let mut sent = Vec::new();
        for held in [
            true, true, false, false, true, false, false, false, false, false, false,
        ] {
            let input = if held { jump } else { MoveInput::default() };
            sent.extend(client.advance(frame, input, &floor));
        }
        assert!(sent.iter().any(|(_, i)| i.toggle_flight));
    }
}
