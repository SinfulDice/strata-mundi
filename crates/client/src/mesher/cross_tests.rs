#![allow(clippy::unwrap_used)]

use super::*;
use crate::settings::Vegetation;
use std::path::Path;
use strata_core::content::{BlockId, Content, Face};
use strata_core::pack::PackSet;

const OPTIONS: MeshOptions = MeshOptions {
    ambient_occlusion: true,
    smooth_lighting: true,
    vegetation: Vegetation::Off,
    texture_variants: false,
    biome_blend: 0,
};

fn content() -> Content {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    Content::load(&packs, &PackSet::default()).0
}

fn key(content: &Content, id: &str) -> BlockId {
    content.blocks.key(&id.parse().unwrap()).unwrap()
}

/// The planes of a lone block at (5, 5, 5): position in sixteenths (lean and lift
/// applied) and texture layer of each vertex, four per quad.
fn planes(content: &Content, id: &str) -> Vec<([i32; 3], u16)> {
    let block = key(content, id);
    let blocks = Neighborhood::build(|x, y, z| {
        if [x, y, z] == [5, 5, 5] {
            block
        } else {
            BlockId::AIR
        }
    });
    let mesh = mesh_section(&blocks, &BlockTable::from_content(content), OPTIONS);
    mesh.opaque
        .vertices
        .iter()
        .map(|v| {
            let [x, y, z] = v.position().map(|c| c as i32 * 16);
            let mut p = [x, y - i32::from(v.drop()), z];
            if let Some((wall, nudge)) = v.lean() {
                let (axis, sign) = match wall {
                    Face::North => (2, -1),
                    Face::South => (2, 1),
                    Face::East => (0, 1),
                    _ => (0, -1),
                };
                p[axis] += sign * i32::from(nudge);
            }
            (p, (v.texture & 0xffff) as u16)
        })
        .collect()
}

fn centre(points: &[([i32; 3], u16)], foot: bool) -> [i32; 3] {
    let chosen: Vec<[i32; 3]> = points
        .iter()
        .map(|(p, _)| *p)
        .filter(|p| (p[1] < 6 * 16) == foot)
        .collect();
    [0, 1, 2].map(|i| chosen.iter().map(|p| p[i]).sum::<i32>() / chosen.len() as i32)
}

#[test]
fn wall_torches_lean_on_their_wall_with_every_plane() {
    let content = content();
    let standing = planes(&content, "strata:torch");
    assert_eq!(
        standing.len(),
        16,
        "two planes, both sides, drawn without vegetation"
    );
    for (name, axis, sign) in [
        ("strata:torch__north", 2, -1),
        ("strata:torch__south", 2, 1),
        ("strata:torch__east", 0, 1),
        ("strata:torch__west", 0, -1),
    ] {
        let leaning = planes(&content, name);
        assert_eq!(leaning.len(), standing.len(), "{name}: no plane missing");
        let (foot, top) = (centre(&leaning, true), centre(&leaning, false));
        let middle = 5 * 16 + 8;
        assert_eq!(
            foot[axis],
            middle + sign * 7,
            "{name}: the foot touches the wall"
        );
        assert_eq!(top[axis], middle + sign * 2, "{name}: the top leans out");
        let other = 2 - axis;
        assert_eq!((foot[other], top[other]), (middle, middle), "{name}");
        assert_eq!(foot[1], 5 * 16 + 3, "{name}: lifted off the floor");
    }
}

#[test]
fn hanging_lanterns_show_their_chain_upright() {
    let content = content();
    let standing = planes(&content, "strata:lantern");
    let hanging = planes(&content, "strata:lantern__ceiling");
    assert_eq!(hanging.len(), standing.len());
    let positions = |points: &[([i32; 3], u16)]| points.iter().map(|(p, _)| *p).collect::<Vec<_>>();
    assert_eq!(
        positions(&hanging),
        positions(&standing),
        "same planes, upright"
    );
    assert_ne!(hanging[0].1, standing[0].1, "the texture with the chain");
}
