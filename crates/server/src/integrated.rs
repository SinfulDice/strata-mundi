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
use strata_core::sky::DayCycle;
use strata_protocol::{
    BiomeInfo, ClientMessage, DAY_TIME_PERIOD, DayState, MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE,
    ServerConnection, ServerMessage,
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
        let start = persistence.player_start(&config.dimension, || {
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
            start.body,
            meta.mode,
        );
        game.set_inventory(start.inventory);
        game.set_look(start.look);
        game.set_fluid_settings(meta.fluids);
        game.set_fire_settings(meta.fire);
        game.set_world_seed(meta.seed, &config.dimension);
        game.restore_boats(persistence.boats(&config.dimension), start.boat);
        game.restore_drops(persistence.drops(&config.dimension));
        game.block_entities = persistence.block_entities(&config.dimension);
        if let (Some(body), Some(movement)) = (game.player_body(), game.movement()) {
            connection.send(ServerMessage::Welcome {
                dimension: config.dimension.clone(),
                body: *body,
                movement,
                border: game.border,
                inventory: Box::new(game.inventory()),
                mode: game.mode(),
                look: game.look(),
                day: DayState {
                    time: meta.day_time,
                    flows: meta.day_cycle,
                    cycle: config.content.day_cycle(&config.dimension).def(),
                },
                biomes: biome_infos(&config.generator),
            });
            for message in game.take_messages() {
                connection.send(message);
            }
        }
        let names = Arc::new(BlockNames::new(&config.content.blocks));
        let store = match persistence.regions(&config.dimension) {
            Some(dir) => RegionStore::on_disk(dir, names),
            None => RegionStore::in_memory(names),
        };
        let workers = GenerationWorkers::spawn(config.generation_threads, config.generator)?;
        game.set_view_distance(config.view_distance);
        let server = Self {
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
        };
        server.send_day_time();
        Ok(server)
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
            let running = self.handle(message);
            self.send_game_messages();
            if !running {
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
                self.game.set_view_distance(self.view_distance);
            }
            ClientMessage::MovementPreferences { auto_jump, step_up } => {
                self.game.set_preferences(auto_jump, step_up);
            }
            ClientMessage::Teleport { feet } => {
                self.game.teleport(feet);
            }
            ClientMessage::Action { id, action } => {
                let changes = self.game.request_action(&mut self.loader, id, action);
                self.broadcast_changes(changes);
            }
            ClientMessage::Inventory(request) => {
                let content = &self.content;
                let names = |item, language: &str| content.item_name(item, language);
                self.game.inventory_request(&self.loader, request, &names);
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
        let cycle = self.day_cycle();
        let mut console = Console {
            game: &mut self.game,
            meta: &mut self.persistence.meta,
            content: &self.content,
            cycle,
        };
        let ran = console.run(line);
        tracing::info!(line, answer = %ran.feedback.key, "command");
        self.connection.send(ServerMessage::Feedback(ran.feedback));
        self.send_day_time();
    }

    /// The cycle of the player's dimension, as it is now.
    fn day_cycle(&self) -> DayCycle {
        self.content.day_cycle(&self.dimension)
    }

    fn send_day_time(&self) {
        let meta = &self.persistence.meta;
        self.connection.send(ServerMessage::DayTime {
            time: meta.day_time,
            cycle: meta.day_cycle,
        });
    }

    fn save(&mut self, icon: Option<&[u8]>) {
        self.persistence
            .save(&self.game, &mut self.loader, &self.dimension, icon);
    }

    fn send_game_messages(&mut self) {
        for message in self.game.take_messages() {
            self.connection.send(message);
        }
    }

    fn broadcast_changes(&self, changes: Vec<BlockChange>) {
        for (pos, block) in changes {
            self.connection
                .send(ServerMessage::BlockChanged { pos, block });
        }
    }

    #[tracing::instrument(skip(self), level = "trace")]
    fn tick(&mut self) {
        self.tick += 1;
        let changes = self.game.tick(&mut self.loader);
        self.broadcast_changes(changes);
        self.send_game_messages();
        if self.persistence.count_tick() {
            self.save(None);
        }
        if self.tick.is_multiple_of(DAY_TIME_PERIOD) {
            self.send_day_time();
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
                LoadEvent::Loaded(column, biomes) => ServerMessage::Column {
                    column: Box::new(column),
                    biomes,
                },
                LoadEvent::Unloaded(pos) => ServerMessage::UnloadColumn(pos),
            };
            self.connection.send(message);
        }
    }
}

fn biome_infos(generator: &Generator) -> Vec<BiomeInfo> {
    let biomes = generator.content().biomes().iter();
    biomes
        .map(|biome| BiomeInfo {
            id: biome.id.clone(),
            tints: biome.tints,
        })
        .collect()
}
