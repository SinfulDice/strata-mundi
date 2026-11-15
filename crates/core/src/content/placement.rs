//! How a block is laid when placed: fixed, along an axis, attached or facing.

use crate::id::Identifier;
use serde::Deserialize;

/// How a block turns when placed.
#[derive(Debug, Clone, PartialEq, Eq, Default, Deserialize)]
pub enum Placement {
    #[default]
    Fixed,
    /// Lies along the axis of the clicked face, like logs. The pack describes it upright.
    Axis,
    /// Held by the clicked block: on the floor, against a wall or under a ceiling
    /// (torches, lanterns). It drops when its support goes.
    Attached {
        /// Stands on the block below (drawn as in the pack).
        #[serde(default = "yes")]
        floor: bool,
        /// Leans against the side of a block (drawn tilted toward it).
        #[serde(default)]
        wall: bool,
        /// Hangs under the block above.
        #[serde(default)]
        ceiling: bool,
        /// Texture of the hanging variant (a lantern with its chain); the block's own if
        /// absent.
        #[serde(default)]
        ceiling_texture: Option<Identifier>,
    },
    /// Turns its front (the pack's north face) toward the player who places it
    /// (furnaces); `lit` adds a lit variant of each, switched by the server.
    Facing {
        #[serde(default)]
        lit: Option<LitDef>,
    },
}

/// The lit look of a facing block: another front texture and the light it gives off.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename = "Lit", deny_unknown_fields)]
pub struct LitDef {
    pub front: Identifier,
    #[serde(default)]
    pub light_emission: u8,
}

fn yes() -> bool {
    true
}
