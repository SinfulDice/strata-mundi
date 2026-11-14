use crate::GameMode;
use crate::commands::Feedback;
use crate::inventory::{InventoryRequest, OpenedContainer};
use serde::{Deserialize, Serialize};
use strata_core::chunk::{BiomeMap, Column};
use strata_core::content::{BiomeTints, BlockId, EntityTypeId, Face, ItemId};
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::dimension::DimensionId;
use strata_core::id::Identifier;
use strata_core::inventory::{Container, PlayerInventory};
use strata_core::physics::{Body, MoveInput, MovementSettings};
use strata_core::sky::DayCycleDef;

/// Ticks between two `ServerMessage::DayTime` while the time goes on.
pub const DAY_TIME_PERIOD: u64 = 20;

/// Intents sent by the client. The server validates and decides.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientMessage {
    /// What the player wants during one tick, numbered so the client can match the
    /// server's answer to its own prediction.
    Input { sequence: u32, input: MoveInput },
    /// Columns to keep loaded around the player, in each direction
    /// (`MIN_VIEW_DISTANCE..=MAX_VIEW_DISTANCE`).
    ViewDistance(u32),
    /// Comfort options from the client's settings.
    MovementPreferences { auto_jump: bool, step_up: bool },
    /// Creative only: move the player instantly (benchmarks, `/tp` from milestone 8).
    Teleport { feet: [f64; 3] },
    /// A click in the world, numbered: the server answers each one with `ActionDone`,
    /// in the order of the numbers, once it is done or refused, so the client can drop
    /// its prediction.
    Action { id: ActionId, action: WorldAction },
    /// A click in an inventory screen, the hand's slot, sorting, locking…
    Inventory(InventoryRequest),
    /// Stop or resume the world (pause menu; any menu if the player asked for it).
    Pause(bool),
    /// Save now, with a capture of the screen for the world list.
    Save { icon: Option<Vec<u8>> },
    /// The player leaves: the server saves (with this capture) and stops.
    Disconnect { icon: Option<Vec<u8>> },
    /// A console line for the server (`tp 0 80 0`, without the `/`).
    Command(String),
}

/// Clicks the server keeps waiting for their turn; the client predicts no more clicks
/// than this without an answer.
pub const MAX_PENDING_ACTIONS: usize = 8;

/// Number of a click in the world, counting up from 1 for each client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ActionId(pub u32);

/// What a click in the world asks for. Entities share the numbering and the queue so
/// that no click is lost, whatever it aims at.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorldAction {
    Break {
        pos: BlockPos,
    },
    /// Put `block` against `face` of the block at `against`.
    Place {
        against: BlockPos,
        face: Face,
        block: BlockId,
    },
    /// Use `item` (held in the hotbar) on `face` of the block at `against` (buckets).
    Use {
        item: ItemId,
        against: BlockPos,
        face: Face,
    },
    /// Right click on a block that does something of its own (opens a chest).
    UseBlock {
        pos: BlockPos,
    },
    /// Right click on an entity (get into a boat).
    Interact(NetEntity),
    /// Left click on an entity (creative: takes a boat away).
    Attack(NetEntity),
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
    /// What a dropped item shows.
    pub item: Option<ItemId>,
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

/// A biome of the player's dimension, at its index in `Welcome::biomes`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BiomeInfo {
    pub id: Identifier,
    pub tints: BiomeTints,
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
        inventory: Box<PlayerInventory>,
        mode: GameMode,
        /// Yaw and pitch, in radians.
        look: [f32; 2],
        day: DayState,
        /// Indexed by `BiomeId`.
        biomes: Vec<BiomeInfo>,
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
    /// A column of blocks entering the player's view distance, with its biomes.
    Column {
        column: Box<Column>,
        biomes: BiomeMap,
    },
    /// A column leaving the view distance: the client forgets it.
    UnloadColumn(ColumnPos),
    BlockChanged {
        pos: BlockPos,
        block: BlockId,
    },
    /// Block action `id` is done or refused; the `BlockChanged` it caused (or, when
    /// refused, the actual state of the cells it aimed at) came before.
    ActionDone(ActionId),
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
    /// The game mode changed (`/gamemode`).
    Mode(GameMode),
    /// Answer to `ClientMessage::Save`: everything is on disk.
    Saved,
    /// The player's inventory after a click, a pickup or `/give`.
    Inventory(Box<PlayerInventory>),
    /// The player opened a container (the screen shows it).
    ContainerOpened(Box<OpenedContainer>),
    /// What the open container holds now.
    ContainerContents(Box<Container>),
    /// The container is gone or out of reach: the screen closes.
    ContainerClosed,
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
    fn columns_keep_their_biomes() {
        use strata_core::chunk::BiomeId;
        use strata_core::content::Tint;
        let pos = strata_core::coords::ColumnPos { x: -3, z: 7 };
        let mut column = Column::empty(pos);
        column.set(4, 130, 9, BlockId(5));
        let biomes = BiomeMap::from_fn(|x, z| BiomeId(if x + z < 12 { 3 } else { 300 }));
        let message = ServerMessage::Column {
            column: Box::new(column),
            biomes,
        };
        assert_eq!(round_trip(&message), message);
        let welcome_biomes = vec![BiomeInfo {
            id: "strata:plains".parse().unwrap(),
            tints: BiomeTints {
                kinds: [Tint([1, 2, 3]), Tint([4, 5, 6]), Tint([7, 8, 9])],
                fog: Tint([200, 100, 50]),
            },
        }];
        assert_eq!(round_trip(&welcome_biomes), welcome_biomes);
    }

    #[test]
    fn entity_messages_survive_serialization() {
        let id = NetEntity(7 | (3 << 32));
        let state = EntityState {
            id,
            kind: EntityTypeId(2),
            position: [12.5, 63.8125, -40.25],
            yaw: 1.5,
            item: Some(ItemId(3)),
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
            ServerMessage::ActionDone(ActionId(u32::MAX)),
        ];
        for message in &server {
            assert_eq!(&round_trip(message), message);
        }
        for message in [
            ClientMessage::Action {
                id: ActionId(3),
                action: WorldAction::Interact(id),
            },
            ClientMessage::Action {
                id: ActionId(4),
                action: WorldAction::Attack(id),
            },
            ClientMessage::Action {
                id: ActionId(9),
                action: WorldAction::Place {
                    against: BlockPos::new(1, 2, 3),
                    face: Face::East,
                    block: BlockId(4),
                },
            },
        ] {
            assert_eq!(round_trip(&message), message);
        }
    }

    #[test]
    fn inventory_messages_survive_serialization() {
        use crate::inventory::{InventoryRequest, OpenedContainer};
        use strata_core::inventory::{Area, Click, DataValue, ItemStack, SlotRef, SortOrder};
        let mut stack = ItemStack {
            item: ItemId(4),
            count: 12,
            durability: Some(7),
            data: Default::default(),
        };
        let parts = vec![
            DataValue::Id("strata:oak".parse().unwrap()),
            DataValue::Int(-3),
        ];
        stack
            .data
            .set("strata:parts".parse().unwrap(), DataValue::List(parts));
        let mut inventory = PlayerInventory::default();
        inventory.slots.slots[40] = Some(stack.clone());
        inventory.cursor = Some(stack.clone());
        let mut contents = Container::new(54);
        contents.slots[53] = Some(stack.clone());
        for message in [
            ServerMessage::Inventory(Box::new(inventory)),
            ServerMessage::ContainerOpened(Box::new(OpenedContainer {
                block: BlockId(9),
                pos: BlockPos::new(1, 2, 3),
                contents: contents.clone(),
            })),
            ServerMessage::ContainerContents(Box::new(contents)),
            ServerMessage::ContainerClosed,
        ] {
            assert_eq!(round_trip(&message), message);
        }
        for request in [
            InventoryRequest::Click {
                slot: SlotRef::open(53),
                click: Click::HotbarSwap(8),
            },
            InventoryRequest::Sort {
                area: Area::Open,
                order: SortOrder::Name,
                language: "lol".into(),
            },
            InventoryRequest::CreativeSet {
                index: 3,
                stack: Some(stack),
            },
            InventoryRequest::DropSelected { whole: true },
        ] {
            let message = ClientMessage::Inventory(request);
            assert_eq!(round_trip(&message), message);
        }
    }
}
