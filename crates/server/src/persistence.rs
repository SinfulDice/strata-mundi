//! Saving a running world: modified columns, the player and the world description.
//! Errors are logged, never fatal: the game goes on and the next save tries again.

use crate::clock::DAY_TICKS;
use crate::game::Game;
use crate::loader::ColumnLoader;
use crate::save::{PlayerData, WorldFolder, WorldMeta, back_up};
use glam::DVec3;
use std::path::PathBuf;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::dimension::DimensionId;
use strata_core::physics::Body;
use strata_protocol::HotbarState;

const MAX_ICON_BYTES: usize = 256 * 1024;

/// The world being played: where it is saved (`None` for a temporary world, e.g. a
/// benchmark) and how often.
pub struct WorldSettings {
    pub meta: WorldMeta,
    pub folder: Option<WorldFolder>,
    pub autosave_ticks: u64,
    /// Where copies of the world go before it is played, and how many are kept.
    pub backups: Option<(PathBuf, usize)>,
}

pub struct Persistence {
    pub meta: WorldMeta,
    folder: Option<WorldFolder>,
    content: Arc<Content>,
    autosave_ticks: u64,
    since_save: u64,
}

impl Persistence {
    /// Makes the backup copy first (on the server thread, before anything is written).
    pub fn new(settings: WorldSettings, content: Arc<Content>) -> Self {
        if let (Some(folder), Some((dir, keep))) = (&settings.folder, &settings.backups) {
            let stamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S").to_string();
            if let Err(error) = back_up(folder, dir, &stamp, *keep) {
                tracing::warn!(%error, "backup failed; the world opens anyway");
            }
        }
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
    ) -> (Body, [f32; 2], HotbarState) {
        let saved = self.folder.as_ref().and_then(|folder| {
            folder
                .read_player()
                .map_err(|error| tracing::error!(%error, "player data unreadable: new start"))
                .ok()
                .flatten()
        });
        let usable = |player: &PlayerData| {
            player.dimension == dimension.0
                && player.body.position.is_finite()
                && player.body.velocity.is_finite()
                && player.look.iter().all(|a| a.is_finite())
        };
        match saved.filter(usable) {
            Some(player) => (player.body, player.look, self.hotbar_from(&player)),
            None => (Body::at(spawn()), [0.0; 2], HotbarState::default()),
        }
    }

    /// Folder of the modified columns of `dimension`, if the world is saved.
    pub fn regions(&self, dimension: &DimensionId) -> Option<std::path::PathBuf> {
        self.folder.as_ref().map(|folder| folder.regions(dimension))
    }

    /// Counts a played tick; `true` when an autosave is due.
    pub fn count_tick(&mut self) -> bool {
        self.meta.play_ticks += 1;
        self.meta.day_time = (self.meta.day_time % DAY_TICKS + 1) % DAY_TICKS;
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
        if let Some(icon) = icon.filter(|png| is_small_png(png)) {
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
            look: game.look(),
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

/// World captures are tiny PNG files; anything else sent as one is ignored.
fn is_small_png(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_ICON_BYTES && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::loader::tests::loader;
    use crate::rules::BlockRules;
    use crate::save;
    use std::path::Path;
    use strata_core::content::ItemId;
    use strata_core::pack::PackSet;
    use strata_protocol::{GameMode, NewWorld};

    fn content() -> Arc<Content> {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        Arc::new(Content::load(&packs, &PackSet::default()).0)
    }

    #[test]
    fn a_saved_player_comes_back_as_left() {
        let saves = std::env::temp_dir().join(format!("strata-persist-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&saves);
        let new = NewWorld {
            name: "Test".into(),
            seed: String::new(),
            mode: GameMode::Creative,
            border: 5_000,
        };
        let (folder, meta) = save::create(&saves, &new, 9, 100).unwrap();
        let settings = |meta| WorldSettings {
            meta,
            folder: Some(folder.clone()),
            autosave_ticks: 0,
            backups: None,
        };
        let content = content();
        let dimension = DimensionId("strata:terra".parse().unwrap());
        let mut persistence = Persistence::new(settings(meta), Arc::clone(&content));
        let body = Body::at(DVec3::new(3.5, 140.0, -7.5));
        let mut game = Game::new(BlockRules::new(&content), 5_000, body, GameMode::Creative);
        let hotbar = HotbarState {
            slots: vec![Some(ItemId(2)), None, Some(ItemId(5))],
            selected: 2,
        };
        game.set_hotbar(hotbar.clone());
        game.set_look([1.0, -0.5]);
        persistence.count_tick();
        persistence.save(&game, &mut loader(), &dimension, None);

        let reopened = Persistence::new(settings(folder.read_meta().unwrap()), content);
        let (body_back, look, hotbar_back) = reopened.player_start(&dimension, || DVec3::ZERO);
        assert_eq!(body_back.position, body.position);
        assert_eq!(look, [1.0, -0.5]);
        assert_eq!(hotbar_back, hotbar);
        assert_eq!(reopened.meta.play_ticks, 1);
        assert!(reopened.meta.last_played > 100);
        std::fs::remove_dir_all(saves).unwrap();
    }
}
