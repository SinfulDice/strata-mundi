//! Voxel light, like Minecraft's: sky light (0-15) that falls straight down through clear
//! blocks then spreads, and block light flooding out of lamps. Light is derived from the
//! blocks, never saved: whoever holds blocks can compute it (the client for drawing, the
//! server later for monster spawning).

mod column;
mod propagate;
mod storage;
#[cfg(test)]
mod tests;

pub use storage::{Channel, ColumnLight, MAX_LIGHT, PackedLight, SectionLight, section_index};

use crate::chunk::Column;
use crate::content::{Block, BlockId, Content, Face};
use crate::coords::{BlockPos, ColumnHasher, ColumnMap, ColumnPos, SECTION_SIZE, WORLD_HEIGHT};
use std::borrow::Borrow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{BuildHasher, BuildHasherDefault};

/// A section of a column, by index from the bottom.
pub type LightSection = (ColumnPos, usize);

/// Light properties of every block, indexed by `BlockId`.
#[derive(Debug, Clone, Default)]
pub struct LightTable {
    opacity: Vec<u8>,
    emission: Vec<u8>,
}

impl LightTable {
    pub fn new<'a>(blocks: impl Iterator<Item = &'a Block>) -> Self {
        let (opacity, emission) = blocks
            .map(|block| (block.light_opacity, block.light_emission))
            .unzip();
        Self { opacity, emission }
    }

    pub fn from_content(content: &Content) -> Self {
        Self::new(content.blocks.iter().map(|(_, _, block)| block))
    }

    /// Unknown blocks stop all light.
    #[inline]
    pub fn opacity(&self, id: BlockId) -> u8 {
        self.opacity
            .get(usize::from(id.0))
            .copied()
            .unwrap_or(MAX_LIGHT)
    }

    #[inline]
    pub fn emission(&self, id: BlockId) -> u8 {
        self.emission.get(usize::from(id.0)).copied().unwrap_or(0)
    }

    fn changes_light(&self, channel: Channel, old: BlockId, new: BlockId) -> bool {
        self.opacity(old) != self.opacity(new)
            || (channel == Channel::Block && self.emission(old) != self.emission(new))
    }
}

/// Where the light engine reads blocks.
pub trait BlockSource {
    fn column(&self, pos: ColumnPos) -> Option<&Column>;
}

impl<C: Borrow<Column>, S: BuildHasher> BlockSource for HashMap<ColumnPos, C, S> {
    fn column(&self, pos: ColumnPos) -> Option<&Column> {
        self.get(&pos).map(Borrow::borrow)
    }
}

/// Light reaching a neighbour from a block lit at `level`, through the face `toward`.
/// Only full sky light goes down without loss (`15 - opacity`); otherwise each block
/// costs one level plus its opacity.
#[inline]
pub fn attenuate(channel: Channel, level: u8, toward: Face, opacity: u8) -> u8 {
    if channel == Channel::Sky && toward == Face::Down && level == MAX_LIGHT {
        MAX_LIGHT.saturating_sub(opacity)
    } else {
        level.saturating_sub(1 + opacity)
    }
}

/// Light of the lit columns, and what changed since it was last asked.
#[derive(Debug, Default)]
pub struct Lighting {
    columns: ColumnMap<ColumnLight>,
    changed: HashSet<LightSection, BuildHasherDefault<ColumnHasher>>,
    last_marked: Option<LightSection>,
    queue: VecDeque<BlockPos>,
    removals: VecDeque<(BlockPos, u8)>,
}

impl Lighting {
    pub fn is_lit(&self, pos: ColumnPos) -> bool {
        self.columns.contains_key(&pos)
    }

    pub fn column(&self, pos: ColumnPos) -> Option<&ColumnLight> {
        self.columns.get(&pos)
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    /// `None` outside the world height or in a column not lit yet.
    pub fn light_at(&self, pos: BlockPos) -> Option<PackedLight> {
        if !pos.is_in_height_range() {
            return None;
        }
        let (index, cell) = locate(pos);
        Some(self.columns.get(&pos.column())?.section(index)?.get(cell))
    }

    pub fn remove_column(&mut self, pos: ColumnPos) {
        self.columns.remove(&pos);
        self.changed.retain(|(column, _)| *column != pos);
        self.last_marked = None;
    }

    /// Sections of lit columns whose light, or the light of their one-block border,
    /// changed since the last call: the ones to mesh again.
    pub fn take_changed(&mut self) -> Vec<LightSection> {
        self.last_marked = None;
        let columns = &self.columns;
        let mut changed: Vec<_> = self
            .changed
            .drain()
            .filter(|(column, _)| columns.contains_key(column))
            .collect();
        changed.sort_unstable_by_key(|(column, index)| (column.x, column.z, *index));
        changed
    }

    pub fn heap_bytes(&self) -> usize {
        self.columns.values().map(ColumnLight::heap_bytes).sum()
    }

    fn write(&mut self, pos: BlockPos, light: PackedLight) {
        let (index, cell) = locate(pos);
        let changed = self
            .columns
            .get_mut(&pos.column())
            .and_then(|column| column.section_mut(index))
            .is_some_and(|section| section.set(cell, light));
        if changed {
            self.mark(pos);
        }
    }

    /// Marks the section of `pos` and every section whose border holds it.
    fn mark(&mut self, pos: BlockPos) {
        let [lx, ly, lz] = [pos.x, pos.y, pos.z].map(|c| c.rem_euclid(SECTION_SIZE));
        let own = (pos.column(), (pos.y / SECTION_SIZE) as usize);
        if [lx, ly, lz]
            .iter()
            .all(|l| (1..SECTION_SIZE - 1).contains(l))
        {
            if self.last_marked != Some(own) {
                self.last_marked = Some(own);
                self.changed.insert(own);
            }
            return;
        }
        for dy in border_offsets(ly) {
            let y = pos.y + dy;
            if !(0..WORLD_HEIGHT).contains(&y) {
                continue;
            }
            for dz in border_offsets(lz) {
                for dx in border_offsets(lx) {
                    let near = BlockPos::new(pos.x + dx, y, pos.z + dz);
                    self.changed
                        .insert((near.column(), (y / SECTION_SIZE) as usize));
                }
            }
        }
    }
}

fn border_offsets(local: i32) -> std::ops::RangeInclusive<i32> {
    match local {
        0 => -1..=0,
        l if l == SECTION_SIZE - 1 => 0..=1,
        _ => 0..=0,
    }
}

/// Section index and cell index of a position inside the world height.
#[inline]
fn locate(pos: BlockPos) -> (usize, usize) {
    let local = |c: i32| c.rem_euclid(SECTION_SIZE) as u32;
    let cell = section_index(local(pos.x), local(pos.y), local(pos.z));
    ((pos.y / SECTION_SIZE) as usize, cell)
}

/// Block at `pos`, `None` where no column is loaded.
#[inline]
fn block_at(blocks: &impl BlockSource, pos: BlockPos) -> Option<BlockId> {
    let column = blocks.column(pos.column())?;
    let local = |c: i32| c.rem_euclid(SECTION_SIZE) as u32;
    Some(column.get(local(pos.x), pos.y, local(pos.z)))
}

/// Reads blocks remembering the last column: light mostly moves inside one column, so
/// most lookups skip the column map.
struct BlockCursor<'a, B: BlockSource> {
    blocks: &'a B,
    last: Option<(ColumnPos, &'a Column)>,
}

impl<'a, B: BlockSource> BlockCursor<'a, B> {
    fn new(blocks: &'a B) -> Self {
        Self { blocks, last: None }
    }

    /// Block at `pos`, `None` where no column is loaded.
    #[inline]
    fn get(&mut self, pos: BlockPos) -> Option<BlockId> {
        let at = pos.column();
        let column = match self.last {
            Some((last, column)) if last == at => column,
            _ => {
                let column = self.blocks.column(at)?;
                self.last = Some((at, column));
                column
            }
        };
        let local = |c: i32| c.rem_euclid(SECTION_SIZE) as u32;
        Some(column.get(local(pos.x), pos.y, local(pos.z)))
    }
}

/// Light a block makes by itself: its emission, or the open sky above the top layer.
fn source(channel: Channel, pos: BlockPos, block: BlockId, table: &LightTable) -> u8 {
    match channel {
        Channel::Block => table.emission(block),
        Channel::Sky if pos.y == WORLD_HEIGHT - 1 => MAX_LIGHT.saturating_sub(table.opacity(block)),
        Channel::Sky => 0,
    }
}
