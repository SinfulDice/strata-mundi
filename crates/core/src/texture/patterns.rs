//! Procedural patterns. Each sets the pixels it covers and leaves the others untouched,
//! so the same code paints whole materials and overlays.

use super::color::Ramp;
use super::noise::{hash, tileable_fractal, unit};
use super::recipe::{BASE_SIZE, Painter, Pattern};
use super::{Rgba, RgbaImage};
use std::collections::HashSet;

const S: u32 = BASE_SIZE;
const CLEAR: Rgba = Rgba([0; 4]);

pub fn paint(
    mut image: RgbaImage,
    ramp: &Ramp,
    pattern: &Pattern,
    painter: &Painter<'_>,
) -> Result<RgbaImage, String> {
    let seed = painter.seed;
    let border = match pattern {
        Pattern::Rings { border } => Some(painter.ramp(border)?),
        _ => None,
    };
    for y in 0..S {
        for x in 0..S {
            let pixel = match pattern {
                Pattern::Noise { cells, octaves } => {
                    Some(noise(ramp, seed, x, y, *cells, *octaves))
                }
                Pattern::Rings { .. } => Some(rings(ramp, border.unwrap_or(ramp), seed, x, y)),
                Pattern::Planks { boards } => Some(planks(ramp, seed, x, y, *boards)),
                Pattern::Leaves { holes } => leaves(ramp, seed, x, y, *holes),
                Pattern::Stripes { period } => Some(stripes(ramp, seed, x, y, *period)),
                Pattern::TopFringe { depth } => fringe(ramp, seed, x, y, *depth),
                Pattern::Frame => frame(ramp, seed, x, y),
            };
            if let Some(color) = pixel {
                image.set(x, y, color);
            }
        }
    }
    Ok(image)
}

/// Color at `level` in `[0, 1]` (0 = darkest).
fn pick(ramp: &Ramp, level: f32) -> Rgba {
    let last = ramp.len().saturating_sub(1);
    ramp[((level * ramp.len() as f32) as usize).min(last)]
}

fn dither(seed: u64, x: u32, y: u32) -> f32 {
    unit(seed ^ 0xD17, x as i32, y as i32) - 0.5
}

fn noise(ramp: &Ramp, seed: u64, x: u32, y: u32, cells: (u32, u32), octaves: u32) -> Rgba {
    let grain = tileable_fractal(seed, x, y, S, cells, octaves);
    pick(ramp, (grain + dither(seed, x, y) * 0.35).clamp(0.0, 1.0))
}

fn rings(ramp: &Ramp, bark: &Ramp, seed: u64, x: u32, y: u32) -> Rgba {
    if x == 0 || y == 0 || x == S - 1 || y == S - 1 {
        return noise(bark, seed, x, y, (8, 8), 1);
    }
    let center = (S as f32 - 1.0) / 2.0;
    let distance = ((x as f32 - center).powi(2) + (y as f32 - center).powi(2)).sqrt();
    let wobble = tileable_fractal(seed, x, y, S, (4, 4), 1) * 1.2;
    let ring = (distance * 0.8 + wobble) as u32 % 2;
    let level = if ring == 0 { 0.8 } else { 0.4 };
    pick(ramp, level + dither(seed, x, y) * 0.2)
}

fn planks(ramp: &Ramp, seed: u64, x: u32, y: u32, boards: u32) -> Rgba {
    let height = (S / boards.clamp(1, S)).max(2);
    let board = y / height;
    let row = y % height;
    let seam_x = hash(seed, board as i32, 7) % S;
    if row == height - 1 || x == seam_x {
        return pick(ramp, 0.0);
    }
    let grain = tileable_fractal(seed, x, y, S, (2, 16), 2);
    let light = if row == 0 { 0.15 } else { 0.0 };
    pick(
        ramp,
        (0.3 + grain * 0.5 + light + dither(seed, x, y) * 0.15).clamp(0.0, 1.0),
    )
}

fn leaves(ramp: &Ramp, seed: u64, x: u32, y: u32, holes: f32) -> Option<Rgba> {
    if unit(seed ^ 0x1EAF, x as i32, y as i32) < holes {
        return Some(CLEAR);
    }
    Some(noise(ramp, seed, x, y, (4, 4), 2))
}

fn stripes(ramp: &Ramp, seed: u64, x: u32, y: u32, period: u32) -> Rgba {
    let period = period.max(2);
    if hash(seed ^ 0x5B1, x as i32, y as i32).is_multiple_of(29) {
        return pick(ramp, 1.0);
    }
    match x % period {
        0 => pick(ramp, 0.1),
        1 => pick(ramp, 0.8),
        _ => pick(ramp, 0.45 + dither(seed, x, y) * 0.3),
    }
}

fn fringe(ramp: &Ramp, seed: u64, x: u32, y: u32, depth: u32) -> Option<Rgba> {
    let local = (depth + hash(seed ^ 0xF41, x as i32, 0) % 3).saturating_sub(1);
    if y >= local {
        return None;
    }
    let level = if y + 1 == local {
        0.15
    } else {
        tileable_fractal(seed, x, y, S, (4, 2), 2)
    };
    Some(pick(
        ramp,
        (level + dither(seed, x, y) * 0.3).clamp(0.0, 1.0),
    ))
}

fn frame(ramp: &Ramp, seed: u64, x: u32, y: u32) -> Option<Rgba> {
    if x == 0 || y == 0 || x == S - 1 || y == S - 1 {
        return Some(pick(ramp, 0.45 + dither(seed, x, y) * 0.3));
    }
    let glint = (x + y == 6 && (2..=4).contains(&x)) || (x + y == 9 && (3..=5).contains(&x));
    Some(if glint { pick(ramp, 1.0) } else { CLEAR })
}

/// Small clusters of ore, each lit from the top left: highlight where the pixel above-left
/// is not ore, shadow where the pixel below-right is not ore.
pub fn ore(mut image: RgbaImage, ramp: &Ramp, clusters: u32, seed: u64) -> RgbaImage {
    const SHAPE: [(i32, i32); 8] = [
        (0, 0),
        (1, 0),
        (0, 1),
        (1, 1),
        (-1, 0),
        (0, -1),
        (2, 1),
        (1, 2),
    ];
    let mut cells = HashSet::new();
    for cluster in 0..clusters as i32 {
        let cx = 2 + (hash(seed, cluster, 1) % (S - 4)) as i32;
        let cy = 2 + (hash(seed, cluster, 2) % (S - 4)) as i32;
        let bits = hash(seed, cluster, 3);
        for (i, (dx, dy)) in SHAPE.iter().enumerate() {
            if i < 2 || bits & (1 << i) != 0 {
                cells.insert((
                    (cx + dx).rem_euclid(S as i32),
                    (cy + dy).rem_euclid(S as i32),
                ));
            }
        }
    }
    let last = ramp.len().saturating_sub(1);
    let wrap = |v: i32| v.rem_euclid(S as i32);
    for &(x, y) in &cells {
        let lit = !cells.contains(&(wrap(x - 1), wrap(y - 1)));
        let shaded = !cells.contains(&(wrap(x + 1), wrap(y + 1)));
        let index = match (lit, shaded) {
            (true, false) => last,
            (false, true) => 0,
            _ => last / 2,
        };
        image.set(x as u32, y as u32, ramp[index]);
    }
    image
}
