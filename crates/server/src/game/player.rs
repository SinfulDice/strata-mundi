//! The player's inputs, look, mode, reach and action rate.

use super::{ACTION_TICKS, Game, MAX_QUEUED_INPUTS, MAX_REACH};
use glam::DVec3;
use strata_core::coords::{BlockPos, WORLD_HEIGHT};
use strata_core::physics::MoveInput;
use strata_protocol::{GameMode, ServerMessage};

impl Game {
    pub fn queue_input(&mut self, sequence: u32, input: MoveInput) {
        let valid = input.yaw.is_finite()
            && input.pitch.is_finite()
            && input.wish.iter().all(|w| w.is_finite());
        if let (true, Some(control)) = (valid, self.ecs.controls.get_mut(self.player))
            && control.pending.len() < MAX_QUEUED_INPUTS
        {
            control.pending.push_back((sequence, input));
        }
    }

    /// Creative only: the player flies at the destination, kept inside the border and the
    /// world height.
    pub fn teleport(&mut self, feet: [f64; 3]) -> bool {
        let border = f64::from(self.border) - 1.0;
        if !feet.iter().all(|c| c.is_finite()) || !self.is_creative() {
            return false;
        }
        self.leave_boat(self.player);
        let Some(body) = self.ecs.bodies.get_mut(self.player) else {
            return false;
        };
        {
            body.position = DVec3::new(
                feet[0].clamp(-border, border),
                feet[1].clamp(0.0, f64::from(WORLD_HEIGHT)),
                feet[2].clamp(-border, border),
            );
            body.velocity = DVec3::ZERO;
            body.flying = true;
        }
        true
    }

    pub fn look(&self) -> [f32; 2] {
        self.ecs
            .controls
            .get(self.player)
            .map_or([0.0; 2], |c| c.look)
    }

    pub fn set_look(&mut self, look: [f32; 2]) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.look = look;
        }
    }

    pub fn set_preferences(&mut self, auto_jump: bool, step_up: bool) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.movement.auto_jump = auto_jump;
            control.movement.step_up = step_up;
        }
    }

    pub fn set_mode(&mut self, mode: GameMode) {
        if let Some(control) = self.ecs.controls.get_mut(self.player) {
            control.mode = mode;
            control.movement.can_fly = mode == GameMode::Creative;
            self.messages.push(ServerMessage::Mode(mode));
            self.recipes_changed();
            if mode != GameMode::Creative
                && let Some(body) = self.ecs.bodies.get_mut(self.player)
            {
                body.flying = false;
            }
        }
    }

    pub fn mode(&self) -> GameMode {
        self.ecs
            .controls
            .get(self.player)
            .map_or(GameMode::Creative, |c| c.mode)
    }

    pub fn set_view_distance(&mut self, distance: u32) {
        self.view_distance = distance;
    }

    pub fn is_creative(&self) -> bool {
        self.ecs
            .controls
            .get(self.player)
            .is_some_and(|c| c.mode == GameMode::Creative)
    }

    /// Rate limit shared by breaking and placing; consumes the action when allowed.
    pub(super) fn take_action(&mut self) -> bool {
        let tick = self.tick;
        self.ecs.controls.get_mut(self.player).is_some_and(|c| {
            let ready = c.last_action.is_none_or(|last| tick >= last + ACTION_TICKS);
            if ready {
                c.last_action = Some(tick);
            }
            ready
        })
    }

    pub(super) fn in_reach(&self, pos: BlockPos) -> bool {
        self.player_body().is_some_and(|body| {
            let center = DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z)) + 0.5;
            body.eyes().distance(center) <= MAX_REACH
        })
    }
}
