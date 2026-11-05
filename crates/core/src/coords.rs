use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};

pub const SECTION_SIZE: i32 = 16;
pub const SECTIONS_PER_COLUMN: i32 = 32;
pub const WORLD_HEIGHT: i32 = SECTION_SIZE * SECTIONS_PER_COLUMN;
/// Default distance from the origin to the world border, in blocks (settable per world).
pub const DEFAULT_WORLD_BORDER: i32 = 10_000_000;

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

    /// Strictly inside the square of half-width `border` (no overflow at `i32` limits).
    pub const fn is_inside_border(self, border: i32) -> bool {
        let limit = border.unsigned_abs();
        self.x.unsigned_abs() < limit && self.z.unsigned_abs() < limit
    }

    /// A position anyone may act on: inside the border and the world height. Checked
    /// before any neighbour arithmetic, so hostile coordinates cannot overflow.
    pub const fn is_valid(self, border: i32) -> bool {
        self.is_in_height_range() && self.is_inside_border(border)
    }
}

/// Map keyed by column, with a hash much cheaper than the default one: column
/// positions come from the game, not from an attacker, so collision resistance is useless
/// and hashing was most of the cost of light propagation.
pub type ColumnMap<V> = HashMap<ColumnPos, V, BuildHasherDefault<ColumnHasher>>;

/// Multiply-rotate hash (as in rustc's `FxHasher`) for small integer keys.
#[derive(Debug, Clone, Copy, Default)]
pub struct ColumnHasher(u64);

impl Hasher for ColumnHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        bytes.iter().for_each(|&byte| self.add(u64::from(byte)));
    }

    fn write_i32(&mut self, value: i32) {
        self.add(u64::from(value as u32));
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }
}

impl ColumnHasher {
    fn add(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
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
