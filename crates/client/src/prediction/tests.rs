#![allow(clippy::unwrap_used)]

use super::*;
use crate::mesh_scheduler::MeshScheduler;
use crate::mesher::{BlockTable, MeshOptions};
use crate::particles::{ParticleBlock, ParticleSystem, ParticleTable};
use crate::settings::{Lighting, ParticleMode};
use crate::streaming::WorldView;
use glam::DVec3;
use std::path::Path;
use std::sync::Arc;
use strata_core::chunk::{BiomeId, BiomeMap, Column};
use strata_core::content::{
    Block, BlockParticles, Content, Particle, ParticleId, RenderKind, TextureId,
};
use strata_core::light::LightTable;
use strata_core::pack::PackSet;

const AIR: BlockId = BlockId::AIR;
const STONE: BlockId = BlockId(1);
const GROUND: BlockPos = BlockPos::new(0, 0, 0);
const ABOVE: BlockPos = BlockPos::new(0, 1, 0);

fn fragments() -> ParticleTable {
    let stone = ParticleBlock {
        solid: true,
        particles: BlockParticles {
            breaking: Some(ParticleId(0)),
            ..BlockParticles::default()
        },
        faces: [TextureId(0); 6],
        ..ParticleBlock::default()
    };
    let particle = Particle {
        texture: None,
        color: [255; 4],
        lifetime: [60.0, 60.0],
        size: [0.1, 0.1],
        gravity: 0.0,
        speed: 0.0,
        drag: 1.0,
        hang: 0.0,
        collides: false,
        glows: false,
        in_liquid: false,
        burst: 20,
        chance: 1.0,
        impact: None,
    };
    ParticleTable::new(vec![ParticleBlock::default(), stone], vec![particle])
}

/// Air and stone; a stone floor at y = 0 in the 3×3 columns around the origin.
fn view() -> WorldView {
    let solid = Block {
        render: RenderKind::Opaque,
        ..Block::air()
    };
    let blocks = [Block::air(), solid];
    let table = Arc::new(BlockTable::new(blocks.iter()));
    let meshing = MeshScheduler::new(table, 1, MeshOptions::default()).unwrap();
    let light = LightTable::new(blocks.iter());
    let mut view = WorldView::new(ClientWorld::default(), meshing, light, Lighting::Full)
        .unwrap()
        .with_particles(ParticleSystem::new(fragments(), ParticleMode::Full));
    for cz in -1..=1 {
        for cx in -1..=1 {
            let mut column = Column::empty(ColumnPos { x: cx, z: cz });
            (0..16).for_each(|x| (0..16).for_each(|z| column.set(x, 0, z, STONE)));
            view.insert(column, BiomeMap::uniform(BiomeId(0)));
        }
    }
    view
}

#[test]
fn a_predicted_placement_shows_at_once_then_is_confirmed() {
    let mut view = view();
    let id = view.predictions.next_id();
    view.predict(id, ABOVE, STONE);
    assert_eq!(view.world.block_at(ABOVE), STONE, "before any answer");
    assert_eq!(view.block_changes, vec![(ABOVE, AIR, STONE)]);
    view.receive_block(ABOVE, STONE);
    view.acknowledge(id);
    assert_eq!(view.world.block_at(ABOVE), STONE);
    assert_eq!(view.block_changes.len(), 1, "one sound, not two");
}

#[test]
fn a_refused_click_is_undone_without_effects() {
    let mut view = view();
    let id = view.predictions.next_id();
    view.predict(id, ABOVE, STONE);
    view.receive_block(ABOVE, AIR);
    assert_eq!(view.world.block_at(ABOVE), STONE, "kept until the answer");
    view.acknowledge(id);
    assert_eq!(view.world.block_at(ABOVE), AIR);
    assert_eq!(view.block_changes.len(), 1, "undoing plays nothing");
}

#[test]
fn a_predicted_break_throws_its_particles_and_sound_once() {
    let mut view = view();
    let id = view.predictions.next_id();
    view.predict(id, GROUND, AIR);
    assert_eq!(view.world.block_at(GROUND), AIR);
    let thrown = view.particles.len();
    assert!(thrown > 0);
    view.receive_block(GROUND, AIR);
    view.acknowledge(id);
    assert_eq!(view.particles.len(), thrown, "no second burst");
    assert_eq!(view.block_changes, vec![(GROUND, STONE, AIR)]);
}

#[test]
fn a_newer_server_change_wins_and_the_prediction_never_comes_back() {
    let mut view = view();
    let place = view.predictions.next_id();
    view.predict(place, ABOVE, STONE);
    let broken = view.predictions.next_id();
    view.predict(broken, ABOVE, AIR);
    view.receive_block(ABOVE, STONE);
    view.acknowledge(place);
    assert_eq!(
        view.world.block_at(ABOVE),
        AIR,
        "the later click still waits"
    );
    view.acknowledge(broken);
    assert_eq!(
        view.world.block_at(ABOVE),
        STONE,
        "the server had the last word"
    );
    view.receive_block(ABOVE, AIR);
    assert_eq!(
        view.world.block_at(ABOVE),
        AIR,
        "later changes show at once"
    );
}

#[test]
fn unpredicted_changes_show_at_once_with_their_effects() {
    let mut view = view();
    let id = view.predictions.next_id();
    view.predict(id, ABOVE, STONE);
    let beside = BlockPos::new(1, 1, 0);
    view.receive_block(beside, STONE);
    assert_eq!(view.world.block_at(beside), STONE);
    assert_eq!(view.block_changes.len(), 2);
}

#[test]
fn past_the_server_queue_clicks_wait_for_the_answer_and_play_once() {
    let mut view = view();
    let cells: Vec<BlockPos> = (0..12).map(|x| BlockPos::new(x, 1, 0)).collect();
    let ids: Vec<ActionId> = cells
        .iter()
        .map(|&pos| {
            let id = view.predictions.next_id();
            if view.predictions.may_predict(id) {
                view.predict(id, pos, STONE);
            }
            id
        })
        .collect();
    assert_eq!(
        view.block_changes.len(),
        MAX_PENDING_ACTIONS,
        "the queue's worth"
    );
    let accepted = MAX_PENDING_ACTIONS + 1;
    for (i, (&pos, &id)) in cells.iter().zip(&ids).enumerate() {
        let block = if i < accepted { STONE } else { AIR };
        view.receive_block(pos, block);
        view.acknowledge(id);
    }
    for (i, &pos) in cells.iter().enumerate() {
        let expected = if i < accepted { STONE } else { AIR };
        assert_eq!(view.world.block_at(pos), expected);
    }
    assert_eq!(
        view.block_changes.len(),
        accepted,
        "one sound per block, none twice"
    );
}

#[test]
fn a_reloaded_column_drops_its_predictions() {
    let mut predictions = BlockPredictions::default();
    let id = predictions.next_id();
    predictions.predict(id, ABOVE, AIR);
    predictions.forget_column(ColumnPos { x: 0, z: 0 });
    assert!(predictions.server_block(ABOVE, STONE));
    assert!(predictions.acknowledge(id).is_empty());
}

struct Scene {
    info: BlockInfo,
    world: ClientWorld,
}

impl Scene {
    fn block(&self, id: &str) -> BlockId {
        self.info.content.blocks.key(&id.parse().unwrap()).unwrap()
    }
}

/// A stone block floating above the ground.
const FLOATING: BlockPos = BlockPos::new(4, 66, 3);

/// The base pack; stone up to y = 63 in column (0, 0), bedrock at y = 0, and `FLOATING`.
fn scene() -> Scene {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let (content, _) = Content::load(&packs, &PackSet::default());
    let info = BlockInfo::new(Arc::new(content));
    let key = |id: &str| info.content.blocks.key(&id.parse().unwrap()).unwrap();
    let (stone, bedrock) = (key("strata:stone"), key("strata:bedrock"));
    let mut column = Column::empty(ColumnPos { x: 0, z: 0 });
    for (x, z) in (0..16).flat_map(|x| (0..16).map(move |z| (x, z))) {
        column.set(x, 0, z, bedrock);
        (1..64).for_each(|y| column.set(x, y, z, stone));
    }
    column.set(4, 66, 3, stone);
    let mut world = ClientWorld::default();
    world.insert(column, BiomeMap::uniform(BiomeId(0)));
    Scene { info, world }
}

fn predict(
    scene: &Scene,
    action: WorldAction,
    player: Option<&Body>,
) -> Option<(BlockPos, BlockId)> {
    predicted_change(&scene.info, &scene.world, action, player, false)
}

#[test]
fn breaks_are_predicted_like_the_server_judges_them() {
    let s = scene();
    let stone = BlockPos::new(3, 63, 3);
    let action = WorldAction::Break { pos: stone };
    assert_eq!(predict(&s, action, None), Some((stone, AIR)));
    let bedrock = WorldAction::Break {
        pos: BlockPos::new(3, 0, 3),
    };
    assert_eq!(
        predict(&s, bedrock, None),
        None,
        "unbreakable outside creative"
    );
    let creative = predicted_change(&s.info, &s.world, bedrock, None, true);
    assert!(creative.is_some());
    let air = WorldAction::Break {
        pos: BlockPos::new(3, 64, 3),
    };
    assert_eq!(predict(&s, air, None), None);
}

#[test]
fn placements_follow_the_face_the_support_and_the_player() {
    let s = scene();
    let place = |against, face, block| WorldAction::Place {
        against,
        face,
        block,
    };
    let (ground, floating) = (BlockPos::new(3, 63, 3), FLOATING);
    let beside = floating.neighbour(Face::East);
    let log = s.block("strata:oak_log");
    let (pos, lying) = predict(&s, place(floating, Face::East, log), None).unwrap();
    assert_eq!(pos, beside);
    assert_ne!(lying, log, "a log lies along the clicked face");
    let poppy = s.block("strata:poppy");
    let on_top = predict(&s, place(ground, Face::Up, poppy), None);
    assert_eq!(on_top, Some((ground.neighbour(Face::Up), poppy)));
    let hanging = predict(&s, place(floating, Face::East, poppy), None);
    assert_eq!(hanging, None, "a plant needs ground");
    let body = Body::at(DVec3::new(3.5, 64.0, 3.5));
    let stone = s.block("strata:stone");
    let into_player = predict(&s, place(ground, Face::Up, stone), Some(&body));
    assert_eq!(into_player, None, "never inside the player");
    let item = s.info.item_of(stone).unwrap();
    let used = WorldAction::Use {
        item,
        against: ground,
        face: Face::Up,
    };
    assert_eq!(predict(&s, used, None), None, "items wait for the server");
}

#[test]
fn torches_and_lanterns_are_predicted_on_every_face() {
    let s = scene();
    let place = |against, face, block| WorldAction::Place {
        against,
        face,
        block,
    };
    let ground = BlockPos::new(3, 63, 3);
    let (torch, lantern) = (s.block("strata:torch"), s.block("strata:lantern"));
    let cases = [
        (ground, Face::Up, torch, "strata:torch"),
        (FLOATING, Face::East, torch, "strata:torch__west"),
        (FLOATING, Face::West, torch, "strata:torch__east"),
        (FLOATING, Face::North, torch, "strata:torch__south"),
        (FLOATING, Face::South, torch, "strata:torch__north"),
        (FLOATING, Face::Down, lantern, "strata:lantern__ceiling"),
        (ground, Face::Up, lantern, "strata:lantern"),
    ];
    for (against, face, held, expected) in cases {
        let predicted = predict(&s, place(against, face, held), None);
        let target = against.neighbour(face);
        assert_eq!(
            predicted,
            Some((target, s.block(expected))),
            "{face:?} {expected}"
        );
    }
    let under = predict(&s, place(FLOATING, Face::Down, torch), None);
    assert_eq!(under, None, "no ceiling torch and nothing else to lean on");
    let beside = predict(&s, place(FLOATING, Face::East, lantern), None);
    assert_eq!(beside, None, "no wall lantern");
}
