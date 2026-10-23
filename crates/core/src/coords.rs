use serde::{Deserialize, Serialize};

pub const SECTION_SIZE: i32 = 16;
pub const SECTIONS_PER_COLUMN: i32 = 32;
pub const WORLD_HEIGHT: i32 = SECTION_SIZE * SECTIONS_PER_COLUMN;

/// Absolute block position. `y` ranges over `0..WORLD_HEIGHT`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockPos {
    pub x: i32,
    pub y: i32,
    pub z: i32,
}

/// Horizontal position of a 16-wide column of sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ColumnPos {
    pub x: i32,
    pub z: i32,
}

impl BlockPos {
    pub const fn new(x: i32, y: i32, z: i32) -> Self {
        Self { x, y, z }
    }

    pub const fn column(self) -> ColumnPos {
        ColumnPos {
            x: self.x.div_euclid(SECTION_SIZE),
            z: self.z.div_euclid(SECTION_SIZE),
        }
    }

    pub const fn is_in_height_range(self) -> bool {
        self.y >= 0 && self.y < WORLD_HEIGHT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_coordinates_map_to_the_right_column() {
        assert_eq!(
            BlockPos::new(-1, 0, -16).column(),
            ColumnPos { x: -1, z: -1 }
        );
        assert_eq!(
            BlockPos::new(-17, 0, 15).column(),
            ColumnPos { x: -2, z: 0 }
        );
        assert_eq!(BlockPos::new(16, 0, 0).column(), ColumnPos { x: 1, z: 0 });
    }

    #[test]
    fn height_range_is_0_to_511() {
        assert_eq!(WORLD_HEIGHT, 512);
        assert!(BlockPos::new(0, 511, 0).is_in_height_range());
        assert!(!BlockPos::new(0, 512, 0).is_in_height_range());
        assert!(!BlockPos::new(0, -1, 0).is_in_height_range());
    }
}
