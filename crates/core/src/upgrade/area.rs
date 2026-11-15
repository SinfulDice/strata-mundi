//! A miner's hammer breaks an area across the face hit: on a wall, `width` across and
//! `height` up, centred on the block hit; on a floor or a ceiling, `width` across the
//! player's facing and `height` ahead. Only the blocks of the tool's kinds that it
//! harvests break with the one hit, and the break takes as long as the hardest.

use super::defs::AreaSize;
use crate::content::{Block, Face, ToolDef};
use crate::coords::BlockPos;

/// The cells of the area around `center`, `center` first.
pub fn area_cells(center: BlockPos, face: Face, area: AreaSize, yaw: f32) -> Vec<BlockPos> {
    let (across, along) = axes(face, yaw);
    let mut cells = vec![center];
    for a in span(area.width) {
        for b in span(area.height) {
            if (a, b) == (0, 0) {
                continue;
            }
            cells.push(BlockPos::new(
                center.x + across[0] * a + along[0] * b,
                center.y + across[1] * a + along[1] * b,
                center.z + across[2] * a + along[2] * b,
            ));
        }
    }
    cells
}

/// The lowest and highest cells of the area, without listing its cells (drawn every
/// frame).
pub fn area_bounds(center: BlockPos, face: Face, area: AreaSize, yaw: f32) -> (BlockPos, BlockPos) {
    let (across, along) = axes(face, yaw);
    let (wide, high) = (span(area.width), span(area.height));
    let ends = |range: &std::ops::Range<i32>| [range.start, range.end - 1];
    let corners = ends(&wide).into_iter().flat_map(|a| {
        ends(&high).map(|b| {
            BlockPos::new(
                center.x + across[0] * a + along[0] * b,
                center.y + across[1] * a + along[1] * b,
                center.z + across[2] * a + along[2] * b,
            )
        })
    });
    corners.fold((center, center), |(low, high), cell| {
        (
            BlockPos::new(low.x.min(cell.x), low.y.min(cell.y), low.z.min(cell.z)),
            BlockPos::new(high.x.max(cell.x), high.y.max(cell.y), high.z.max(cell.z)),
        )
    })
}

/// Offsets of a side of `size` cells, centred (an even side leans forward).
fn span(size: u8) -> std::ops::Range<i32> {
    let size = i32::from(size.max(1));
    let start = -((size - 1) / 2);
    start..start + size
}

/// The unit steps across and along the area, as `[x, y, z]`.
fn axes(face: Face, yaw: f32) -> ([i32; 3], [i32; 3]) {
    match face {
        Face::North | Face::South => ([1, 0, 0], [0, 1, 0]),
        Face::East | Face::West => ([0, 0, 1], [0, 1, 0]),
        Face::Up | Face::Down => {
            // Forward is (sin yaw, 0, −cos yaw), as the camera's.
            let (sin, cos) = yaw.sin_cos();
            if sin.abs() > cos.abs() {
                ([0, 0, 1], [1, 0, 0])
            } else {
                ([1, 0, 0], [0, 0, 1])
            }
        }
    }
}

/// Whether the hammer's area takes `block` along: a block of one of its kinds that it
/// harvests and can break.
pub fn fits_area(block: &Block, tool: &ToolDef) -> bool {
    let kind = block.tool.as_ref().is_some_and(|t| tool.is(&t.kind));
    kind && block.harvests(Some(tool)) && block.break_ticks(Some(tool)).is_some()
}

/// The cells a hit on `center` breaks besides it: none unless `area` is larger than one
/// block and the block hit is one the tool works on; then those of the area it works on
/// (`blocks` gives the block of a cell, `None` when empty or unknown).
pub fn swing_cells<'a>(
    center: BlockPos,
    face: Face,
    (area, yaw): (AreaSize, f32),
    tool: &ToolDef,
    blocks: impl Fn(BlockPos) -> Option<&'a Block>,
) -> Vec<BlockPos> {
    let fits = |at: BlockPos| blocks(at).is_some_and(|block| fits_area(block, tool));
    if area == AreaSize::SINGLE || !fits(center) {
        return Vec::new();
    }
    area_cells(center, face, area, yaw)
        .into_iter()
        .skip(1)
        .filter(|&at| fits(at))
        .collect()
}

/// Ticks a hit takes: `single` for the block hit, or more for a harder one among `cells`.
pub fn swing_ticks<'a>(
    cells: &[BlockPos],
    single: u32,
    tool: &ToolDef,
    blocks: impl Fn(BlockPos) -> Option<&'a Block>,
) -> u32 {
    cells
        .iter()
        .filter_map(|&pos| blocks(pos)?.break_ticks(Some(tool)))
        .fold(single, u32::max)
}
