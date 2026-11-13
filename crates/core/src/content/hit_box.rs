//! Where the aim stops on a block, and what holds a block up.

use super::attached::Attachment;
use super::block::Face;

/// Box inside a block, in fractions of the block (0 to 1 on each axis).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HitBox {
    pub min: [f64; 3],
    pub max: [f64; 3],
}

impl HitBox {
    pub const FULL: Self = Self {
        min: [0.0; 3],
        max: [1.0; 3],
    };
    /// Narrower and lower than a cube, so the aim slips past plants at their edges.
    pub const PLANT: Self = Self {
        min: [0.125, 0.0, 0.125],
        max: [0.875, 0.8125, 0.875],
    };
    /// Lantern under a ceiling, chain included.
    pub const HANGING: Self = Self {
        min: [0.3125, 0.1875, 0.3125],
        max: [0.6875, 1.0, 0.6875],
    };
    /// Torch leaning on a north wall (-Z); the other walls turn it.
    const WALL_NORTH: Self = Self {
        min: [0.3125, 0.1875, 0.0],
        max: [0.6875, 0.875, 0.5],
    };

    pub(super) fn attached(attachment: Attachment) -> Self {
        match attachment.support {
            Face::Down => Self::PLANT,
            Face::Up => Self::HANGING,
            wall => Self::WALL_NORTH.turned_toward(wall),
        }
    }

    /// The box turned about the vertical axis so that its north side faces `wall`.
    fn turned_toward(self, wall: Face) -> Self {
        let [x0, y0, z0] = self.min;
        let [x1, y1, z1] = self.max;
        let (min, max) = match wall {
            Face::South => ([1.0 - x1, y0, 1.0 - z1], [1.0 - x0, y1, 1.0 - z0]),
            Face::West => ([z0, y0, x0], [z1, y1, x1]),
            Face::East => ([1.0 - z1, y0, 1.0 - x1], [1.0 - z0, y1, 1.0 - x0]),
            _ => (self.min, self.max),
        };
        Self { min, max }
    }
}

/// What holds a block up: the neighbour on `side` must be solid (or the same block when
/// `chains`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Support {
    pub side: Face,
    pub chains: bool,
}

impl Support {
    pub(super) const fn new(side: Face, chains: bool) -> Self {
        Self { side, chains }
    }
}
