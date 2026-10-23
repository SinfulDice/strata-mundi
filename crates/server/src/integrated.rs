use crate::TickClock;
use crate::game::{BlockChange, Game};
use crate::generation::GenerationWorkers;
use crate::loader::{ColumnLoader, LoadEvent, Viewer};
use crate::rules::BlockRules;
use glam::DVec3;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;
use strata_core::dimension::DimensionId;
use strata_core::physics::Body;
use strata_protocol::{
    ClientMessage, GameMode, MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE, ServerConnection, ServerMessage,
};
use strata_worldgen::Generator;

pub struct ServerConfig {
    pub dimension: DimensionId,
    pub generator: Arc<Generator>,
    pub rules: BlockRules,
    pub generation_threads: usize,
    pub view_distance: u32,
    pub border: i32,
    pub mode: GameMode,
}

/// Server running on its own thread in the game process (single player).
pub struct IntegratedServer {
    connection: ServerConnection,
    loader: ColumnLoader,
    game: Game,
    view_distance: u32,
    tick: u64,
}

impl IntegratedServer {
    pub fn spawn(
        connection: ServerConnection,
        config: ServerConfig,
    ) -> std::io::Result<JoinHandle<()>> {
        thread::Builder::new().name("server".into()).spawn(move || {
            match Self::start(connection, config) {
                Ok(server) => server.run(),
                Err(error) => tracing::error!(%error, "server could not start"),
            }
        })
    }

    fn start(connection: ServerConnection, config: ServerConfig) -> std::io::Result<Self> {
        let spawn = config.generator.spawn_point();
        tracing::info!(?spawn, dimension = %config.dimension, "spawn point chosen");
        let feet = DVec3::new(
            f64::from(spawn.x) + 0.5,
            f64::from(spawn.y),
            f64::from(spawn.z) + 0.5,
        );
        let game = Game::new(config.rules, config.border, Body::at(feet), config.mode);
        if let (Some(body), Some(movement)) = (game.player_body(), game.movement()) {
            connection.send(ServerMessage::Welcome {
                dimension: config.dimension,
                body: *body,
                movement,
                border: game.border,
            });
        }
        let workers = GenerationWorkers::spawn(config.generation_threads, config.generator)?;
        Ok(Self {
            connection,
            loader: ColumnLoader::new(workers),
            game,
            view_distance: config.view_distance,
            tick: 0,
        })
    }

    fn run(mut self) {
        tracing::info!("integrated server started");
        let mut clock = TickClock::new(Instant::now());
        while self.handle_messages() {
            for _ in 0..clock.due_ticks(Instant::now()) {
                self.tick();
            }
            thread::sleep(clock.next_tick().saturating_duration_since(Instant::now()));
        }
        tracing::info!(tick = self.tick, "integrated server stopped");
    }

    /// Returns `false` when the client is gone.
    fn handle_messages(&mut self) -> bool {
        while let Ok(received) = self.connection.try_recv() {
            let Some(message) = received else {
                return true;
            };
            if !self.handle(message) {
                return false;
            }
        }
        false
    }

    fn handle(&mut self, message: ClientMessage) -> bool {
        match message {
            ClientMessage::Input { sequence, input } => self.game.queue_input(sequence, input),
            ClientMessage::ViewDistance(distance) => {
                self.view_distance = distance.clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE);
            }
            ClientMessage::MovementPreferences { auto_jump, step_up } => {
                self.game.set_preferences(auto_jump, step_up);
            }
            ClientMessage::Teleport { feet } => self.game.teleport(feet),
            ClientMessage::BreakBlock { pos } => {
                let change = self.game.break_block(&mut self.loader, pos);
                self.broadcast_change(change);
            }
            ClientMessage::PlaceBlock {
                against,
                face,
                block,
            } => {
                let change = self
                    .game
                    .place_block(&mut self.loader, against, face, block);
                self.broadcast_change(change);
            }
            ClientMessage::Disconnect => {
                self.game.remove_player();
                return false;
            }
        }
        true
    }

    fn broadcast_change(&self, change: Option<BlockChange>) {
        if let Some((pos, block)) = change {
            self.connection
                .send(ServerMessage::BlockChanged { pos, block });
        }
    }

    #[tracing::instrument(skip(self), level = "trace")]
    fn tick(&mut self) {
        self.tick += 1;
        self.game.tick(&self.loader);
        if let Some(body) = self.game.player_body() {
            self.connection.send(ServerMessage::PlayerState {
                sequence: self.game.last_sequence(),
                body: *body,
            });
            let viewer = Viewer {
                x: body.position.x,
                z: body.position.z,
                yaw: 0.0,
                view_distance: self.view_distance,
            };
            self.stream_columns(&viewer);
        }
        self.connection
            .send(ServerMessage::Tick { tick: self.tick });
    }

    fn stream_columns(&mut self, viewer: &Viewer) {
        for event in self.loader.update(viewer) {
            let message = match event {
                LoadEvent::Loaded(column) => ServerMessage::Column(Box::new(column)),
                LoadEvent::Unloaded(pos) => ServerMessage::UnloadColumn(pos),
            };
            self.connection.send(message);
        }
    }
}
