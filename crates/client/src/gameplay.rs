//! What the player does in the world: moving (predicted), looking, aiming, building.

use crate::camera::Camera;
use crate::hotbar::Hotbar;
use crate::input::{Action, InputState};
use crate::interaction::{BlockInfo, Interaction};
use crate::look::LookSettings;
use crate::player::LocalPlayer;
use crate::render::Renderer;
use crate::settings::ControlSettings;
use crate::streaming::WorldView;
use crate::world::ClientWorld;
use glam::DVec3;
use strata_core::coords::{BlockPos, DEFAULT_WORLD_BORDER};
use strata_core::physics::{EYE_HEIGHT, MoveInput, solid_at};
use strata_protocol::{ClientConnection, ClientMessage, HotbarState, ServerMessage};

pub struct Gameplay {
    pub view: WorldView,
    pub camera: Camera,
    pub hotbar: Hotbar,
    pub interaction: Interaction,
    pub info: BlockInfo,
    pub controls: ControlSettings,
    player: Option<LocalPlayer>,
    look: LookSettings,
    border: i32,
    /// Last hotbar sent to the server, to send only changes.
    sent_hotbar: HotbarState,
}

impl Gameplay {
    pub fn new(
        view: WorldView,
        camera: Camera,
        info: BlockInfo,
        controls: ControlSettings,
    ) -> Self {
        Self {
            view,
            camera,
            hotbar: Hotbar::default(),
            interaction: Interaction::default(),
            info,
            controls,
            player: None,
            look: LookSettings::default(),
            border: DEFAULT_WORLD_BORDER,
            sent_hotbar: HotbarState::default(),
        }
    }

    pub fn apply(
        &mut self,
        message: ServerMessage,
        renderer: &mut Renderer,
        connection: &ClientConnection,
    ) {
        match message {
            ServerMessage::Welcome {
                body,
                mut movement,
                border,
                hotbar,
                look,
                ..
            } => {
                self.border = border;
                (self.camera.yaw, self.camera.pitch) = (look[0], look[1]);
                self.hotbar.restore(&hotbar);
                self.sent_hotbar = self.hotbar.state();
                movement.auto_jump = self.controls.auto_jump;
                movement.step_up = self.controls.step_up;
                connection.send(ClientMessage::MovementPreferences {
                    auto_jump: movement.auto_jump,
                    step_up: movement.step_up,
                });
                self.player = Some(LocalPlayer::new(body, movement));
            }
            ServerMessage::PlayerState { sequence, body } => {
                let solid = solid_lookup(&self.view.world, &self.info, self.border);
                if let Some(player) = &mut self.player {
                    player.reconcile(sequence, body, &solid);
                }
            }
            ServerMessage::Column(column) => self.view.insert(*column),
            ServerMessage::UnloadColumn(pos) => self.view.remove(pos, renderer),
            ServerMessage::BlockChanged { pos, block } => self.view.set_block(pos, block),
            ServerMessage::Tick { .. } | ServerMessage::Saved => {}
        }
    }

    /// Player-driven frame: look, move (fixed ticks), aim and build.
    pub fn update(&mut self, dt: f32, input: &InputState, connection: &ClientConnection) {
        self.look.apply(&mut self.camera, input, dt);
        self.select_hotbar(input);
        let move_input = MoveInput {
            wish: input.movement().to_array(),
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            jump: input.is_held(Action::Jump),
            sneak: input.is_held(Action::Sneak),
            sprint: input.is_held(Action::Sprint),
            toggle_flight: false,
        };
        let solid = solid_lookup(&self.view.world, &self.info, self.border);
        if let Some(player) = &mut self.player {
            for (sequence, input) in player.advance(f64::from(dt), move_input, &solid) {
                connection.send(ClientMessage::Input { sequence, input });
            }
            self.camera.position = player.eyes();
        }
        let message = self.interaction.update(
            dt,
            &self.camera,
            &self.view.world,
            &self.info,
            input,
            &mut self.hotbar,
        );
        if let Some(message) = message {
            connection.send(message);
        }
    }

    /// Tells the server about hotbar changes (from play or from the inventory).
    pub fn sync_hotbar(&mut self, connection: &ClientConnection) {
        if !self.hotbar.matches(&self.sent_hotbar) {
            self.sent_hotbar = self.hotbar.state();
            connection.send(ClientMessage::Hotbar(self.sent_hotbar.clone()));
        }
    }

    /// Scripted camera (benchmarks): the player is teleported along so the world streams.
    pub fn follow_camera(&mut self, connection: &ClientConnection) {
        let feet = self.camera.position - DVec3::Y * EYE_HEIGHT;
        connection.send(ClientMessage::Teleport {
            feet: feet.to_array(),
        });
    }

    fn select_hotbar(&mut self, input: &InputState) {
        for (index, action) in Action::HOTBAR.into_iter().enumerate() {
            if input.just_pressed(action) {
                self.hotbar.select(index);
            }
        }
        let buttons = i32::from(input.just_pressed(Action::HotbarNext))
            - i32::from(input.just_pressed(Action::HotbarPrevious));
        self.hotbar.scroll(buttons - input.scroll_steps());
    }
}

/// The same solidity rules as the server (`solid_at`), on the client's copy of the world.
fn solid_lookup<'a>(
    world: &'a ClientWorld,
    info: &'a BlockInfo,
    border: i32,
) -> impl Fn(i32, i32, i32) -> bool + 'a {
    move |x, y, z| {
        solid_at(BlockPos::new(x, y, z), border, |pos| {
            world
                .is_loaded(pos)
                .then(|| info.is_solid(world.block_at(pos)))
        })
    }
}
