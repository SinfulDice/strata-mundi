//! The client's copy of the blocks around the player, as sent by the server.

use crate::mesher::{BlockTable, Neighborhood, SectionKey};
use std::collections::HashMap;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::{ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN};

#[derive(Debug, Default)]
pub struct ClientWorld {
    columns: HashMap<ColumnPos, Column>,
}

impl ClientWorld {
    /// Creates the column if needed (used to build scenes on the client).
    pub fn set_block(&mut self, x: i32, y: i32, z: i32, block: BlockId) {
        let pos = column_of(x, z);
        self.columns
            .entry(pos)
            .or_insert_with(|| Column::empty(pos))
            .set(local(x), y, local(z), block);
    }

    /// True when the section and its six neighbours are all one opaque block: nothing
    /// inside can be seen (deep underground), so meshing is skipped.
    pub fn is_buried(&self, key: SectionKey, table: &BlockTable) -> bool {
        let (cx, cz, index) = key;
        let solid = |x: i32, z: i32, y: isize| {
            if y < 0 || y >= SECTIONS_PER_COLUMN as isize {
                return true;
            }
            self.columns
                .get(&ColumnPos { x, z })
                .and_then(|column| column.section(y as usize))
                .and_then(|section| section.uniform())
                .is_some_and(|block| table.is_opaque(block))
        };
        let y = index as isize;
        solid(cx, cz, y)
            && solid(cx, cz, y - 1)
            && solid(cx, cz, y + 1)
            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .all(|(dx, dz)| solid(cx + dx, cz + dz, y))
    }

    /// Non-empty sections, as mesh keys.
    pub fn section_keys(&self) -> impl Iterator<Item = SectionKey> + '_ {
        self.columns.values().flat_map(|column| {
            column
                .sections()
                .iter()
                .enumerate()
                .filter(|(_, section)| !section.is_empty())
                .map(move |(index, _)| (column.pos.x, column.pos.z, index))
        })
    }

    /// Section `key` plus a one-block border, with the 3×3 neighbouring columns looked up once.
    pub fn neighborhood(&self, key: SectionKey) -> Neighborhood {
        let (cx, cz, section) = key;
        let near: [[Option<&Column>; 3]; 3] = std::array::from_fn(|dz| {
            std::array::from_fn(|dx| {
                self.columns.get(&ColumnPos {
                    x: cx + dx as i32 - 1,
                    z: cz + dz as i32 - 1,
                })
            })
        });
        let base_y = section as i32 * SECTION_SIZE;
        Neighborhood::build(|x, y, z| {
            let (dx, lx) = split(x);
            let (dz, lz) = split(z);
            near[dz][dx].map_or(BlockId::AIR, |column| column.get(lx, base_y + y, lz))
        })
    }
}

fn column_of(x: i32, z: i32) -> ColumnPos {
    ColumnPos {
        x: x.div_euclid(SECTION_SIZE),
        z: z.div_euclid(SECTION_SIZE),
    }
}

fn local(c: i32) -> u32 {
    c.rem_euclid(SECTION_SIZE) as u32
}

/// `-1..=16` relative to a section → (neighbour index 0..3, local coordinate).
fn split(c: i32) -> (usize, u32) {
    match c {
        -1 => (0, 15),
        16 => (2, 0),
        _ => (1, c as u32),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neighborhood_reads_across_column_borders() {
        let mut world = ClientWorld::default();
        world.set_block(-1, 20, 0, BlockId(3));
        world.set_block(16, 16, 15, BlockId(4));
        let blocks = world.neighborhood((0, 0, 1));
        assert_eq!(blocks.get(-1, 4, 0), BlockId(3));
        assert_eq!(blocks.get(16, 0, 15), BlockId(4));
        assert_eq!(blocks.get(0, 0, 0), BlockId::AIR);
    }
}
