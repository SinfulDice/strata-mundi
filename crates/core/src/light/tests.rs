use super::*;
use crate::content::Block;

const AIR: BlockId = BlockId::AIR;
const STONE: BlockId = BlockId(1);
const TORCH: BlockId = BlockId(2);
const WATER: BlockId = BlockId(3);
const GLOWSTONE: BlockId = BlockId(4);
const LEAVES: BlockId = BlockId(5);
const GROUND: i32 = 60;

fn table() -> LightTable {
    let block = |opacity, emission| Block {
        light_opacity: opacity,
        light_emission: emission,
        ..Block::air()
    };
    let blocks = [
        Block::air(),
        block(15, 0),
        block(0, 14),
        block(2, 0),
        block(15, 15),
        block(1, 0),
    ];
    LightTable::new(blocks.iter())
}

type World = crate::coords::ColumnMap<Column>;

/// Stone up to `GROUND` over the 3×3 columns around the origin column.
fn ground() -> World {
    let mut world = World::default();
    for cz in -1..=1 {
        for cx in -1..=1 {
            let pos = ColumnPos { x: cx, z: cz };
            let mut column = Column::empty(pos);
            for y in 0..=GROUND {
                (0..16).for_each(|x| (0..16).for_each(|z| column.set(x, y, z, STONE)));
            }
            world.insert(pos, column);
        }
    }
    world
}

fn set(world: &mut World, pos: BlockPos, block: BlockId) -> BlockId {
    let local = |c: i32| c.rem_euclid(SECTION_SIZE) as u32;
    let Some(column) = world.get_mut(&pos.column()) else {
        return AIR;
    };
    let old = column.get(local(pos.x), pos.y, local(pos.z));
    column.set(local(pos.x), pos.y, local(pos.z), block);
    old
}

fn lit(world: &World, order: &[ColumnPos]) -> Lighting {
    let mut lighting = Lighting::default();
    for &pos in order {
        assert!(lighting.light_column(world, &table(), pos));
    }
    lighting
}

fn all_columns() -> Vec<ColumnPos> {
    (-1..=1)
        .flat_map(|z| (-1..=1).map(move |x| ColumnPos { x, z }))
        .collect()
}

fn lit_all(world: &World) -> Lighting {
    lit(world, &all_columns())
}

/// Every light value of the lit columns, whatever their storage form.
fn snapshot(lighting: &Lighting) -> Vec<u8> {
    let mut cells = Vec::new();
    for pos in all_columns() {
        for y in 0..WORLD_HEIGHT {
            for z in 0..16 {
                for x in 0..16 {
                    let at = BlockPos::new(pos.x * 16 + x, y, pos.z * 16 + z);
                    cells.push(lighting.light_at(at).map_or(255, |light| light.0));
                }
            }
        }
    }
    cells
}

fn change(lighting: &mut Lighting, world: &mut World, pos: BlockPos, block: BlockId) {
    let old = set(world, pos, block);
    lighting.block_changed(world, &table(), pos, old);
}

fn at(lighting: &Lighting, x: i32, y: i32, z: i32) -> PackedLight {
    lighting
        .light_at(BlockPos::new(x, y, z))
        .unwrap_or(PackedLight(255))
}

/// A closed room inside the stone, from (2, 40, 2) to (12, 46, 12) of the origin column,
/// one block past its east side so it reaches the neighbouring column.
fn cave(world: &mut World) {
    for y in 40..=46 {
        for z in 2..=12 {
            for x in 2..=17 {
                set(world, BlockPos::new(x, y, z), AIR);
            }
        }
    }
}

#[test]
fn open_sky_is_full_and_rock_is_dark() {
    let lighting = lit_all(&ground());
    assert_eq!(at(&lighting, 3, GROUND + 1, 3), PackedLight::OPEN_SKY);
    assert_eq!(at(&lighting, 3, 400, 3), PackedLight::OPEN_SKY);
    assert_eq!(at(&lighting, 3, GROUND, 3), PackedLight::DARK);
    assert_eq!(at(&lighting, 3, 5, 3), PackedLight::DARK);
}

#[test]
fn placing_then_breaking_a_torch_restores_every_value() {
    let mut world = ground();
    cave(&mut world);
    let mut lighting = lit_all(&world);
    let before = snapshot(&lighting);
    let torch = BlockPos::new(6, 41, 6);
    change(&mut lighting, &mut world, torch, TORCH);
    assert_eq!(at(&lighting, 6, 41, 6).block(), 14);
    assert_eq!(at(&lighting, 8, 41, 6).block(), 12);
    assert_eq!(at(&lighting, 6, 41, 6).sky(), 0);
    change(&mut lighting, &mut world, torch, AIR);
    assert_eq!(snapshot(&lighting), before);
}

#[test]
fn a_torch_on_a_column_border_lights_both_columns_and_leaves_no_trace() {
    let mut world = ground();
    cave(&mut world);
    let mut lighting = lit_all(&world);
    let before = snapshot(&lighting);
    let torch = BlockPos::new(15, 42, 7);
    change(&mut lighting, &mut world, torch, TORCH);
    assert_eq!(at(&lighting, 16, 42, 7).block(), 13);
    assert_eq!(at(&lighting, 17, 42, 7).block(), 12);
    assert_eq!(at(&lighting, 18, 42, 7).block(), 0, "behind the cave wall");
    assert!(
        lighting
            .take_changed()
            .contains(&(ColumnPos { x: 1, z: 0 }, 2))
    );
    change(&mut lighting, &mut world, torch, AIR);
    assert_eq!(snapshot(&lighting), before);
}

#[test]
fn a_lamp_lit_before_its_neighbour_reaches_it() {
    let mut world = ground();
    cave(&mut world);
    set(&mut world, BlockPos::new(15, 42, 7), TORCH);
    let east = ColumnPos { x: 1, z: 0 };
    let origin = ColumnPos { x: 0, z: 0 };
    let rest: Vec<_> = all_columns()
        .into_iter()
        .filter(|pos| *pos != east && *pos != origin)
        .collect();
    let first = lit(&world, &[rest.clone(), vec![origin, east]].concat());
    let second = lit(&world, &[vec![east], rest, vec![origin]].concat());
    assert_eq!(at(&first, 17, 42, 7).block(), 12);
    assert_eq!(snapshot(&first), snapshot(&second));
}

#[test]
fn an_overhang_casts_a_soft_shadow() {
    let mut world = ground();
    for z in 0..10 {
        for x in 0..10 {
            set(&mut world, BlockPos::new(x, GROUND + 6, z), STONE);
        }
    }
    let lighting = lit_all(&world);
    let edge = at(&lighting, 0, GROUND + 2, 5).sky();
    let middle = at(&lighting, 5, GROUND + 2, 5).sky();
    assert_eq!(edge, 14, "next to the open sky");
    assert!(middle < edge && middle > 0, "diffused, got {middle}");
    assert_eq!(at(&lighting, 5, GROUND + 7, 5).sky(), 15);
}

#[test]
fn water_and_leaves_dim_the_sky() {
    let mut world = ground();
    for y in GROUND - 4..=GROUND {
        set(&mut world, BlockPos::new(5, y, 5), WATER);
    }
    set(&mut world, BlockPos::new(9, GROUND + 3, 9), LEAVES);
    let lighting = lit_all(&world);
    assert_eq!(at(&lighting, 5, GROUND, 5).sky(), 13);
    assert_eq!(at(&lighting, 5, GROUND - 1, 5).sky(), 10);
    assert_eq!(at(&lighting, 9, GROUND + 3, 9).sky(), 14);
    assert_eq!(
        at(&lighting, 9, GROUND + 2, 9).sky(),
        14,
        "lit from the side"
    );
}

#[test]
fn breaking_the_ceiling_lights_the_cave() {
    let mut world = ground();
    cave(&mut world);
    let mut lighting = lit_all(&world);
    assert_eq!(at(&lighting, 6, 44, 6).sky(), 0);
    for y in 47..=GROUND {
        change(&mut lighting, &mut world, BlockPos::new(6, y, 6), AIR);
    }
    assert_eq!(at(&lighting, 6, 44, 6).sky(), 15, "straight under the hole");
    assert_eq!(at(&lighting, 8, 44, 6).sky(), 13);
    assert_eq!(snapshot(&lighting), snapshot(&lit_all(&world)));
    for y in (47..=GROUND).rev() {
        change(&mut lighting, &mut world, BlockPos::new(6, y, 6), STONE);
    }
    assert_eq!(at(&lighting, 6, 44, 6).sky(), 0);
}

#[test]
fn placing_a_block_in_the_open_shades_below_it() {
    let mut world = ground();
    let mut lighting = lit_all(&world);
    let before = snapshot(&lighting);
    change(&mut lighting, &mut world, BlockPos::new(4, 200, 4), STONE);
    assert_eq!(at(&lighting, 4, GROUND + 1, 4).sky(), 14);
    assert_eq!(snapshot(&lighting), snapshot(&lit_all(&world)));
    change(&mut lighting, &mut world, BlockPos::new(4, 200, 4), AIR);
    assert_eq!(snapshot(&lighting), before);
}

#[test]
fn many_edits_match_a_fresh_computation() {
    let mut world = ground();
    cave(&mut world);
    let mut lighting = lit_all(&world);
    let mut seed = 7u32;
    let mut next = |range: i32| {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as i32 % range
    };
    let blocks = [AIR, STONE, TORCH, WATER, GLOWSTONE, LEAVES];
    for _ in 0..120 {
        let pos = BlockPos::new(next(40) - 12, 36 + next(30), next(40) - 12);
        let block = blocks[next(blocks.len() as i32) as usize];
        change(&mut lighting, &mut world, pos, block);
    }
    assert_eq!(snapshot(&lighting), snapshot(&lit_all(&world)));
}

#[test]
fn unlit_columns_are_left_alone() {
    let world = ground();
    let mut lighting = Lighting::default();
    assert!(lighting.light_column(&world, &table(), ColumnPos { x: 0, z: 0 }));
    assert!(!lighting.light_column(&world, &table(), ColumnPos { x: 5, z: 0 }));
    assert_eq!(lighting.light_at(BlockPos::new(-1, 70, 0)), None);
    lighting.remove_column(ColumnPos { x: 0, z: 0 });
    assert_eq!(lighting.column_count(), 0);
}

#[test]
fn rock_and_sky_sections_take_no_memory() {
    let lighting = lit_all(&ground());
    let column = lighting.column(ColumnPos { x: 0, z: 0 });
    let mixed = column.map_or(0, |c| {
        c.sections()
            .iter()
            .filter(|s| matches!(s, SectionLight::Mixed(_)))
            .count()
    });
    assert_eq!(mixed, 1, "only the section holding the surface");
}

#[test]
fn an_edit_beside_an_unlit_neighbour_then_lighting_it_matches_a_fresh_computation() {
    let mut world = ground();
    cave(&mut world);
    let east = ColumnPos { x: 1, z: 0 };
    let others: Vec<_> = all_columns().into_iter().filter(|p| *p != east).collect();
    let mut lighting = lit(&world, &others);
    change(&mut lighting, &mut world, BlockPos::new(15, 42, 7), TORCH);
    change(&mut lighting, &mut world, BlockPos::new(15, 38, 7), AIR);
    change(&mut lighting, &mut world, BlockPos::new(15, 44, 3), STONE);
    assert!(lighting.light_column(&world, &table(), east));
    assert_eq!(snapshot(&lighting), snapshot(&lit_all(&world)));
}
