//! What the menus can do with worlds: list, create, open (after a backup), delete, and
//! start the integrated server for one.

use crate::offline;
use anyhow::Context;
use glam::DVec3;
use std::path::PathBuf;
use std::sync::Arc;
use strata_client::launcher::{LaunchError, Launcher, Panorama, RunningWorld};
use strata_core::content::Content;
use strata_core::coords::DEFAULT_WORLD_BORDER;
use strata_core::dimension::DimensionId;
use strata_protocol::{GameMode, NewWorld, WorldSummary};
use strata_server::save::{self, FORMAT_VERSION, SaveError, WorldFolder, WorldMeta};
use strata_server::{IntegratedServer, ServerConfig, WorldSettings};
use strata_worldgen::{Generator, Seed, WorldgenContent};

/// Columns around the title screen camera.
const PANORAMA_RADIUS: u32 = 5;
const PANORAMA_SEED: &str = "Strata Mundi";

pub struct GameLauncher {
    pub content: Arc<Content>,
    pub worldgen: Arc<WorldgenContent>,
    pub dimension: DimensionId,
    pub saves: PathBuf,
    pub backups: PathBuf,
    pub backups_kept: usize,
    pub autosave_ticks: u64,
    pub generation_threads: usize,
    pub view_distance: u32,
}

impl GameLauncher {
    /// A world that is never written (benchmarks, `--seed`).
    pub fn temporary(&self, seed: Seed) -> anyhow::Result<RunningWorld> {
        let meta = WorldMeta {
            format: FORMAT_VERSION,
            game_version: env!("CARGO_PKG_VERSION").to_owned(),
            name: String::new(),
            seed: seed.0,
            seed_text: String::new(),
            mode: GameMode::Creative,
            border: DEFAULT_WORLD_BORDER,
            created: 0,
            last_played: 0,
            play_ticks: 0,
        };
        self.start(meta, None)
            .context("could not start the server thread")
    }

    fn start(&self, meta: WorldMeta, folder: Option<WorldFolder>) -> std::io::Result<RunningWorld> {
        tracing::info!(seed = meta.seed as i64, name = %meta.name, "starting world");
        let generator = Arc::new(Generator::new(Seed(meta.seed), Arc::clone(&self.worldgen)));
        let (connection, server_end) = strata_protocol::local_pair();
        let config = ServerConfig {
            dimension: self.dimension.clone(),
            generator,
            content: Arc::clone(&self.content),
            generation_threads: self.generation_threads,
            view_distance: self.view_distance,
            world: WorldSettings {
                meta,
                folder,
                autosave_ticks: self.autosave_ticks,
            },
        };
        let server = IntegratedServer::spawn(server_end, config)?;
        Ok(RunningWorld { connection, server })
    }

    fn start_saved(
        &self,
        folder: WorldFolder,
        meta: WorldMeta,
    ) -> Result<RunningWorld, LaunchError> {
        self.start(meta, Some(folder)).map_err(|error| {
            tracing::error!(%error, "could not start the server thread");
            LaunchError::Unavailable
        })
    }
}

impl Launcher for GameLauncher {
    fn worlds(&self) -> Vec<WorldSummary> {
        save::list(&self.saves)
    }

    fn create(&mut self, world: NewWorld) -> Result<RunningWorld, LaunchError> {
        let seed = match world.seed.trim() {
            "" => crate::random_seed(),
            text => Seed::from_text(text),
        };
        let now = chrono::Utc::now().timestamp();
        let (folder, meta) = save::create(&self.saves, &world, seed.0, now).map_err(reported)?;
        self.start_saved(folder, meta)
    }

    fn open(&mut self, id: &str) -> Result<RunningWorld, LaunchError> {
        let (folder, meta) = save::open(&self.saves, id).map_err(reported)?;
        let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
        if let Err(error) = save::back_up(&folder, &self.backups, &stamp, self.backups_kept) {
            tracing::warn!(%error, "backup failed; the world opens anyway");
        }
        self.start_saved(folder, meta)
    }

    fn delete(&mut self, id: &str) -> Result<(), LaunchError> {
        save::delete(&self.saves, id).map_err(reported)
    }

    fn panorama(&self) -> Panorama {
        let generator = Arc::new(Generator::new(
            Seed::from_text(PANORAMA_SEED),
            Arc::clone(&self.worldgen),
        ));
        let spawn = generator.spawn_point();
        let center = DVec3::new(
            f64::from(spawn.x) + 0.5,
            f64::from(spawn.y),
            f64::from(spawn.z) + 0.5,
        );
        Panorama {
            world: offline::generate_around(&generator, center.x, center.z, PANORAMA_RADIUS),
            center,
        }
    }
}

fn reported(error: SaveError) -> LaunchError {
    tracing::error!(%error, "world unavailable");
    match error {
        SaveError::Incompatible { .. } => LaunchError::Incompatible,
        _ => LaunchError::Unavailable,
    }
}
