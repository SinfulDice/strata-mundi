//! What the player does in the world: moving (predicted), looking, aiming, building,
//! riding boats.

use crate::camera::Camera;
use crate::cinematic::PlayingShot;
use crate::hotbar::Hotbar;
use crate::input::{Action, InputState};
use crate::interaction::{BlockInfo, Interaction};
use crate::look::apply_look;
use crate::player::LocalPlayer;
use crate::prediction::predicted_change;
use crate::render::Renderer;
use crate::settings::ControlSettings;
use crate::sky::DayClock;
use crate::streaming::WorldView;
use crate::world::ClientWorld;
use glam::DVec3;
use strata_core::cinematic::Cinematic;
use strata_core::coords::{BlockPos, DEFAULT_WORLD_BORDER};
use strata_core::dimension::DimensionId;
use strata_core::physics::{Body, EYE_HEIGHT, MoveInput, Terrain, TerrainFns, solid_at};
use strata_core::sky::DayCycle;
use strata_protocol::commands::Feedback;
use strata_protocol::{
    BiomeInfo, ClientConnection, ClientMessage, GameMode, HotbarState, ServerMessage, WorldAction,
};

/// Field of view widening while sprinting, and how fast it eases in and out (per second).
const SPRINT_ZOOM: f32 = 1.1;
const ZOOM_SPEED: f32 = 8.0;

pub struct Gameplay {
    pub view: WorldView,
    pub camera: Camera,
    pub hotbar: Hotbar,
    pub interaction: Interaction,
    pub info: BlockInfo,
    pub controls: ControlSettings,
    /// Off with "reduce camera effects".
    pub camera_effects: bool,
    /// A cinematic has the camera; the player stands still meanwhile.
    pub cinematic: Option<PlayingShot>,
    /// Answers to console commands, not shown yet.
    pub feedback: Vec<Feedback>,
    /// Where the player is, from the welcome message.
    pub dimension: Option<DimensionId>,
    /// The world time, once the server has given it.
    pub day: Option<DayClock>,
    /// The server's day cycle (not the client's packs), from the welcome message.
    pub day_cycle: Option<DayCycle>,
    /// The server's biomes, indexed by `BiomeId`.
    pub biomes: Vec<BiomeInfo>,
    /// Told by the server (creative breaks anything).
    mode: GameMode,
    player: Option<LocalPlayer>,
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
            camera_effects: true,
            cinematic: None,
            feedback: Vec::new(),
            dimension: None,
            day: None,
            day_cycle: None,
            biomes: Vec::new(),
            mode: GameMode::Creative,
            player: None,
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
                dimension,
                body,
                mut movement,
                border,
                hotbar,
                mode,
                look,
                day,
                biomes,
            } => {
                self.mode = mode;
                let tints = biomes.iter().map(|biome| biome.tints).collect();
                self.view.world.set_biome_tints(tints);
                self.biomes = biomes;
                self.dimension = Some(dimension);
                self.day = Some(DayClock::at(day.time, day.flows));
                self.day_cycle = Some(DayCycle::new(&day.cycle));
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
                let terrain = terrain(&self.view.world, &self.info, self.border);
                if let Some(player) = &mut self.player {
                    player.reconcile(sequence, body, &terrain);
                }
            }
            ServerMessage::Column { column, biomes } => self.view.insert(*column, biomes),
            ServerMessage::UnloadColumn(pos) => self.view.remove(pos, renderer),
            ServerMessage::BlockChanged { pos, block } => self.view.receive_block(pos, block),
            ServerMessage::ActionDone(id) => self.view.acknowledge(id),
            ServerMessage::Hotbar(hotbar) => {
                self.hotbar.restore(&hotbar);
                self.sent_hotbar = hotbar;
            }
            ServerMessage::Feedback(feedback) => self.feedback.push(feedback),
            ServerMessage::EntitySpawned(state) => self.view.world.entities.spawn(state),
            ServerMessage::EntityMoved { id, position, yaw } => {
                self.view.world.entities.moved(id, position, yaw);
            }
            ServerMessage::EntityRemoved(id) => self.view.world.entities.remove(id),
            ServerMessage::Mode(mode) => self.mode = mode,
            ServerMessage::Mounted(mount) => {
                self.interaction.mount = mount;
                if let Some(player) = &mut self.player {
                    player.riding = mount.is_some();
                }
            }
            ServerMessage::DayTime { time, cycle } => match &mut self.day {
                Some(day) => day.receive(time, cycle),
                None => self.day = Some(DayClock::at(time, cycle)),
            },
            ServerMessage::Tick { .. } | ServerMessage::Saved => {}
        }
    }

    /// Starts `shot` around the player's eyes.
    pub fn play(&mut self, shot: Cinematic) {
        let origin = self.camera.position;
        self.interaction.target = None;
        self.cinematic = Some(PlayingShot::new(shot, origin, &self.camera));
    }

    pub fn stop_cinematic(&mut self) {
        if let Some(playing) = self.cinematic.take() {
            playing.restore(&mut self.camera);
        }
    }

    /// Player-driven frame: look, move (fixed ticks), aim and build.
    pub fn update(&mut self, dt: f32, input: &InputState, connection: &ClientConnection) {
        if let Some(playing) = &mut self.cinematic {
            if !playing.advance(dt, &mut self.camera) {
                self.stop_cinematic();
            }
            return;
        }
        apply_look(&mut self.camera, input, dt, &self.controls);
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
        self.ease_sprint_zoom(dt, move_input.sprint && move_input.wish != [0.0; 2]);
        self.advance_player(dt, move_input, connection);
        if let Some(eyes) = self.riding_eyes() {
            self.camera.position = eyes;
        }
        let intent = self.interaction.update(
            dt,
            &self.camera,
            &self.view.world,
            &self.info,
            input,
            &mut self.hotbar,
        );
        if let Some(action) = intent {
            self.send_action(action, connection);
        }
    }

    fn advance_player(&mut self, dt: f32, input: MoveInput, connection: &ClientConnection) {
        let terrain = terrain(&self.view.world, &self.info, self.border);
        if let Some(player) = &mut self.player {
            for (sequence, input) in player.advance(f64::from(dt), input, &terrain) {
                connection.send(ClientMessage::Input { sequence, input });
            }
            self.camera.position = player.eyes();
        }
    }

    /// Numbers the click, shows what it should change, and sends it.
    fn send_action(&mut self, action: WorldAction, connection: &ClientConnection) {
        let id = self.view.predictions.next_id();
        let body = self.player_body();
        let creative = self.mode == GameMode::Creative;
        let change = self.view.predictions.may_predict(id).then(|| {
            predicted_change(
                &self.info,
                &self.view.world,
                action,
                body.as_ref(),
                creative,
            )
        });
        if let Some((pos, block)) = change.flatten() {
            self.view.predict(id, pos, block);
        }
        connection.send(ClientMessage::Action { id, action });
    }

    /// Sitting in a boat, the eyes follow the boat as drawn, so both move as one.
    fn riding_eyes(&self) -> Option<DVec3> {
        let mount = self.interaction.mount?;
        let entities = &self.view.world.entities;
        let pose = entities.pose(mount)?;
        let seat = self
            .info
            .content
            .entities
            .get(entities.kind(mount)?)?
            .boat?
            .seat;
        Some(pose.position + DVec3::Y * (seat + EYE_HEIGHT))
    }

    /// 0 at night, 1 by day (by day until the server gave the time).
    pub fn daylight(&self) -> f32 {
        match (&self.day_cycle, &self.day) {
            (Some(cycle), Some(day)) => cycle.daylight(day.time()),
            _ => 1.0,
        }
    }

    /// The biome under the camera, as the server named it.
    pub fn biome_at_camera(&self) -> Option<&BiomeInfo> {
        let position = self.camera.position;
        let (x, z) = (position.x.floor() as i32, position.z.floor() as i32);
        let id = self.view.world.biome_at(x, z)?;
        self.biomes.get(usize::from(id.0))
    }

    /// The local player's body as predicted, unless sitting in a boat.
    pub fn player_body(&self) -> Option<Body> {
        self.player
            .as_ref()
            .filter(|_| !self.is_riding())
            .map(|player| player.body)
    }

    /// Whether the player sits in a boat (the HUD tells how to get off).
    pub fn is_riding(&self) -> bool {
        self.interaction.mount.is_some()
    }

    fn ease_sprint_zoom(&mut self, dt: f32, sprinting: bool) {
        let target = if sprinting && self.camera_effects {
            SPRINT_ZOOM
        } else {
            1.0
        };
        let blend = (dt * ZOOM_SPEED).min(1.0);
        self.camera.zoom += (target - self.camera.zoom) * blend;
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

/// The same solidity and liquid rules as the server (`solid_at`), on the client's copy
/// of the world.
fn terrain<'a>(world: &'a ClientWorld, info: &'a BlockInfo, border: i32) -> impl Terrain + 'a {
    TerrainFns {
        solid: move |x, y, z| {
            solid_at(BlockPos::new(x, y, z), border, |pos| {
                world
                    .is_loaded(pos)
                    .then(|| info.is_solid(world.block_at(pos)))
            })
        },
        liquid: move |x, y, z| info.liquid(world.block_at(BlockPos::new(x, y, z))),
    }
}
