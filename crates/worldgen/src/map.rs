//! Top-down preview of the generation: biome colors shaded by the relief, water in blue.
//! Used by the world creation screen and `strata --map`.

use crate::generator::Generator;

/// Area seen on a map, in blocks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MapArea {
    pub center: (i32, i32),
    pub size: (u32, u32),
    /// Blocks per pixel (1 or more).
    pub scale: u32,
}

/// RGB pixels, row by row from the north-west corner.
pub fn render(generator: &Generator, area: MapArea) -> Vec<[u8; 3]> {
    render_with(generator, area, 1, &|| false).unwrap_or_default()
}

/// `render` on `threads` threads, each taking rows in turn; `None` as soon as `cancelled`
/// says so (checked between rows).
pub fn render_with(
    generator: &Generator,
    area: MapArea,
    threads: usize,
    cancelled: &(dyn Fn() -> bool + Sync),
) -> Option<Vec<[u8; 3]>> {
    let (width, height) = (area.size.0 as usize, area.size.1 as usize);
    let threads = threads.clamp(1, height.max(1));
    let parts: Vec<Option<Rows>> = std::thread::scope(|scope| {
        let workers: Vec<_> = (0..threads)
            .map(|first| {
                scope.spawn(move || {
                    (first..height)
                        .step_by(threads)
                        .map(|row| (!cancelled()).then(|| (row, sample_row(generator, area, row))))
                        .collect::<Option<Vec<_>>>()
                })
            })
            .collect();
        workers
            .into_iter()
            .map(|worker| worker.join().ok().flatten())
            .collect()
    });
    let mut rows = vec![Vec::new(); height];
    for (row, samples) in parts
        .into_iter()
        .collect::<Option<Vec<_>>>()?
        .into_iter()
        .flatten()
    {
        rows[row] = samples;
    }
    let samples: Vec<Ground> = rows.into_iter().flatten().collect();
    (samples.len() == width * height).then(|| shade_all(generator, area, &samples))
}

/// Ground height and biome color of one point.
type Ground = (i32, [u8; 3]);
/// Rows drawn by one thread, with their number.
type Rows = Vec<(usize, Vec<Ground>)>;

fn sample_row(generator: &Generator, area: MapArea, row: usize) -> Vec<Ground> {
    let scale = area.scale.max(1) as i32;
    let left = area.center.0 - area.size.0 as i32 * scale / 2;
    let top = area.center.1 - area.size.1 as i32 * scale / 2;
    let z = top + row as i32 * scale;
    (0..area.size.0 as i32)
        .map(|px| {
            let (sample, biome) = generator.sample(left + px * scale, z);
            (sample.height, biome.map_color)
        })
        .collect()
}

fn shade_all(generator: &Generator, area: MapArea, samples: &[Ground]) -> Vec<[u8; 3]> {
    let width = area.size.0 as usize;
    let scale = area.scale.max(1) as i32;
    let sea_level = generator.content().terrain.sea_level;
    let mut pixels = Vec::with_capacity(samples.len());
    for (index, &(ground, base)) in samples.iter().enumerate() {
        let color = if ground < sea_level {
            water(sea_level - ground)
        } else {
            let west = if index % width > 0 {
                samples[index - 1].0
            } else {
                ground
            };
            let north = samples
                .get(index.wrapping_sub(width))
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
