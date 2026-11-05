//! Saving a running world: modified columns, the player and the world description.
//! Errors are logged, never fatal: the game goes on and the next save tries again.

use crate::clock::DAY_TICKS;
use crate::game::Game;
use crate::loader::ColumnLoader;
use crate::save::{EntitiesData, PlayerData, SavedBoat, WorldFolder, WorldMeta, back_up};
use glam::DVec3;
use std::path::PathBuf;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::content::EntityTypeId;
use strata_core::dimension::DimensionId;
use strata_core::physics::{BoatBody, Body};
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

/// Where and how the player comes back.
#[derive(Debug, Clone, PartialEq)]
pub struct PlayerStart {
    pub body: Body,
    pub look: [f32; 2],
    pub hotbar: HotbarState,
    /// Index of the boat they sat in, among the saved boats.
    pub boat: Option<usize>,
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
    ) -> PlayerStart {
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
            Some(player) => PlayerStart {
                body: player.body,
                look: player.look,
                hotbar: self.hotbar_from(&player),
                boat: player.boat,
            },
            None => PlayerStart {
                body: Body::at(spawn()),
                look: [0.0; 2],
                hotbar: HotbarState::default(),
                boat: None,
            },
        }
    }

    /// Boats saved in `dimension`; those whose type is gone (pack removed) are dropped.
    pub fn boats(&self, dimension: &DimensionId) -> Vec<Option<(EntityTypeId, BoatBody)>> {
        let Some(folder) = &self.folder else {
            return Vec::new();
        };
        let data = folder.read_entities(dimension).unwrap_or_else(|error| {
            tracing::error!(%error, "entities unreadable: none loaded");
            EntitiesData::default()
        });
        let entities = &self.content.entities;
        data.boats
            .into_iter()
            .map(|boat| {
                let kind = entities.key(&boat.kind)?;
                let position = DVec3::from_array(boat.position);
                let usable = position.is_finite() && boat.yaw.is_finite();
                usable.then(|| (kind, BoatBody::at(position, boat.yaw)))
            })
            .collect()
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
        let (boats, seat) = self.boats_data(game);
        results.push(folder.write_entities(dimension, &boats));
        if let Some(mut player) = self.player_data(game, dimension) {
            player.boat = seat;
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
            boat: None,
        })
    }

    fn boats_data(&self, game: &Game) -> (EntitiesData, Option<usize>) {
        let (boats, seat) = game.saved_boats();
        let entities = &self.content.entities;
        let boats = boats
            .into_iter()
            .filter_map(|(kind, body)| {
                Some(SavedBoat {
                    kind: entities.identifier(kind)?.clone(),
                    position: body.position.to_array(),
                    yaw: body.yaw,
                })
            })
            .collect();
        (EntitiesData { boats }, seat)
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
        let start = reopened.player_start(&dimension, || DVec3::ZERO);
        assert_eq!(start.body.position, body.position);
        assert_eq!(start.look, [1.0, -0.5]);
        assert_eq!(start.hotbar, hotbar);
        assert_eq!(start.boat, None);
        assert_eq!(reopened.meta.play_ticks, 1);
        assert!(reopened.meta.last_played > 100);
        std::fs::remove_dir_all(saves).unwrap();
    }

    #[test]
    fn boats_and_the_seat_come_back_and_old_worlds_have_none() {
        let saves = std::env::temp_dir().join(format!("strata-boats-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&saves);
        let new = NewWorld {
            name: "Boats".into(),
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
        assert!(persistence.boats(&dimension).is_empty(), "no file yet");
        let rules = BlockRules::new(&content);
        let body = Body::at(DVec3::new(0.5, 70.0, 0.5));
        let mut game = Game::new(rules.clone(), 5_000, body, GameMode::Creative);
        let kind = |name: &str| content.entities.key(&name.parse().unwrap()).unwrap();
        let (birch, acacia) = (kind("strata:birch_boat"), kind("strata:acacia_boat"));
        let first = BoatBody::at(DVec3::new(4.5, 63.7, -2.5), 1.25);
        let second = BoatBody::at(DVec3::new(-8.5, 63.7, 3.5), 0.0);
        game.spawn_boat(birch, first);
        game.restore_boats(vec![Some((acacia, second))], Some(0));
        persistence.save(&game, &mut loader(), &dimension, None);

        let reopened = Persistence::new(settings(folder.read_meta().unwrap()), content);
        let boats = reopened.boats(&dimension);
        assert_eq!(boats, vec![Some((birch, first)), Some((acacia, second))]);
        let start = reopened.player_start(&dimension, || DVec3::ZERO);
        assert_eq!(start.boat, Some(1));
        let mut back = Game::new(rules, 5_000, start.body, GameMode::Creative);
        back.restore_boats(boats, start.boat);
        let (saved, seat) = back.saved_boats();
        assert_eq!((saved.len(), seat), (2, Some(1)));
        std::fs::remove_dir_all(saves).unwrap();
    }
}
