//! The strongest heat around a body: in a stone cave with nothing warm (every section
//! skipped by its palette), and in a room lit by torches with a lava pool and furnaces
//! (every section walked, a wall checked).

use criterion::{Criterion, criterion_group, criterion_main};
use glam::DVec3;
use std::collections::HashMap;
use std::hint::black_box;
use std::path::Path;
use strata_core::chunk::Column;
use strata_core::content::{BlockId, Content};
use strata_core::coords::{BlockPos, ColumnPos};
use strata_core::pack::PackSet;
use strata_core::survival::temperature::strongest_heat;

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Content::load(&packs, &PackSet::default()).0
}

fn block(content: &Content, name: &str) -> BlockId {
    let id = name.parse().ok();
    id.and_then(|id| content.blocks.key(&id))
        .unwrap_or(BlockId::AIR)
}

/// 3 × 3 columns of stone up to y 80, with a hollow 9-block room at y 64-70.
fn cave(content: &Content) -> HashMap<ColumnPos, Column> {
    let stone = block(content, "strata:stone");
    let mut columns = HashMap::new();
    for cx in -1..=1 {
        for cz in -1..=1 {
            let pos = ColumnPos { x: cx, z: cz };
            let mut column = Column::empty(pos);
            for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
                for y in 0..80 {
                    let (wx, wz) = (cx * 16 + x as i32, cz * 16 + z as i32);
                    let room = wx.abs() <= 4 && wz.abs() <= 4 && (64..=70).contains(&y);
                    if !room {
                        column.set(x, y, z, stone);
                    }
                }
            }
            columns.insert(pos, column);
        }
    }
    columns
}

fn set(columns: &mut HashMap<ColumnPos, Column>, pos: BlockPos, block: BlockId) {
    if let Some(column) = columns.get_mut(&pos.column()) {
        column.set(
            pos.x.rem_euclid(16) as u32,
            pos.y,
            pos.z.rem_euclid(16) as u32,
            block,
        );
    }
}

fn heat(c: &mut Criterion) {
    let content = content();
    let survival = &content.survival;
    let mut columns = cave(&content);
    let centre = DVec3::new(0.5, 64.9, 0.5);
    let solid = |columns: &HashMap<ColumnPos, Column>, pos: BlockPos| {
        let column = columns.get(&pos.column());
        let block = column.map(|c| {
            c.get(
                pos.x.rem_euclid(16) as u32,
                pos.y,
                pos.z.rem_euclid(16) as u32,
            )
        });
        block.is_some_and(|b| content.blocks.get(b).is_some_and(|b| b.solid))
    };
    let mut group = c.benchmark_group("temperature");
    let mut run = |name: &str, columns: &HashMap<ColumnPos, Column>| {
        group.bench_function(name, |b| {
            b.iter(|| {
                strongest_heat(
                    &survival.heat,
                    survival.temperature.walls,
                    black_box(centre),
                    |pos| columns.get(&pos),
                    |pos| solid(columns, pos),
                )
            });
        });
    };
    run("heat_near_cold_cave", &columns);
    let torch = block(&content, "strata:torch");
    for (x, z) in [(-4, -4), (4, -4), (-4, 4), (4, 4)] {
        set(&mut columns, BlockPos::new(x, 66, z), torch);
    }
    let lava = block(&content, "strata:lava");
    for (x, z) in (-6..=-5).flat_map(|x| (-1..=1).map(move |z| (x, z))) {
        set(&mut columns, BlockPos::new(x, 64, z), lava);
    }
    let furnace = block(&content, "strata:furnace");
    let lit = content
        .blocks
        .get(furnace)
        .and_then(|b| b.facing)
        .map_or(furnace, |f| f.lit(true));
    set(&mut columns, BlockPos::new(3, 64, 0), lit);
    run("heat_near_warm_room", &columns);
    group.finish();
}

criterion_group!(benches, heat);
criterion_main!(benches);
