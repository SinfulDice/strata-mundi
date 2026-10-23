use super::registry::TextureId;
use crate::id::Identifier;
use serde::Deserialize;

/// How a block is drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
pub enum RenderKind {
    /// Not drawn at all (air).
    Invisible,
    /// Fully opaque cube: hides the faces of its neighbours.
    #[default]
    Opaque,
    /// Pixels are either opaque or fully transparent (leaves).
    Cutout,
    /// Blended, drawn after everything else (water, glass).
    Translucent,
}

/// Faces of a cube. North is -Z, east is +X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    Up,
    Down,
    North,
    South,
    East,
    West,
}

impl RenderKind {
    pub fn is_visible(self) -> bool {
        self != Self::Invisible
    }
}

impl Face {
    pub const ALL: [Face; 6] = [
        Self::Up,
        Self::Down,
        Self::North,
        Self::South,
        Self::East,
        Self::West,
    ];

    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Textures of each face, written in the shortest form that fits.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum BlockTextures {
    None,
    All(Identifier),
    /// Logs, cactus: `end` on top and bottom, `side` around.
    Column {
        end: Identifier,
        side: Identifier,
    },
    TopBottomSides {
        top: Identifier,
        bottom: Identifier,
        sides: Identifier,
    },
    Faces {
        up: Identifier,
        down: Identifier,
        north: Identifier,
        south: Identifier,
        east: Identifier,
        west: Identifier,
    },
}

impl BlockTextures {
    pub fn face(&self, face: Face) -> Option<&Identifier> {
        match self {
            Self::None => None,
            Self::All(id) => Some(id),
            Self::Column { end, side } => Some(match face {
                Face::Up | Face::Down => end,
                _ => side,
            }),
            Self::TopBottomSides { top, bottom, sides } => Some(match face {
                Face::Up => top,
                Face::Down => bottom,
                _ => sides,
            }),
            Self::Faces {
                up,
                down,
                north,
                south,
                east,
                west,
            } => Some(match face {
                Face::Up => up,
                Face::Down => down,
                Face::North => north,
                Face::South => south,
                Face::East => east,
                Face::West => west,
            }),
        }
    }
}

/// A block as written in `blocks/<name>.ron`. Every field but `textures` has a default.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Block", deny_unknown_fields)]
pub struct BlockDef {
    #[serde(default = "yes")]
    pub solid: bool,
    #[serde(default)]
    pub render: RenderKind,
    /// How much light the block stops, 0 (none) to 15 (all). Used from v0.2.
    #[serde(default = "full_opacity")]
    pub light_opacity: u8,
    /// Time to mine, like Minecraft; negative = unbreakable. Used from v0.3.
    #[serde(default = "one")]
    pub hardness: f32,
    pub textures: BlockTextures,
    /// Sound set (v0.2), e.g. `strata:stone`.
    #[serde(default)]
    pub sound: Option<Identifier>,
    /// Tab of the creative inventory.
    #[serde(default)]
    pub category: Option<Identifier>,
    /// Whether an item is created for this block.
    #[serde(default = "yes")]
    pub has_item: bool,
    /// Stack size of the block's item.
    #[serde(default = "default_stack")]
    pub max_stack: u16,
    /// Hide the face between two identical non-opaque blocks (water, glass); leaves keep it.
    #[serde(default = "yes")]
    pub cull_same: bool,
}

fn default_stack() -> u16 {
    64
}

fn yes() -> bool {
    true
}

fn full_opacity() -> u8 {
    15
}

fn one() -> f32 {
    1.0
}

/// A loaded block, ready for meshing and simulation.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub solid: bool,
    pub render: RenderKind,
    pub light_opacity: u8,
    pub hardness: f32,
    /// Texture layer of each face, indexed by `Face::index`.
    pub faces: [TextureId; 6],
    pub sound: Option<Identifier>,
    pub category: Option<Identifier>,
    pub has_item: bool,
    pub max_stack: u16,
    pub cull_same: bool,
}

impl Block {
    pub fn air() -> Self {
        Self {
            solid: false,
            render: RenderKind::Invisible,
            light_opacity: 0,
            hardness: 0.0,
            faces: [TextureId::MISSING; 6],
            sound: None,
            category: None,
            has_item: false,
            max_stack: 64,
            cull_same: true,
        }
    }

    pub fn texture(&self, face: Face) -> TextureId {
        self.faces[face.index()]
    }

    /// Neighbours of an opaque block do not need their shared face drawn.
    pub fn is_opaque(&self) -> bool {
        self.render == RenderKind::Opaque
    }
}
