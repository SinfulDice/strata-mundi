use crate::TickClock;
use crate::commands::Console;
use crate::game::{BlockChange, Game};
use crate::generation::GenerationWorkers;
use crate::loader::{ColumnLoader, LoadEvent, Viewer};
use crate::persistence::{Persistence, WorldSettings};
use crate::rules::BlockRules;
use crate::save::{BlockNames, RegionStore};
use glam::DVec3;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;
use strata_core::content::Content;
use strata_core::dimension::DimensionId;
use strata_protocol::{
    ClientMessage, MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE, ServerConnection, ServerMessage,
};
use strata_worldgen::Generator;

pub struct ServerConfig {
    pub dimension: DimensionId,
    pub generator: Arc<Generator>,
    pub content: Arc<Content>,
    pub generation_threads: usize,
    pub view_distance: u32,
    pub world: WorldSettings,
}

/// Server running on its own thread in the game process (single player).
pub struct IntegratedServer {
    connection: ServerConnection,
    loader: ColumnLoader,
    game: Game,
    persistence: Persistence,
    dimension: DimensionId,
    view_distance: u32,
    paused: bool,
    tick: u64,
    /// Capture sent with `Disconnect`, written by the final save.
    leaving_icon: Option<Vec<u8>>,
    content: Arc<Content>,
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
        let persistence = Persistence::new(config.world, Arc::clone(&config.content));
        let generator = Arc::clone(&config.generator);
        let (body, look, hotbar) = persistence.player_start(&config.dimension, || {
            let spawn = generator.spawn_point();
            tracing::info!(?spawn, "spawn point chosen");
            DVec3::new(
                f64::from(spawn.x) + 0.5,
                f64::from(spawn.y),
                f64::from(spawn.z) + 0.5,
            )
        });
        let meta = &persistence.meta;
        let mut game = Game::new(
            BlockRules::new(&config.content),
            meta.border,
            body,
            meta.mode,
        );
        game.set_hotbar(hotbar);
        game.set_look(look);
        if let (Some(body), Some(movement)) = (game.player_body(), game.movement()) {
            connection.send(ServerMessage::Welcome {
                dimension: config.dimension.clone(),
                body: *body,
                movement,
                border: game.border,
                hotbar: game.hotbar(),
                look: game.look(),
            });
        }
        let names = Arc::new(BlockNames::new(&config.content.blocks));
        let store = match persistence.regions(&config.dimension) {
            Some(dir) => RegionStore::on_disk(dir, names),
            None => RegionStore::in_memory(names),
        };
        let workers = GenerationWorkers::spawn(config.generation_threads, config.generator)?;
        Ok(Self {
            connection,
            loader: ColumnLoader::new(workers, store),
            game,
            persistence,
            dimension: config.dimension,
            view_distance: config.view_distance,
            paused: false,
            tick: 0,
            leaving_icon: None,
            content: config.content,
        })
    }

    fn run(mut self) {
        tracing::info!("integrated server started");
        let mut clock = TickClock::new(Instant::now());
        while self.handle_messages() {
            for _ in 0..clock.due_ticks(Instant::now()) {
                if !self.paused {
                    self.tick();
                }
            }
            thread::sleep(clock.next_tick().saturating_duration_since(Instant::now()));
        }
        let icon = self.leaving_icon.take();
        self.save(icon.as_deref());
        self.game.remove_player();
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
            ClientMessage::Teleport { feet } => {
                self.game.teleport(feet);
            }
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
            ClientMessage::Hotbar(hotbar) => {
                self.game.set_hotbar(hotbar);
            }
            ClientMessage::Pause(paused) => self.paused = paused,
            ClientMessage::Save { icon } => {
                self.save(icon.as_deref());
                self.connection.send(ServerMessage::Saved);
            }
            ClientMessage::Disconnect { icon } => {
                self.leaving_icon = icon;
                return false;
            }
            ClientMessage::Command(line) => self.run_command(&line),
        }
        true
    }

    fn run_command(&mut self, line: &str) {
        let mut console = Console {
            game: &mut self.game,
            meta: &mut self.persistence.meta,
            content: &self.content,
        };
        let ran = console.run(line);
        tracing::info!(line, answer = %ran.feedback.key, "command");
        if let Some(hotbar) = ran.hotbar {
            self.connection.send(ServerMessage::Hotbar(hotbar));
        }
        self.connection.send(ServerMessage::Feedback(ran.feedback));
    }

    fn save(&mut self, icon: Option<&[u8]>) {
        self.persistence
            .save(&self.game, &mut self.loader, &self.dimension, icon);
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
        if self.persistence.count_tick() {
            self.save(None);
        }
        if let Some(body) = self.game.player_body() {
            self.connection.send(ServerMessage::PlayerState {
                sequence: self.game.last_sequence(),
                body: *body,
            });
            let viewer = Viewer {
                x: body.position.x,
                z: body.position.z,
                yaw: self.game.look()[0],
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
