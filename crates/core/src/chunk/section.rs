use super::packed::{PackedArray, bits_for};
use crate::content::BlockId;
use crate::coords::SECTION_SIZE;

pub const SECTION_VOLUME: usize = (SECTION_SIZE * SECTION_SIZE * SECTION_SIZE) as usize;

/// 16×16×16 blocks stored as a small palette of block ids plus packed indices into it:
/// a section with few kinds of blocks takes little memory, an all-air section almost none.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    palette: Vec<BlockId>,
    indices: PackedArray,
    non_air: u16,
}

impl Default for Section {
    fn default() -> Self {
        Self::filled(BlockId::AIR)
    }
}

impl Section {
    pub fn filled(block: BlockId) -> Self {
        Self {
            palette: vec![block],
            indices: PackedArray::new(SECTION_VOLUME, 0),
            non_air: if block == BlockId::AIR {
                0
            } else {
                SECTION_VOLUME as u16
            },
        }
    }

    /// Local coordinates in `0..16`.
    pub fn get(&self, x: u32, y: u32, z: u32) -> BlockId {
        self.palette[usize::from(self.indices.get(index(x, y, z)))]
    }

    pub fn set(&mut self, x: u32, y: u32, z: u32, block: BlockId) {
        let i = index(x, y, z);
        let previous = self.palette[usize::from(self.indices.get(i))];
        if previous == block {
            return;
        }
        let entry = self.palette_entry(block);
        self.indices.set(i, entry);
        match (previous == BlockId::AIR, block == BlockId::AIR) {
            (true, false) => self.non_air += 1,
            (false, true) => self.non_air -= 1,
            _ => {}
        }
    }

    pub fn is_empty(&self) -> bool {
        self.non_air == 0
    }

    pub fn non_air_count(&self) -> u16 {
        self.non_air
    }

    /// Single block type everywhere: lets meshing skip the inside of solid sections.
    pub fn uniform(&self) -> Option<BlockId> {
        (self.palette.len() == 1).then(|| self.palette[0])
    }

    pub fn palette(&self) -> &[BlockId] {
        &self.palette
    }

    pub fn heap_bytes(&self) -> usize {
        self.indices.heap_bytes() + self.palette.capacity() * std::mem::size_of::<BlockId>()
    }

    fn palette_entry(&mut self, block: BlockId) -> u16 {
        if let Some(position) = self.palette.iter().position(|&b| b == block) {
            return position as u16;
        }
        self.palette.push(block);
        let needed = bits_for(self.palette.len());
        if needed > self.indices.bits() {
            self.indices = self.indices.resized(needed);
        }
        (self.palette.len() - 1) as u16
    }
}

/// Y-major then Z then X: horizontal slices are contiguous, which suits terrain filling.
fn index(x: u32, y: u32, z: u32) -> usize {
    debug_assert!(x < 16 && y < 16 && z < 16);
    ((y * 16 + z) * 16 + x) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_section_uses_no_index_memory() {
        let section = Section::default();
        assert!(section.is_empty());
        assert_eq!(section.get(3, 4, 5), BlockId::AIR);
        assert_eq!(section.heap_bytes(), std::mem::size_of::<BlockId>());
    }

    #[test]
    fn palette_grows_and_counts_blocks() {
        let mut section = Section::default();
        section.set(1, 2, 3, BlockId(5));
        section.set(4, 5, 6, BlockId(7));
        assert_eq!(section.get(1, 2, 3), BlockId(5));
        assert_eq!(section.get(4, 5, 6), BlockId(7));
        assert_eq!(section.palette(), &[BlockId::AIR, BlockId(5), BlockId(7)]);
        section.set(4, 5, 6, BlockId::AIR);
        assert_eq!(section.non_air_count(), 1);
    }

    #[test]
    fn many_block_types_still_round_trip() {
        let mut section = Section::default();
        for i in 0..300u32 {
            section.set(i % 16, (i / 16) % 16, i / 256, BlockId(i as u16 + 1));
        }
        for i in 0..300u32 {
            assert_eq!(
                section.get(i % 16, (i / 16) % 16, i / 256),
                BlockId(i as u16 + 1)
            );
        }
    }
}
