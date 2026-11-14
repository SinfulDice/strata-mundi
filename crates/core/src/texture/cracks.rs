//! Cracks spreading over a block being broken. Every stage of a series shares its seed and
//! shows a longer prefix of the same crack lines, so the cracks grow instead of jumping.

use super::RgbaImage;
use super::color::Ramp;
use super::noise::hash;
use super::recipe::BASE_SIZE;

const BRANCHES: i32 = 9;
const BRANCH_LENGTH: i32 = 14;
/// The eight neighbour steps, turning clockwise.
const STEPS: [(i32, i32); 8] = [
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
    (0, -1),
    (1, -1),
];

/// Paints the cracks of `stage` (0 = first) out of `stages`.
pub fn paint(mut image: RgbaImage, ramp: &Ramp, stage: u32, stages: u32, seed: u64) -> RgbaImage {
    let pixels = crack_pixels(seed);
    let shown = pixels.len() * (stage as usize + 1) / stages.max(1) as usize;
    let last = ramp.len().saturating_sub(1);
    for &(x, y) in pixels.iter().take(shown) {
        let level = hash(seed ^ 0xC4A, x, y) as usize % (last + 1);
        image.set(x as u32, y as u32, ramp[level]);
    }
    image
}

/// Pixels of every branch, in the order they appear: the branches grow together, one
/// step each in turn, from points spreading out from the centre.
fn crack_pixels(seed: u64) -> Vec<(i32, i32)> {
    let size = BASE_SIZE as i32;
    let mut walkers: Vec<((i32, i32), usize)> = (0..BRANCHES)
        .map(|branch| {
            // The first branches start near the centre, the later ones farther out.
            let spread = 2 + branch * (size / 2 - 3) / BRANCHES;
            let around =
                |axis| size / 2 - spread + (hash(seed, branch, axis) % (2 * spread as u32)) as i32;
            let start = (around(0), around(1));
            (start, hash(seed, branch, 2) as usize % STEPS.len())
        })
        .collect();
    let mut pixels = Vec::new();
    for step in 0..BRANCH_LENGTH {
        for (branch, ((x, y), heading)) in walkers.iter_mut().enumerate() {
            let turn = hash(seed, branch as i32, 3 + step) % 5;
            *heading = match turn {
                0 => (*heading + 1) % STEPS.len(),
                1 => (*heading + STEPS.len() - 1) % STEPS.len(),
                _ => *heading,
            };
            let (dx, dy) = STEPS[*heading];
            let (nx, ny) = (*x + dx, *y + dy);
            if !(0..size).contains(&nx) || !(0..size).contains(&ny) {
                // A branch reaching the edge turns back instead of tracing the border.
                *heading = (*heading + STEPS.len() / 2) % STEPS.len();
                continue;
            }
            (*x, *y) = (nx, ny);
            if !pixels.contains(&(*x, *y)) {
                pixels.push((*x, *y));
            }
        }
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::texture::Rgba;

    fn painted(stage: u32) -> Vec<(u32, u32)> {
        let ramp: Ramp = vec![Rgba([0, 0, 0, 200]), Rgba([20, 20, 20, 180])];
        let blank = RgbaImage::new(BASE_SIZE, BASE_SIZE, Rgba([0; 4]));
        let image = paint(blank, &ramp, stage, 10, 7);
        (0..BASE_SIZE)
            .flat_map(|y| (0..BASE_SIZE).map(move |x| (x, y)))
            .filter(|&(x, y)| image.get(x, y).0[3] > 0)
            .collect()
    }

    #[test]
    fn each_stage_keeps_the_cracks_of_the_one_before() {
        let mut before = painted(0);
        assert!(!before.is_empty());
        for stage in 1..10 {
            let now = painted(stage);
            assert!(now.len() > before.len(), "stage {stage}");
            assert!(before.iter().all(|pixel| now.contains(pixel)));
            before = now;
        }
        assert!(before.len() < (BASE_SIZE * BASE_SIZE) as usize * 3 / 5);
    }
}
