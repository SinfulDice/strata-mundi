use super::*;
use std::path::Path;
use std::sync::Arc;
use strata_core::chunk::{BiomeId, BiomeMap, Column};
use strata_core::content::Content;
use strata_core::coords::ColumnPos;
use strata_core::pack::PackSet;

const DIRT: BlockPos = BlockPos::new(1, 0, 1);
const STONE: BlockPos = BlockPos::new(2, 0, 1);
const POPPY: BlockPos = BlockPos::new(3, 0, 1);
/// A frame of a twentieth of a second: one tick.
const TICK: f32 = 0.05;

struct Scene {
    info: BlockInfo,
    world: ClientWorld,
}

fn scene() -> Scene {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, _) = Content::load(&packs, &PackSet::default());
    let info = BlockInfo::new(Arc::new(content));
    let key = |id: &str| info.content.blocks.key(&id.parse().unwrap()).unwrap();
    let mut column = Column::empty(ColumnPos { x: 0, z: 0 });
    for (pos, name) in [
        (DIRT, "strata:dirt"),
        (STONE, "strata:stone"),
        (POPPY, "strata:poppy"),
    ] {
        column.set(pos.x as u32, pos.y, pos.z as u32, key(name));
    }
    let mut world = ClientWorld::default();
    world.insert(column, BiomeMap::uniform(BiomeId(0)));
    Scene { info, world }
}

fn aim(pos: BlockPos) -> Option<RayHit> {
    Some(RayHit {
        block: pos,
        face: Face::Up,
    })
}

fn frame(mining: &mut Mining, scene: &Scene, target: Option<RayHit>) -> Option<WorldAction> {
    let input = MiningInput {
        world: &scene.world,
        info: &scene.info,
        target,
        held: None,
        ready: true,
    };
    mining.update(TICK, &input)
}

#[test]
fn holding_on_a_block_starts_then_breaks_it_in_time() {
    let scene = scene();
    let mut mining = Mining::default();
    let start = frame(&mut mining, &scene, aim(DIRT));
    assert_eq!(start, Some(WorldAction::StartBreak { pos: DIRT }));
    let ticks = (1..100)
        .find(|_| frame(&mut mining, &scene, aim(DIRT)).is_some())
        .unwrap();
    assert_eq!(ticks, 15, "dirt by hand");
    assert!(!mining.hits.is_empty(), "hit sounds while breaking");
}

#[test]
fn letting_go_or_looking_away_stops() {
    let scene = scene();
    let mut mining = Mining::default();
    frame(&mut mining, &scene, aim(DIRT));
    assert_eq!(
        frame(&mut mining, &scene, None),
        Some(WorldAction::StopBreak)
    );
    assert_eq!(frame(&mut mining, &scene, None), None);
    frame(&mut mining, &scene, aim(DIRT));
    let other = frame(&mut mining, &scene, aim(STONE));
    assert_eq!(other, Some(WorldAction::StartBreak { pos: STONE }));
}

#[test]
fn instant_blocks_break_without_a_start() {
    let scene = scene();
    let mut mining = Mining::default();
    let action = frame(&mut mining, &scene, aim(POPPY));
    assert_eq!(action, Some(WorldAction::Break { pos: POPPY }));
}

#[test]
fn cracks_grow_and_merge_with_reported_ones() {
    let scene = scene();
    let mut mining = Mining::default();
    frame(&mut mining, &scene, aim(STONE));
    assert_eq!(mining.cracks(&scene.world), vec![(STONE, 0)]);
    for _ in 0..75 {
        frame(&mut mining, &scene, aim(STONE));
    }
    let [(pos, half)] = mining.cracks(&scene.world)[..] else {
        panic!("one crack");
    };
    assert_eq!(pos, STONE);
    assert!((126..=127).contains(&half), "3.75 of 7.5 seconds: {half}");
    let other = NetEntity(9);
    mining.report(other, STONE, Some(200));
    mining.report(other, DIRT, Some(30));
    let mut cracks = mining.cracks(&scene.world);
    cracks.sort_by_key(|(pos, _)| pos.x);
    assert_eq!(cracks, vec![(DIRT, 30), (STONE, half)]);
    mining.report(NetEntity(4), STONE, Some(200));
    assert!(mining.cracks(&scene.world).contains(&(STONE, 200)));
    mining.forget(NetEntity(4));
    mining.report(other, DIRT, None);
    mining.report(NetEntity(5), BlockPos::new(5, 5, 5), Some(3));
    assert_eq!(
        mining.cracks(&scene.world),
        vec![(STONE, half)],
        "none on air"
    );
    let draws = mining.draws(&scene.world, &scene.info, DVec3::ZERO);
    assert_eq!(draws.len(), 1);
    assert_eq!(draws[0].min, glam::Vec3::new(2.0, 0.0, 1.0));
    let stages = &scene.info.content.crack_stages;
    assert_eq!(draws[0].layer, u32::from(stages[stages.len() / 2 - 1].0));
    mining.clear();
    assert!(mining.cracks(&scene.world).is_empty());
}
