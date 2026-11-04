#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use common::caves::column_of;
use std::sync::Arc;
use strata_core::chunk::Column;
use strata_core::content::BlockId;
use strata_core::coords::ColumnPos;
use strata_worldgen::{Deposit, Feature, Generator, Seed, Vein};

const SEEDS: [u64; 3] = [42, 7, 99];
/// Half the side of the searched area: 4×4 cells of 256 blocks per deposit kind.
const AREA: i32 = 512;

fn deposit<'a>(generator: &'a Generator, vein: &Vein) -> &'a Deposit {
    match &generator.content().features[vein.feature] {
        Feature::Deposit(deposit) => deposit,
        _ => panic!("veins come from deposits"),
    }
}

fn veins(generator: &Generator) -> Vec<Vein> {
    generator.deposits_near((-AREA, -AREA), (AREA - 1, AREA - 1))
}

fn generator_without_deposits(seed: u64) -> Generator {
    let mut worldgen = (*common::load().1).clone();
    worldgen.terrain.deposits.clear();
    Generator::new(Seed(seed), Arc::new(worldgen))
}

fn center_column(vein: &Vein) -> ColumnPos {
    column_of((vein.center.x, vein.center.z))
}

fn count(column: &Column, wanted: &[BlockId]) -> usize {
    (0..16u32)
        .flat_map(|x| (0..16u32).map(move |z| (x, z)))
        .flat_map(|(x, z)| (1..320).map(move |y| column.get(x, y, z)))
        .filter(|block| wanted.contains(block))
        .count()
}

fn ores(d: &Deposit) -> [BlockId; 2] {
    [d.ore, d.deep_ore]
}

#[test]
fn iron_and_copper_deposits_are_found_on_every_seed() {
    let (content, _) = common::load();
    for seed in SEEDS {
        let generator = common::generator(seed);
        for filler in ["strata:ferrous_rock", "strata:verdigris_rock"] {
            let filler = content.blocks.key(&filler.parse().unwrap()).unwrap();
            let rich = veins(&generator)
                .iter()
                .filter(|vein| deposit(&generator, vein).filler == filler)
                .map(|vein| {
                    count(
                        &generator.generate(center_column(vein)),
                        &ores(deposit(&generator, vein)),
                    )
                })
                .max()
                .unwrap_or(0);
            assert!(
                rich >= 30,
                "seed {seed}: no rich deposit of {filler:?} ({rich} ores)"
            );
        }
    }
}

#[test]
fn deposits_do_not_depend_on_generation_order() {
    let generator = Arc::new(common::generator(42));
    let vein = veins(&generator)[0];
    let center = center_column(&vein);
    let alone = common::generator(42).generate(center);
    let around: Vec<ColumnPos> = (-1..=1)
        .flat_map(|dz| {
            (-1..=1).map(move |dx| ColumnPos {
                x: center.x + dx,
                z: center.z + dz,
            })
        })
        .collect();
    let columns: Vec<Column> = std::thread::scope(|scope| {
        let handles: Vec<_> = around
            .iter()
            .rev()
            .map(|&pos| {
                let generator = Arc::clone(&generator);
                scope.spawn(move || generator.generate(pos))
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    let again = columns.iter().find(|c| c.pos == center).unwrap();
    assert_eq!(&alone, again);
    assert!(count(&alone, &[deposit(&generator, &vein).filler]) > 0);
}

/// Every block a deposit changed was stone or deep stone, or is a sign on open ground.
#[test]
fn deposits_only_replace_stone_and_leave_signs_on_open_ground() {
    for seed in SEEDS {
        let with = common::generator(seed);
        let without = generator_without_deposits(seed);
        let terrain = &with.content().terrain;
        let mut changed = 0;
        for vein in veins(&with).iter().take(4) {
            let d = deposit(&with, vein);
            let parts = [d.ore, d.deep_ore, d.filler, d.deep_filler];
            let pos = center_column(vein);
            let (a, b) = (with.generate(pos), without.generate(pos));
            for (x, z) in (0..16u32).flat_map(|x| (0..16u32).map(move |z| (x, z))) {
                for y in 0..511 {
                    let (now, before) = (a.get(x, y, z), b.get(x, y, z));
                    if now == before {
                        continue;
                    }
                    changed += 1;
                    let rock = before == terrain.stone || before == terrain.deep_stone;
                    let sign = Some(now) == d.sign
                        && d.sign_on.contains(&before)
                        && a.get(x, y + 1, z) == BlockId::AIR;
                    assert!(
                        (rock && parts.contains(&now)) || sign,
                        "seed {seed}: {before:?} became {now:?} at ({x}, {y}, {z}) of {pos:?}"
                    );
                }
            }
        }
        assert!(changed > 0, "seed {seed}: deposits changed nothing");
    }
}

/// Signs on the ground right above deposits, and none far from them.
#[test]
fn signs_lie_above_deposits_only() {
    for seed in SEEDS {
        let generator = common::generator(seed);
        let veins = veins(&generator);
        let signs: Vec<BlockId> = veins
            .iter()
            .filter_map(|v| deposit(&generator, v).sign)
            .collect();
        let above: usize = veins
            .iter()
            .take(6)
            .map(|vein| surface_signs(&generator.generate(center_column(vein)), &signs))
            .sum();
        assert!(above > 0, "seed {seed}: no sign above deposits");
        let far = (0..64)
            .map(|i| ((i % 8) * 128 - AREA + 40, (i / 8) * 128 - AREA + 40))
            .filter(|&(x, z)| {
                veins
                    .iter()
                    .all(|v| (v.center.x - x).abs().max((v.center.z - z).abs()) > 120)
            })
            .take(8)
            .map(|(x, z)| count(&generator.generate(column_of((x, z))), &signs))
            .sum::<usize>();
        assert_eq!(far, 0, "seed {seed}: deposit rock far from any deposit");
    }
}

fn surface_signs(column: &Column, signs: &[BlockId]) -> usize {
    let mut found = 0;
    for (x, z) in (0..16u32).flat_map(|x| (0..16u32).map(move |z| (x, z))) {
        let top = (1..511)
            .rev()
            .find(|&y| column.get(x, y, z) != BlockId::AIR);
        if top.is_some_and(|y| signs.contains(&column.get(x, y, z))) {
            found += 1;
        }
    }
    found
}

/// A vein crossing a column border goes on, block for block, on the other side.
#[test]
fn veins_continue_across_column_borders() {
    let generator = common::generator(42);
    let mut pairs = 0;
    for vein in veins(&generator).iter().take(4) {
        let d = deposit(&generator, vein);
        let parts = [d.ore, d.deep_ore, d.filler, d.deep_filler];
        let center = center_column(vein);
        let here = generator.generate(center);
        let east = generator.generate(ColumnPos {
            x: center.x + 1,
            z: center.z,
        });
        for z in 0..16 {
            for y in 1..320 {
                if parts.contains(&here.get(15, y, z)) && parts.contains(&east.get(0, y, z)) {
                    pairs += 1;
                }
            }
        }
    }
    assert!(pairs > 10, "only {pairs} vein blocks touch across borders");
}
