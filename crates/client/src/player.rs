//! The local player, predicted with the same physics as the server: the client moves at
//! once, the server answers with the authoritative state, and inputs it has not applied
//! yet are replayed on top of it. Rendering interpolates between the last two ticks.

use glam::DVec3;
use std::collections::VecDeque;
use strata_core::physics::{Body, MoveInput, MovementSettings, TICK, step};

/// Two jump presses closer than this toggle flight (creative), like Minecraft.
const DOUBLE_TAP_SECONDS: f64 = 0.3;
/// Unacknowledged inputs kept for replay; beyond that the server is far behind anyway.
const MAX_PENDING: usize = 64;

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
        }
    }

    /// Runs the fixed ticks due after `dt` seconds; returns the inputs to send.
    pub fn advance(
        &mut self,
        dt: f64,
        mut input: MoveInput,
        solid: &impl Fn(i32, i32, i32) -> bool,
    ) -> Vec<(u32, MoveInput)> {
        self.since_tick += dt.min(0.25);
        self.since_jump_press += dt;
        if input.jump && !self.jump_was_held {
            let double_tap = self.since_jump_press < DOUBLE_TAP_SECONDS;
            self.toggle_pending |= double_tap;
            self.since_jump_press = if double_tap { f64::INFINITY } else { 0.0 };
        }
        self.jump_was_held = input.jump;
        let mut sent = Vec::new();
        while self.since_tick >= TICK {
            self.since_tick -= TICK;
            self.sequence = self.sequence.wrapping_add(1);
            input.toggle_flight = std::mem::take(&mut self.toggle_pending);
            self.previous = self.body;
            step(&mut self.body, &input, &self.movement, solid);
            self.pending.push_back((self.sequence, input));
            if self.pending.len() > MAX_PENDING {
                self.pending.pop_front();
            }
            sent.push((self.sequence, input));
        }
        sent
    }

    /// Server state after input `sequence`: adopt it and replay the newer inputs.
    pub fn reconcile(
        &mut self,
        sequence: u32,
        server: Body,
        solid: &impl Fn(i32, i32, i32) -> bool,
    ) {
        while self
            .pending
            .front()
            .is_some_and(|(s, _)| s.wrapping_sub(sequence) as i32 <= 0)
        {
            self.pending.pop_front();
        }
        let mut body = server;
        for (_, input) in &self.pending {
            step(&mut body, input, &self.movement, solid);
        }
        if body.position.distance(self.body.position) > 1e-3 {
            tracing::debug!(
                error = body.position.distance(self.body.position),
                "prediction corrected"
            );
            self.previous = body;
        }
        self.body = body;
    }

    /// Eye position between the last two ticks, for smooth rendering at any frame rate.
    pub fn eyes(&self) -> DVec3 {
        let alpha = (self.since_tick / TICK).clamp(0.0, 1.0);
        self.previous.eyes().lerp(self.body.eyes(), alpha)
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
        client.reconcile(sent[5].0, server, &floor);
        assert!(client.body.position.distance(predicted.position) < 1e-9);
    }

    #[test]
    fn server_corrections_win() {
        let mut client = LocalPlayer::new(Body::at(DVec3::ZERO), settings());
        let sent = client.advance(0.2, forward(), &floor);
        let mut forced = Body::at(DVec3::new(100.0, 0.0, 0.0));
        forced.on_ground = true;
        client.reconcile(sent.last().unwrap().0, forced, &floor);
        assert_eq!(client.body.position, forced.position);
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
