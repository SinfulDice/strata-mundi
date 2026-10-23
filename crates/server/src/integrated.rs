use crate::TickClock;
use crate::generation::GenerationWorkers;
use crate::loader::{ColumnLoader, LoadEvent, Viewer};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::Instant;
use strata_core::coords::DEFAULT_WORLD_BORDER;
use strata_core::dimension::DimensionId;
use strata_protocol::{
    ClientMessage, MAX_VIEW_DISTANCE, MIN_VIEW_DISTANCE, ServerConnection, ServerMessage,
};
use strata_worldgen::Generator;

pub struct ServerConfig {
    pub dimension: DimensionId,
    pub generator: Arc<Generator>,
    pub generation_threads: usize,
    pub view_distance: u32,
}

/// Server running on its own thread in the game process (single player).
pub struct IntegratedServer {
    connection: ServerConnection,
    loader: ColumnLoader,
    viewer: Viewer,
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
        let position = [
            f64::from(spawn.x) + 0.5,
            f64::from(spawn.y),
            f64::from(spawn.z) + 0.5,
        ];
        connection.send(ServerMessage::Welcome {
            dimension: config.dimension,
            spawn: position,
        });
        let workers = GenerationWorkers::spawn(config.generation_threads, config.generator)?;
        Ok(Self {
            connection,
            loader: ColumnLoader::new(workers),
            viewer: Viewer {
                x: position[0],
                z: position[2],
                yaw: 0.0,
                view_distance: config.view_distance,
            },
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
            ClientMessage::PlayerView { position, yaw, .. } => {
                let border = f64::from(DEFAULT_WORLD_BORDER);
                if position.iter().all(|c| c.is_finite()) && yaw.is_finite() {
                    self.viewer.x = position[0].clamp(-border, border);
                    self.viewer.z = position[2].clamp(-border, border);
                    self.viewer.yaw = yaw;
                }
            }
            ClientMessage::ViewDistance(distance) => {
                self.viewer.view_distance = distance.clamp(MIN_VIEW_DISTANCE, MAX_VIEW_DISTANCE);
            }
            ClientMessage::Disconnect => return false,
        }
        true
    }

    #[tracing::instrument(skip(self), level = "trace")]
    fn tick(&mut self) {
        self.tick += 1;
        for event in self.loader.update(&self.viewer) {
            let message = match event {
                LoadEvent::Loaded(column) => ServerMessage::Column(Box::new(column)),
                LoadEvent::Unloaded(pos) => ServerMessage::UnloadColumn(pos),
            };
            self.connection.send(message);
        }
        self.connection
            .send(ServerMessage::Tick { tick: self.tick });
    }
}
