//! Finding the strongest heat around a body: only the sections whose palette holds a
//! source are walked, then the source kept is checked for a wall in between.

use super::heat::HeatSources;
use crate::chunk::Column;
use crate::coords::{BlockPos, ColumnPos, SECTION_SIZE, SECTIONS_PER_COLUMN};
use glam::DVec3;

/// The source felt most, where it is and the degrees it gives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatFound {
    pub source: BlockPos,
    pub warmth: f32,
}

/// The strongest heat felt at `centre`. `column` gives the loaded columns; `solid` says
/// whether a block stands between (the source kept gives only `walls` of its heat then).
pub fn strongest_heat<'c>(
    sources: &HeatSources,
    walls: f32,
    centre: DVec3,
    column: impl Fn(ColumnPos) -> Option<&'c Column>,
    solid: impl Fn(BlockPos) -> bool,
) -> Option<HeatFound> {
    let reach = i32::from(sources.reach);
    if reach == 0 {
        return None;
    }
    let middle = cell(centre);
    let min = BlockPos::new(middle.x - reach, middle.y - reach, middle.z - reach);
    let max = BlockPos::new(middle.x + reach, middle.y + reach, middle.z + reach);
    let mut best: Option<HeatFound> = None;
    for cx in min.x.div_euclid(SECTION_SIZE)..=max.x.div_euclid(SECTION_SIZE) {
        for cz in min.z.div_euclid(SECTION_SIZE)..=max.z.div_euclid(SECTION_SIZE) {
            let Some(column) = column(ColumnPos { x: cx, z: cz }) else {
                continue;
            };
            scan_column(sources, column, (min, max), centre, &mut best);
        }
    }
    best.map(|found| {
        if blocked(centre, found.source, &solid) {
            HeatFound {
                warmth: found.warmth * walls,
                ..found
            }
        } else {
            found
        }
    })
}

fn scan_column(
    sources: &HeatSources,
    column: &Column,
    (min, max): (BlockPos, BlockPos),
    centre: DVec3,
    best: &mut Option<HeatFound>,
) {
    let (base_x, base_z) = (column.pos.x * SECTION_SIZE, column.pos.z * SECTION_SIZE);
    let first = min.y.div_euclid(SECTION_SIZE).max(0);
    let last = max.y.div_euclid(SECTION_SIZE).min(SECTIONS_PER_COLUMN - 1);
    for index in first..=last {
        let Some(section) = column.section(index as usize) else {
            continue;
        };
        if !section.palette().iter().any(|&b| sources.get(b).is_some()) {
            continue;
        }
        let base_y = index * SECTION_SIZE;
        let span =
            |low: i32, high: i32, base: i32| low.max(base)..=high.min(base + SECTION_SIZE - 1);
        for x in span(min.x, max.x, base_x) {
            for y in span(min.y, max.y, base_y) {
                for z in span(min.z, max.z, base_z) {
                    let local = (
                        (x - base_x) as u32,
                        (y - base_y) as u32,
                        (z - base_z) as u32,
                    );
                    let Some(source) = sources.get(section.get(local.0, local.1, local.2)) else {
                        continue;
                    };
                    let pos = BlockPos::new(x, y, z);
                    let distance = (centre_of(pos) - centre).length() as f32;
                    let warmth = source.at((distance - 0.5).max(0.0));
                    if warmth > best.map_or(0.0, |b| b.warmth) {
                        *best = Some(HeatFound {
                            source: pos,
                            warmth,
                        });
                    }
                }
            }
        }
    }
}

/// A solid block on the straight line from `from` to the centre of `source`.
fn blocked(from: DVec3, source: BlockPos, solid: &impl Fn(BlockPos) -> bool) -> bool {
    let to = centre_of(source);
    let steps = ((to - from).length() * 4.0).ceil().max(1.0) as u32;
    let start = cell(from);
    let mut last = start;
    (1..steps).any(|step| {
        let at = cell(from + (to - from) * (f64::from(step) / f64::from(steps)));
        let new = at != last && at != start && at != source;
        last = at;
        new && solid(at)
    })
}

fn cell(point: DVec3) -> BlockPos {
    BlockPos::new(
        point.x.floor() as i32,
        point.y.floor() as i32,
        point.z.floor() as i32,
    )
}

fn centre_of(pos: BlockPos) -> DVec3 {
    DVec3::new(
        f64::from(pos.x) + 0.5,
        f64::from(pos.y) + 0.5,
        f64::from(pos.z) + 0.5,
    )
}
