//! Temporary scene of milestone 1: one textured cube at the origin.
//! Replaced by the block registry (milestone 2) and the chunk world (milestones 3-4).

use crate::render::{Gpu, Renderer, cube};
use glam::DVec3;
use strata_core::texture::{MaterialRecipe, Rgba, RgbaImage, generate_material};

const TEXTURE_SEED: u64 = 0x5747_4154;

pub fn textures() -> Vec<RgbaImage> {
    let stone = MaterialRecipe {
        palette: vec![
            Rgba::rgb(0x6b, 0x6b, 0x70),
            Rgba::rgb(0x7f, 0x7f, 0x84),
            Rgba::rgb(0x91, 0x91, 0x95),
            Rgba::rgb(0xa4, 0xa4, 0xa8),
        ],
        cells: 4,
        octaves: 3,
    };
    vec![generate_material(&stone, TEXTURE_SEED, 16)]
}

pub fn populate(renderer: &mut Renderer, gpu: &Gpu) {
    renderer.add_object(gpu, &cube(0), DVec3::ZERO);
}
