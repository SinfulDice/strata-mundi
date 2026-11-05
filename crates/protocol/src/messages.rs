use crate::commands::Feedback;
use serde::{Deserialize, Serialize};
use strata_core::chunk::Column;
use strata_core::content::{BlockId, EntityTypeId, Face, ItemId};
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::dimension::DimensionId;
use strata_core::physics::{Body, MoveInput, MovementSettings};
use strata_core::sky::DayCycleDef;

/// Ticks between two `ServerMessage::DayTime` while the time goes on.
pub const DAY_TIME_PERIOD: u64 = 20;

/// Intents sent by the client. The server validates and decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// What the player wants during one tick, numbered so the client can match the
    /// server's answer to its own prediction.
    Input {
        sequence: u32,
        input: MoveInput,
    },
    /// Columns to keep loaded around the player, in each direction
    /// (`MIN_VIEW_DISTANCE..=MAX_VIEW_DISTANCE`).
    ViewDistance(u32),
    /// Comfort options from the client's settings.
    MovementPreferences {
        auto_jump: bool,
        step_up: bool,
    },
    /// Creative only: move the player instantly (benchmarks, `/tp` from milestone 8).
    Teleport {
        feet: [f64; 3],
    },
    BreakBlock {
        pos: BlockPos,
    },
    /// Put `block` against `face` of the block at `against`.
    PlaceBlock {
        against: BlockPos,
        face: Face,
        block: BlockId,
    },
    /// Use `item` (held in the hotbar) on `face` of the block at `against` (buckets).
    UseItem {
        item: ItemId,
        against: BlockPos,
        face: Face,
    },
    /// Right click on an entity (get into a boat).
    InteractEntity(NetEntity),
    /// Left click on an entity (creative: takes a boat away).
    AttackEntity(NetEntity),
    /// The hotbar changed (creative: the client picks any item; kept with the world).
    Hotbar(HotbarState),
    /// Stop or resume the world (pause menu; any menu if the player asked for it).
    Pause(bool),
    /// Save now, with a capture of the screen for the world list.
    Save {
        icon: Option<Vec<u8>>,
    },
    /// The player leaves: the server saves (with this capture) and stops.
    Disconnect {
        icon: Option<Vec<u8>>,
    },
    /// A console line for the server (`tp 0 80 0`, without the `/`).
    Command(String),
}

/// An entity as the network names it: stable while it lives, never reused by another.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct NetEntity(pub u64);

/// What a client needs to draw an entity other than its own player.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EntityState {
    pub id: NetEntity,
    pub kind: EntityTypeId,
    /// Bottom centre.
    pub position: [f64; 3],
    /// Radians, 0 = north (-Z).
    pub yaw: f32,
}

/// The nine quick slots and the selected one.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HotbarState {
    pub slots: Vec<Option<ItemId>>,
    pub selected: u8,
}

/// The day of the player's dimension, given on arrival so the first image of the world
/// already has its sky.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct DayState {
    /// Ticks since the first sunrise.
    pub time: u64,
    /// Whether the time goes on.
    pub flows: bool,
    /// The server's cycle (bounded), whatever the client's packs say.
    pub cycle: DayCycleDef,
}

/// State broadcast by the server.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerMessage {
    /// First message: where and how the player starts.
    Welcome {
        dimension: DimensionId,
        body: Body,
        movement: MovementSettings,
        /// Half-width of the world, in blocks.
        border: i32,
        hotbar: HotbarState,
        /// Yaw and pitch, in radians.
        look: [f32; 2],
        day: DayState,
    },
    /// Sent once per tick so the client can interpolate between ticks.
    Tick {
        tick: u64,
    },
    /// World time in ticks since the first sunrise, and whether it goes on. Sent on
    /// arrival, every `DAY_TIME_PERIOD` ticks and after `/time`; the client interpolates.
    DayTime {
        time: u64,
        cycle: bool,
    },
    /// Authoritative state of the player after applying input `sequence`.
    PlayerState {
        sequence: u32,
        body: Body,
    },
    /// A column of blocks entering the player's view distance.
    Column(Box<Column>),
    /// A column leaving the view distance: the client forgets it.
    UnloadColumn(ColumnPos),
    BlockChanged {
        pos: BlockPos,
        block: BlockId,
    },
    /// An entity came into existence (or is already there when the player arrives).
    EntitySpawned(EntityState),
    /// An entity moved or turned during the last tick.
    EntityMoved {
        id: NetEntity,
        position: [f64; 3],
        yaw: f32,
    },
    EntityRemoved(NetEntity),
    /// The player now rides this entity (a boat), or walks again (`None`).
    Mounted(Option<NetEntity>),
    /// Answer to `ClientMessage::Save`: everything is on disk.
    Saved,
    /// The server changed the hotbar (`/give`).
    Hotbar(HotbarState),
    /// Answer to a console command.
    Feedback(Feedback),
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn round_trip<T: Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
        let bytes = postcard::to_stdvec(value).unwrap();
        postcard::from_bytes(&bytes).unwrap()
    }

    #[test]
    fn the_day_cycle_survives_serialization() {
        let day = DayState {
            time: 123_456,
            flows: true,
            cycle: DayCycleDef {
                length: 30_000,
                day_share: 0.62,
                twilight: 0.04,
                night_light: 0.15,
            },
        };
        assert_eq!(round_trip(&day), day);
        let time = ServerMessage::DayTime {
            time: 9,
            cycle: false,
        };
        assert_eq!(round_trip(&time), time);
    }

    #[test]
    fn entity_messages_survive_serialization() {
        let id = NetEntity(7 | (3 << 32));
        let state = EntityState {
            id,
            kind: EntityTypeId(2),
            position: [12.5, 63.8125, -40.25],
            yaw: 1.5,
        };
        let server = [
            ServerMessage::EntitySpawned(state),
            ServerMessage::EntityMoved {
                id,
                position: [1.0, 2.0, 3.0],
                yaw: -0.5,
            },
            ServerMessage::EntityRemoved(id),
            ServerMessage::Mounted(Some(id)),
            ServerMessage::Mounted(None),
        ];
        for message in &server {
            assert_eq!(&round_trip(message), message);
        }
        for message in [
            ClientMessage::InteractEntity(id),
            ClientMessage::AttackEntity(id),
        ] {
            assert_eq!(round_trip(&message), message);
        }
    }
}
