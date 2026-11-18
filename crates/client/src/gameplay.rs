//! What the player does in the world: moving (predicted), looking, aiming, building,
//! riding boats.

use crate::audio::Audio;
use crate::camera::Camera;
use crate::cinematic::PlayingShot;
use crate::crafting::ClientCrafting;
use crate::hammer::{held_gear, outline_box};
use crate::input::{Action, InputState};
use crate::interaction::{BlockInfo, Interaction};
use crate::inventory::ClientInventory;
use crate::look::apply_look;
use crate::player::LocalPlayer;
use crate::prediction::predicted_change;
use crate::render::Renderer;
use crate::settings::ControlSettings;
use crate::sky::DayClock;
use crate::streaming::WorldView;
use glam::DVec3;
use strata_core::cinematic::Cinematic;
use strata_core::coords::{BlockPos, DEFAULT_WORLD_BORDER};
use strata_core::dimension::DimensionId;
use strata_core::physics::{Body, EYE_HEIGHT, Mobility, MoveInput, Terrain, TerrainFns, solid_at};
use strata_core::sky::DayCycle;
use strata_protocol::commands::Feedback;
use strata_protocol::{
    BiomeInfo, ClientConnection, ClientMessage, GameMode, InventoryRequest, OpenedContainer,
    WorldAction,
};

mod messages;

/// Field of view widening while sprinting, and how fast it eases in and out (per second).
const SPRINT_ZOOM: f32 = 1.1;
const ZOOM_SPEED: f32 = 8.0;

pub struct Gameplay {
    pub view: WorldView,
    pub camera: Camera,
    pub inventory: ClientInventory,
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
    /// The container the player opened, as the server last described it.
    pub container: Option<OpenedContainer>,
    /// A container just opened: its screen must come up.
    pub container_opened: bool,
    furnace_clock: crate::furnace::FurnaceClock,
    pub crafting: ClientCrafting,
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
            inventory: ClientInventory::default(),
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
            container: None,
            container_opened: false,
            furnace_clock: Default::default(),
            crafting: ClientCrafting::default(),
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
        self.furnace_clock.follow(self.container.as_mut(), dt);
        if let Some(playing) = &mut self.cinematic {
            if !playing.advance(dt, &mut self.camera) {
                self.stop_cinematic();
            }
            self.cancel_breaking(connection);
            return;
        }
        apply_look(&mut self.camera, input, dt, &self.controls);
        self.select_hotbar(input);
        if input.just_pressed(Action::Drop) {
            let whole = input.is_held(Action::Sprint);
            self.inventory
                .request(InventoryRequest::DropSelected { whole });
        }
        let move_input = MoveInput {
            wish: input.movement().to_array(),
            yaw: self.camera.yaw,
            pitch: self.camera.pitch,
            jump: input.is_held(Action::Jump),
            sneak: input.is_held(Action::Sneak),
            sprint: input.is_held(Action::Sprint),
            toggle_flight: false,
            dash: input.just_pressed(Action::Dash),
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
            &mut self.inventory,
        );
        if let Some(action) = intent {
            self.send_action(action, connection);
        }
    }

    fn advance_player(&mut self, dt: f32, input: MoveInput, connection: &ClientConnection) {
        let terrain = terrain(&self.view, &self.info, self.border, None);
        if let Some(player) = &mut self.player {
            let worn = self.inventory.worn_stats(&self.info.content);
            let movement = &mut player.movement;
            movement.speed = f64::from(worn.burden.speed_factor());
            movement.mobility = if movement.gear_mobility {
                self.info.content.worn.mobility.mobility(&worn)
            } else {
                Mobility::default()
            };
            for (sequence, input) in player.advance(f64::from(dt), input, &terrain) {
                connection.send(ClientMessage::Input { sequence, input });
            }
            self.camera.position = player.eyes();
        }
    }

    /// The aimed block's outline, the cracks of the blocks being broken, and the hits of
    /// this frame (a fragment and a sound each).
    pub fn show_breaking(&mut self, renderer: &mut Renderer, audio: &mut Audio) {
        let overlays = &mut renderer.overlays;
        let world = &self.view.world;
        let content = &self.info.content;
        let gear = held_gear(&self.inventory, content);
        let yaw = self.camera.yaw;
        overlays.outline =
            (self.interaction.target).map(|hit| outline_box(hit, &gear, yaw, world, content));
        let mining = &mut self.interaction.mining;
        overlays.cracks = mining.draws(world, &self.info, self.camera.position);
        for (pos, face) in mining.hits.drain(..) {
            self.view.particles.block_hit(pos, face, world);
            audio.block_hit(pos, world.block_at(pos));
        }
    }

    /// A screen, a menu or another window took over: a break in progress stops.
    pub fn cancel_breaking(&mut self, connection: &ClientConnection) {
        if let Some(stop) = self.interaction.mining.cancel() {
            self.send_action(stop, connection);
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
            let after_input = self.player.as_ref().map_or(0, LocalPlayer::last_input);
            self.view.predict(id, pos, block, after_input);
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

    /// Sends the inventory requests of the frame (from play or from the screens).
    pub fn sync_inventory(&mut self, connection: &ClientConnection) {
        for request in self.inventory.take_requests() {
            connection.send(ClientMessage::Inventory(request));
        }
    }

    pub fn is_creative(&self) -> bool {
        self.mode == GameMode::Creative
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
                self.inventory.select(index);
            }
        }
        let buttons = i32::from(input.just_pressed(Action::HotbarNext))
            - i32::from(input.just_pressed(Action::HotbarPrevious));
        self.inventory.scroll(buttons - input.scroll_steps());
    }
}

/// The same solidity and liquid rules as the server (`solid_at`), on the client's copy
/// of the world; with `before: Some(input)`, as the server had it when it applied that
/// input (the cells predicted by later clicks keep the server's block).
fn terrain<'a>(
    view: &'a WorldView,
    info: &'a BlockInfo,
    border: i32,
    before: Option<u32>,
) -> impl Terrain + 'a {
    let block = move |pos| {
        before
            .and_then(|input| view.predictions.block_before(pos, input))
            .unwrap_or_else(|| view.world.block_at(pos))
    };
    TerrainFns {
        solid: move |x, y, z| {
            solid_at(BlockPos::new(x, y, z), border, |pos| {
                view.world.is_loaded(pos).then(|| info.is_solid(block(pos)))
            })
        },
        liquid: move |x, y, z| info.liquid(block(BlockPos::new(x, y, z))),
    }
}
