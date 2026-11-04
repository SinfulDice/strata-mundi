//! Top-down preview of the generation: biome colors shaded by the relief, water in blue.
//! Used by the world creation screen and `strata --map`.

use crate::generator::Generator;

/// Area seen on a map, in blocks.
#[derive(Debug, Clone, Copy)]
pub struct MapArea {
    pub center: (i32, i32),
    pub size: (u32, u32),
    /// Blocks per pixel (1 or more).
    pub scale: u32,
}

/// RGB pixels, row by row from the north-west corner.
pub fn render(generator: &Generator, area: MapArea) -> Vec<[u8; 3]> {
    let (width, height) = area.size;
    let scale = area.scale.max(1) as i32;
    let left = area.center.0 - width as i32 * scale / 2;
    let top = area.center.1 - height as i32 * scale / 2;
    let sea_level = generator.content().terrain.sea_level;
    let samples: Vec<(i32, [u8; 3])> = (0..height as i32)
        .flat_map(|py| (0..width as i32).map(move |px| (px, py)))
        .map(|(px, py)| {
            let (sample, biome) = generator.sample(left + px * scale, top + py * scale);
            (sample.height, biome.map_color)
        })
        .collect();
    let mut pixels = Vec::with_capacity(samples.len());
    for (index, &(ground, base)) in samples.iter().enumerate() {
        let color = if ground < sea_level {
            water(sea_level - ground)
        } else {
            let west = if index % width as usize > 0 {
                samples[index - 1].0
            } else {
                ground
            };
            let north = samples
                .get(index.wrapping_sub(width as usize))
                .map_or(ground, |s| s.0);
            shade(base, (ground - west) + (ground - north), scale)
        };
        pixels.push(color);
    }
    pixels
}

fn water(depth: i32) -> [u8; 3] {
    let deep = (depth as f32 / 50.0).min(1.0);
    let mix = |shallow: f32, far: f32| (shallow + (far - shallow) * deep) as u8;
    [mix(70.0, 18.0), mix(120.0, 40.0), mix(205.0, 110.0)]
}

/// Lit from the north-west, like a relief map.
fn shade(base: [u8; 3], slope: i32, scale: i32) -> [u8; 3] {
    let light = 1.0 + (slope as f32 / scale as f32 * 0.08).clamp(-0.45, 0.45);
    base.map(|c| (f32::from(c) * light).clamp(0.0, 255.0) as u8)
}
