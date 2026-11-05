//! First light of a column: sky shafts straight down, lamps, then a flood fill seeded only
//! where light can still move (shaft edges, lamps, borders with lit neighbours).

use super::storage::{ColumnLight, MAX_LIGHT, SectionLight, section_index};
use super::{BlockSource, Channel, LightTable, Lighting, attenuate, block_at};
use crate::chunk::Column;
use crate::content::Face;
use crate::coords::{BlockPos, ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN};

const SIZE: usize = SECTION_SIZE as usize;

/// Lowest height of full sky light at each x/z of a column, by `z * 16 + x`.
type Floors = [i32; SIZE * SIZE];

impl Lighting {
    /// Computes the light of a loaded column from scratch, spreading into and from its lit
    /// neighbours. Sections above the column's highest block are open sky without any work.
    /// Returns false when the column's blocks are not loaded.
    pub fn light_column(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        pos: ColumnPos,
    ) -> bool {
        let Some(column) = blocks.column(pos) else {
            return false;
        };
        let top = column.sections().iter().rposition(|s| !s.is_empty());
        let mut light = ColumnLight::below_open_sky(top);
        let mut lamps = Vec::new();
        if let Some(top) = top {
            let floors = sky_shafts(column, table, top, &mut light, &mut self.queue);
            queue_shaft_edges(pos, &floors, &mut self.queue);
            lamps = light_lamps(column, table, &mut light);
        }
        self.columns.insert(pos, light);
        self.last_marked = None;
        self.changed
            .extend((0..SECTIONS_PER_COLUMN as usize).map(|index| (pos, index)));
        self.exchange_borders(blocks, table, Channel::Sky, pos);
        self.spread(blocks, table, Channel::Sky);
        self.queue.extend(lamps);
        self.exchange_borders(blocks, table, Channel::Block, pos);
        self.spread(blocks, table, Channel::Block);
        if let Some(light) = self.columns.get_mut(&pos) {
            light.compact();
        }
        true
    }

    /// Lets light cross each side shared with a lit neighbour, both ways, queueing every
    /// block that brightened.
    fn exchange_borders(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        channel: Channel,
        pos: ColumnPos,
    ) {
        for face in [Face::North, Face::South, Face::East, Face::West] {
            let (dx, _, dz) = face.offset();
            let other = ColumnPos {
                x: pos.x + dx,
                z: pos.z + dz,
            };
            if !self.is_lit(other) {
                continue;
            }
            for section in 0..SECTIONS_PER_COLUMN as usize {
                if !self.same_uniform(pos, other, section) {
                    self.exchange_section_side(blocks, table, channel, pos, face, section);
                }
            }
        }
    }

    /// Two uniform sections side by side cannot light each other.
    fn same_uniform(&self, a: ColumnPos, b: ColumnPos, section: usize) -> bool {
        let uniform = |pos: ColumnPos| match self.columns.get(&pos)?.section(section)? {
            SectionLight::Uniform(light) => Some(*light),
            SectionLight::Mixed(_) => None,
        };
        matches!((uniform(a), uniform(b)), (Some(x), Some(y)) if x == y)
    }

    fn exchange_section_side(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        channel: Channel,
        pos: ColumnPos,
        face: Face,
        section: usize,
    ) {
        let base = BlockPos::new(
            pos.x * SECTION_SIZE,
            section as i32 * SECTION_SIZE,
            pos.z * SECTION_SIZE,
        );
        let last = SECTION_SIZE - 1;
        for y in 0..SECTION_SIZE {
            for along in 0..SECTION_SIZE {
                let (x, z) = match face {
                    Face::North => (along, 0),
                    Face::South => (along, last),
                    Face::East => (last, along),
                    _ => (0, along),
                };
                let ours = BlockPos::new(base.x + x, base.y + y, base.z + z);
                self.cross(blocks, table, channel, ours, face);
            }
        }
    }

    /// Light moving between `a` and its neighbour through `face`, in whichever direction
    /// brightens something.
    fn cross(
        &mut self,
        blocks: &impl BlockSource,
        table: &LightTable,
        channel: Channel,
        a: BlockPos,
        face: Face,
    ) {
        let b = a.neighbour(face);
        let (Some(light_a), Some(light_b)) = (self.light_at(a), self.light_at(b)) else {
            return;
        };
        let (Some(block_a), Some(block_b)) = (block_at(blocks, a), block_at(blocks, b)) else {
            return;
        };
        let (level_a, level_b) = (light_a.get(channel), light_b.get(channel));
        let into_a = attenuate(channel, level_b, face, table.opacity(block_a));
        let into_b = attenuate(channel, level_a, face, table.opacity(block_b));
        if into_a > level_a {
            self.write(a, light_a.with(channel, into_a));
            self.queue.push_back(a);
        } else if into_b > level_b {
            self.write(b, light_b.with(channel, into_b));
            self.queue.push_back(b);
        }
    }
}

/// Fills the full-sky shafts from the top of section `top` down to the first block that
/// dims the light, which gets the dimmed level and is queued. Returns where each shaft ends.
fn sky_shafts(
    column: &Column,
    table: &LightTable,
    top: usize,
    light: &mut ColumnLight,
    queue: &mut std::collections::VecDeque<BlockPos>,
) -> Floors {
    let mut floors = [0; SIZE * SIZE];
    let origin = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
    for z in 0..SIZE as u32 {
        for x in 0..SIZE as u32 {
            let mut floor = 0;
            for y in (0..(top as i32 + 1) * SECTION_SIZE).rev() {
                let opacity = table.opacity(column.get(x, y, z));
                let level = MAX_LIGHT.saturating_sub(opacity);
                set_sky(light, x, y, z, level);
                if opacity > 0 {
                    if level > 1 {
                        queue.push_back(BlockPos::new(origin.0 + x as i32, y, origin.1 + z as i32));
                    }
                    floor = y + 1;
                    break;
                }
            }
            floors[z as usize * SIZE + x as usize] = floor;
        }
    }
    floors
}

/// Queues the full-sky blocks beside a lower neighbouring shaft floor, which the flood
/// fill can light sideways (cliff edges, cave mouths, under overhangs).
fn queue_shaft_edges(
    pos: ColumnPos,
    floors: &Floors,
    queue: &mut std::collections::VecDeque<BlockPos>,
) {
    let floor = |x: i32, z: i32| {
        ((0..SECTION_SIZE).contains(&x) && (0..SECTION_SIZE).contains(&z))
            .then(|| floors[z as usize * SIZE + x as usize])
    };
    for z in 0..SECTION_SIZE {
        for x in 0..SECTION_SIZE {
            let own = floors[z as usize * SIZE + x as usize];
            let highest = [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .filter_map(|(dx, dz)| floor(x + dx, z + dz))
                .max()
                .unwrap_or(own);
            for y in own..highest {
                queue.push_back(BlockPos::new(
                    pos.x * SECTION_SIZE + x,
                    y,
                    pos.z * SECTION_SIZE + z,
                ));
            }
        }
    }
}

/// Sets the light of every lamp block and returns them, skipping sections without any.
fn light_lamps(column: &Column, table: &LightTable, light: &mut ColumnLight) -> Vec<BlockPos> {
    let mut lamps = Vec::new();
    for (index, section) in column.sections().iter().enumerate() {
        if !section.palette().iter().any(|&id| table.emission(id) > 0) {
            continue;
        }
        let Some(target) = light.section_mut(index) else {
            continue;
        };
        for y in 0..SIZE as u32 {
            for z in 0..SIZE as u32 {
                for x in 0..SIZE as u32 {
                    let emission = table.emission(section.get(x, y, z));
                    if emission == 0 {
                        continue;
                    }
                    let cell = section_index(x, y, z);
                    target.set(cell, target.get(cell).with(Channel::Block, emission));
                    lamps.push(BlockPos::new(
                        column.pos.x * SECTION_SIZE + x as i32,
                        index as i32 * SECTION_SIZE + y as i32,
                        column.pos.z * SECTION_SIZE + z as i32,
                    ));
                }
            }
        }
    }
    lamps
}

fn set_sky(light: &mut ColumnLight, x: u32, y: i32, z: u32, level: u8) {
    let index = (y / SECTION_SIZE) as usize;
    let cell = section_index(x, y.rem_euclid(SECTION_SIZE) as u32, z);
    if let Some(section) = light.section_mut(index) {
        section.set(cell, section.get(cell).with(Channel::Sky, level));
    }
}
