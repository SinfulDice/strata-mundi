use super::*;
use crate::mesher::{BlockTable, MeshOptions};
use std::collections::HashSet;
use strata_core::chunk::BiomeId;
use strata_core::content::{Block, RenderKind};
use strata_core::light::{LightTable, PackedLight, SectionLight};

fn view() -> WorldView {
    let solid = Block {
        render: RenderKind::Opaque,
        ..Block::air()
    };
    let blocks = [Block::air(), solid];
    let table = Arc::new(BlockTable::new(blocks.iter()));
    let meshing = MeshScheduler::new(table, 1, MeshOptions::default()).unwrap();
    WorldView::new(
        ClientWorld::default(),
        meshing,
        LightTable::new(blocks.iter()),
        Lighting::Full,
    )
    .unwrap()
}

/// 5×5 columns, each with blocks in sections 0 and 2.
fn loaded() -> WorldView {
    let mut view = view();
    for cz in -2..=2 {
        for cx in -2..=2 {
            let pos = ColumnPos { x: cx, z: cz };
            let mut column = Column::empty(pos);
            column.set(0, 0, 0, BlockId(1));
            column.set(0, 40, 0, BlockId(1));
            view.insert(column, BiomeMap::uniform(BiomeId(0)));
        }
    }
    view
}

fn lit(positions: impl IntoIterator<Item = ColumnPos>) -> LightResult {
    LightResult {
        lit: positions.into_iter().collect(),
        ..LightResult::default()
    }
}

const CENTRE: ColumnPos = ColumnPos { x: 0, z: 0 };

#[test]
fn a_section_waits_for_the_light_of_its_column_and_neighbours() {
    let mut view = loaded();
    assert!(view.meshing.take_dirty().is_empty(), "nothing lit yet");
    view.apply_light(lit(around(CENTRE)));
    assert!(
        view.meshing.take_dirty().is_empty(),
        "the centre is not lit"
    );
    view.apply_light(lit([CENTRE]));
    let dirty = view.meshing.take_dirty();
    let expected: HashSet<_> = [(0, 0, 0), (0, 0, 2)].into();
    assert_eq!(dirty, expected, "only the centre has its 8 neighbours lit");
}

#[test]
fn a_light_change_meshes_exactly_the_reported_sections() {
    let mut view = loaded();
    view.apply_light(lit(around(CENTRE).chain([CENTRE])));
    view.meshing.take_dirty();
    let light = SectionLight::Uniform(PackedLight::new(3, 9));
    let sections = vec![
        (CENTRE, 2, light.clone()),
        (CENTRE, 5, light.clone()),
        (ColumnPos { x: 1, z: 0 }, 0, light.clone()),
    ];
    view.apply_light(LightResult {
        sections,
        ..LightResult::default()
    });
    let expected: HashSet<_> = [(0, 0, 2)].into();
    assert_eq!(
        view.meshing.take_dirty(),
        expected,
        "empty sections and columns without lit neighbours wait"
    );
    let cell = BlockPos::new(3, 35, 3);
    assert_eq!(view.world.light_at(cell), PackedLight::new(3, 9));
}

#[test]
fn flat_light_meshes_at_once_and_switching_back_waits_for_the_thread() {
    let mut view = loaded();
    view.set_lighting(Lighting::Flat);
    let dirty = view.meshing.take_dirty();
    assert!(dirty.contains(&(0, 0, 0)) && dirty.contains(&(1, 1, 2)));
    assert!(
        !dirty.iter().any(|key| key.0.abs() == 2),
        "edges lack neighbours"
    );
    let mut column = Column::empty(CENTRE);
    column.set(0, 0, 0, BlockId(1));
    view.insert(column, BiomeMap::uniform(BiomeId(0)));
    assert!(view.world.is_lit(CENTRE), "lit as soon as it arrives");
    assert!(view.meshing.take_dirty().contains(&(0, 0, 0)));
    view.set_lighting(Lighting::Full);
    assert!(!view.world.is_lit(CENTRE), "lit again by the thread");
    assert!(view.lighting.is_some());
}

#[test]
fn a_broken_block_is_meshed_with_a_guessed_light() {
    let mut view = loaded();
    view.apply_light(lit(around(CENTRE).chain([CENTRE])));
    let dark = SectionLight::Uniform(PackedLight::DARK);
    view.world.set_section_light(CENTRE, 0, dark);
    view.meshing.take_dirty();
    view.set_block(BlockPos::new(0, 15, 0), BlockId::AIR);
    assert!(view.meshing.take_dirty().contains(&(0, 0, 0)));
    assert_eq!(
        view.world.light_at(BlockPos::new(0, 15, 0)),
        PackedLight::OPEN_SKY
    );
    assert_eq!(
        view.world.light_at(BlockPos::new(0, 14, 0)),
        PackedLight::DARK
    );
}
