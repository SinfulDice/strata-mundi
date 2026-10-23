use super::section::Section;
use crate::content::BlockId;
use crate::coords::{ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN};

/// A vertical stack of 32 sections (y 0 to 511) at one horizontal position.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    pub pos: ColumnPos,
    sections: Vec<Section>,
}

impl Column {
    pub fn empty(pos: ColumnPos) -> Self {
        Self {
            pos,
            sections: vec![Section::default(); SECTIONS_PER_COLUMN as usize],
        }
    }

    pub fn section(&self, index: usize) -> Option<&Section> {
        self.sections.get(index)
    }

    pub fn sections(&self) -> &[Section] {
        &self.sections
    }

    /// Local x/z in `0..16`, y in `0..512`; anything outside reads air.
    pub fn get(&self, x: u32, y: i32, z: u32) -> BlockId {
        let Some((section, local_y)) = split_y(y).filter(|_| x < 16 && z < 16) else {
            return BlockId::AIR;
        };
        self.sections[section].get(x, local_y, z)
    }

    /// Ignored outside the column.
    pub fn set(&mut self, x: u32, y: i32, z: u32, block: BlockId) {
        if let Some((section, local_y)) = split_y(y).filter(|_| x < 16 && z < 16) {
            self.sections[section].set(x, local_y, z, block);
        }
    }

    pub fn heap_bytes(&self) -> usize {
        self.sections.iter().map(Section::heap_bytes).sum()
    }
}

fn split_y(y: i32) -> Option<(usize, u32)> {
    let in_range = (0..SECTION_SIZE * SECTIONS_PER_COLUMN).contains(&y);
    in_range.then_some(((y / SECTION_SIZE) as usize, (y % SECTION_SIZE) as u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heights_map_to_sections() {
        let mut column = Column::empty(ColumnPos { x: 0, z: 0 });
        column.set(1, 0, 1, BlockId(1));
        column.set(1, 511, 1, BlockId(2));
        column.set(1, 512, 1, BlockId(3));
        assert_eq!(column.get(1, 0, 1), BlockId(1));
        assert_eq!(column.get(1, 511, 1), BlockId(2));
        assert_eq!(column.get(1, 512, 1), BlockId::AIR);
        assert!(!column.section(31).is_some_and(Section::is_empty));
    }
}
