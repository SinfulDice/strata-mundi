//! Saving a running world: modified columns, the player and the world description.
//! Errors are logged, never fatal: the game goes on and the next save tries again.

use crate::game::{BlockEntities, BlockEntity, Game};
use crate::loader::ColumnLoader;
use crate::save::{
    BlockEntitiesData, EntitiesData, PlayerData, SavedBlockEntity, SavedBoat, SavedDrop,
    SavedInventory, WorldFolder, WorldMeta, back_up,
};
use glam::DVec3;
use std::path::PathBuf;
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::content::EntityTypeId;
use strata_core::coords::BlockPos;
use strata_core::dimension::DimensionId;
use strata_core::inventory::{HOTBAR, ItemStack, PlayerInventory};
use strata_core::physics::{BoatBody, Body, ItemBody};

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
    pub inventory: PlayerInventory,
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
                inventory: self.inventory_from(&player),
                boat: player.boat,
            },
            None => PlayerStart {
                body: Body::at(spawn()),
                look: [0.0; 2],
                inventory: PlayerInventory::default(),
                boat: None,
            },
        }
    }

    /// Boats saved in `dimension`; those whose type is gone (pack removed) are dropped.
    pub fn boats(&self, dimension: &DimensionId) -> Vec<Option<(EntityTypeId, BoatBody)>> {
        let data = self.entities_data(dimension);
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

    /// Items on the ground saved in `dimension`; unknown items are dropped.
    pub fn drops(&self, dimension: &DimensionId) -> Vec<(ItemStack, ItemBody, u32)> {
        let items = &self.content.items;
        self.entities_data(dimension)
            .items
            .into_iter()
            .filter_map(|drop| {
                let position = DVec3::from_array(drop.position);
                let stack = drop.stack.to_stack(items)?;
                Some((stack, ItemBody::at(position, DVec3::ZERO), drop.age))
            })
            .collect()
    }

    /// Container contents saved in `dimension`.
    pub fn block_entities(&self, dimension: &DimensionId) -> BlockEntities {
        let Some(folder) = &self.folder else {
            return BlockEntities::default();
        };
        let data = folder
            .read_block_entities(dimension)
            .unwrap_or_else(|error| {
                tracing::error!(%error, "container contents unreadable: none loaded");
                BlockEntitiesData::default()
            });
        let items = &self.content.items;
        data.entities
            .into_iter()
            .map(|saved| {
                let [x, y, z] = saved.pos;
                let entity = BlockEntity {
                    container: crate::save::load_slots(&saved.slots, items),
                    data: saved.data,
                };
                (BlockPos::new(x, y, z), entity)
            })
            .collect()
    }

    fn entities_data(&self, dimension: &DimensionId) -> EntitiesData {
        let Some(folder) = &self.folder else {
            return EntitiesData::default();
        };
        folder.read_entities(dimension).unwrap_or_else(|error| {
            tracing::error!(%error, "entities unreadable: none loaded");
            EntitiesData::default()
        })
    }

    /// Folder of the modified columns of `dimension`, if the world is saved.
    pub fn regions(&self, dimension: &DimensionId) -> Option<std::path::PathBuf> {
        self.folder.as_ref().map(|folder| folder.regions(dimension))
    }

    /// Counts a played tick; `true` when an autosave is due.
    pub fn count_tick(&mut self) -> bool {
        self.meta.play_ticks += 1;
        if self.meta.day_cycle {
            self.meta.day_time = self.meta.day_time.saturating_add(1);
        }
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
        let (entities, seat) = self.entities_to_save(game);
        results.push(folder.write_entities(dimension, &entities));
        results.push(folder.write_block_entities(dimension, &self.block_entities_data(game)));
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
        let items = &self.content.items;
        Some(PlayerData {
            dimension: dimension.0.clone(),
            body,
            look: game.look(),
            inventory: Some(SavedInventory::from_inventory(&game.inventory(), items)),
            hotbar: Vec::new(),
            selected: 0,
            boat: None,
        })
    }

    fn block_entities_data(&self, game: &Game) -> BlockEntitiesData {
        let items = &self.content.items;
        let mut entities: Vec<SavedBlockEntity> = game
            .block_entities
            .iter()
            .filter(|(_, entity)| !entity.container.is_empty() || !entity.data.is_empty())
            .map(|(pos, entity)| SavedBlockEntity {
                pos: [pos.x, pos.y, pos.z],
                slots: crate::save::save_slots(&entity.container.slots, items),
                data: entity.data.clone(),
            })
            .collect();
        entities.sort_by_key(|entity| entity.pos);
        BlockEntitiesData { entities }
    }

    fn entities_to_save(&self, game: &Game) -> (EntitiesData, Option<usize>) {
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
        let items = &self.content.items;
        let drops = game
            .saved_drops()
            .into_iter()
            .filter_map(|(stack, body, age)| {
                Some(SavedDrop {
                    stack: crate::save::SavedStack::from_stack(&stack, items)?,
                    position: body.position.to_array(),
                    age,
                })
            })
            .collect();
        (
            EntitiesData {
                boats,
                items: drops,
            },
            seat,
        )
    }

    /// The saved inventory; a world from before v0.3 gets its hotbar back, one of each
    /// item. Items that no longer exist (pack removed) leave an empty slot.
    fn inventory_from(&self, player: &PlayerData) -> PlayerInventory {
        let items = &self.content.items;
        if let Some(saved) = &player.inventory {
            return saved.to_inventory(items);
        }
        let mut inventory = PlayerInventory::default();
        let old = player.hotbar.iter().take(HOTBAR.end);
        for (slot, id) in inventory.slots.slots.iter_mut().zip(old) {
            let item = id.as_ref().and_then(|id| items.key(id));
            *slot = item.map(|item| ItemStack::new(item, 1, items));
        }
        inventory.selected = u8::try_from(player.selected.min(HOTBAR.end - 1)).unwrap_or(0);
        inventory
    }
}

/// World captures are tiny PNG files; anything else sent as one is ignored.
fn is_small_png(bytes: &[u8]) -> bool {
    bytes.len() <= MAX_ICON_BYTES && bytes.starts_with(b"\x89PNG\r\n\x1a\n")
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
