//! Saving a running world: modified columns, the player and the world description.
//! Errors are logged, never fatal: the game goes on and the next save tries again.

use crate::game::Game;
use crate::loader::ColumnLoader;
use crate::save::{PlayerData, WorldFolder, WorldMeta};
use glam::DVec3;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::dimension::DimensionId;
use strata_core::physics::Body;
use strata_protocol::HotbarState;

/// The world being played: where it is saved (`None` for a temporary world, e.g. a
/// benchmark) and how often.
pub struct WorldSettings {
    pub meta: WorldMeta,
    pub folder: Option<WorldFolder>,
    pub autosave_ticks: u64,
}

pub struct Persistence {
    pub meta: WorldMeta,
    folder: Option<WorldFolder>,
    content: Arc<Content>,
    autosave_ticks: u64,
    since_save: u64,
}

impl Persistence {
    pub fn new(settings: WorldSettings, content: Arc<Content>) -> Self {
        Self {
            meta: settings.meta,
            folder: settings.folder,
            content,
            autosave_ticks: settings.autosave_ticks,
            since_save: 0,
        }
    }

    /// Where the player starts: as saved, or standing on `spawn` the first time.
    pub fn player_start(
        &self,
        dimension: &DimensionId,
        spawn: impl FnOnce() -> DVec3,
    ) -> (Body, HotbarState) {
        let saved = self.folder.as_ref().and_then(|folder| {
            folder
                .read_player()
                .map_err(|error| tracing::error!(%error, "player data unreadable: new start"))
                .ok()
                .flatten()
        });
        match saved.filter(|player| player.dimension == dimension.0) {
            Some(player) => (player.body, self.hotbar_from(&player)),
            None => (Body::at(spawn()), HotbarState::default()),
        }
    }

    /// Folder of the modified columns of `dimension`, if the world is saved.
    pub fn regions(&self, dimension: &DimensionId) -> Option<std::path::PathBuf> {
        self.folder.as_ref().map(|folder| folder.regions(dimension))
    }

    /// Counts a played tick; `true` when an autosave is due.
    pub fn count_tick(&mut self) -> bool {
        self.meta.play_ticks += 1;
        self.since_save += 1;
        self.autosave_ticks > 0 && self.since_save >= self.autosave_ticks
    }

    pub fn save(
        &mut self,
        game: &Game,
        loader: &mut ColumnLoader,
        dimension: &DimensionId,
        icon: Option<&[u8]>,
    ) {
        self.since_save = 0;
        let Some(folder) = &self.folder else {
            return;
        };
        let started = std::time::Instant::now();
        self.meta.last_played = chrono::Utc::now().timestamp();
        let mut results = vec![loader.save(), folder.write_meta(&self.meta)];
        if let Some(player) = self.player_data(game, dimension) {
            results.push(folder.write_player(&player));
        }
        if let Some(icon) = icon {
            results.push(folder.write_icon(icon));
        }
        for error in results.into_iter().filter_map(Result::err) {
            tracing::error!(%error, "save failed");
        }
        tracing::info!(world = %folder.id, ms = started.elapsed().as_millis(), "world saved");
    }

    fn player_data(&self, game: &Game, dimension: &DimensionId) -> Option<PlayerData> {
        let body = *game.player_body()?;
        let hotbar = game
            .ecs
            .hotbars
            .get(game.player)
            .cloned()
            .unwrap_or_default();
        let items = &self.content.items;
        Some(PlayerData {
            dimension: dimension.0.clone(),
            body,
            hotbar: hotbar
                .slots
                .iter()
                .map(|slot| slot.and_then(|item| items.identifier(item).cloned()))
                .collect(),
            selected: usize::from(hotbar.selected),
        })
    }

    /// Items that no longer exist (pack removed) leave an empty slot.
    fn hotbar_from(&self, player: &PlayerData) -> HotbarState {
        let items = &self.content.items;
        HotbarState {
            slots: player
                .hotbar
                .iter()
                .map(|slot| slot.as_ref().and_then(|id| items.key(id)))
                .collect(),
            selected: u8::try_from(player.selected).unwrap_or(0),
        }
    }
}
