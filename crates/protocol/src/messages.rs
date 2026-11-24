use crate::GameMode;
use crate::commands::Feedback;
use crate::inventory::{CraftingView, FurnaceView, InventoryRequest, OpenedContainer, TankView};
use serde::{Deserialize, Serialize};
use strata_core::chunk::{BiomeMap, Column};
use strata_core::content::{BiomeTints, BlockId, EntityTypeId, Face, ItemId, RecipeId};
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
    /// Survival: starts hitting the block at `pos` (the server counts the time from here).
    /// Another block, or `StopBreak`, ends the previous one.
    StartBreak { pos: BlockPos, face: Face },
    /// Survival: the player stopped hitting before the block broke.
    StopBreak,
    /// Breaks the block: at once in creative; in survival, once it has been hit long
    /// enough since `StartBreak` (instant blocks need no start).
    Break { pos: BlockPos },
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
    UseBlock { pos: BlockPos },
    /// Starts eating, drinking or applying `item`, held in hotbar slot `slot`, until
    /// `StopUse` (or another item, or another slot) stops it.
    StartUse { item: ItemId, slot: u8 },
    /// The use button was let go before the item was consumed.
    StopUse,
    /// Fills the held empty container (a bottle) at the liquid source at `pos`.
    Fill { item: ItemId, pos: BlockPos },
    /// Sneaking with an empty hand, drinks a sip at the liquid source at `pos`.
    Drink { pos: BlockPos },
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
        body: Box<Body>,
        movement: Box<MovementSettings>,
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
    /// How far someone is breaking a block, 0 to 255 (each client turns it into its
    /// own crack stages), or `None` when they stop. Sent for every breaker, so other
    /// players' work shows too.
    Cracking {
        breaker: NetEntity,
        pos: BlockPos,
        progress: Option<u8>,
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
    /// The open container went up a tier (a kit): its block and where its upgrade
    /// slots start now.
    ContainerUpgraded {
        block: BlockId,
        upgrades_at: u16,
    },
    /// The container is gone or out of reach: the screen closes.
    ContainerClosed,
    /// The open furnace's fire and cooking changed.
    Furnace(FurnaceView),
    /// The open bag's tank changed (or its tank upgrade came or went).
    Tank(Option<TankView>),
    /// The crafting grid changed, or a work station opened (or closed: `station` empty).
    Crafting(Box<CraftingView>),
    /// The recipes the player knows; `all` when every recipe counts as known (creative,
    /// or the world option « recipes: all known »).
    Recipes {
        known: Vec<RecipeId>,
        all: bool,
    },
    /// A recipe crafted for the first time (by experimenting): now in the recipe book.
    RecipeLearned(RecipeId),
    /// Answer to a console command.
    Feedback(Feedback),
    /// The survival gauges changed.
    Vitals(Box<crate::VitalsView>),
    /// The player was hurt.
    Hurt(crate::HurtView),
    /// The player fell unconscious and woke at the spawn point, keeping everything
    /// (until death comes with milestone 10).
    Fainted,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[path = "messages_tests.rs"]
mod tests;
