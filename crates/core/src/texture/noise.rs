//! Integer-hash value noise. Tileable over the texture size so blocks repeat seamlessly.

/// Stable 32-bit hash (no dependence on platform or Rust version).
pub fn hash(seed: u64, x: i32, y: i32) -> u32 {
    let mut h = seed ^ (x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    h ^= (y as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    h ^= h >> 33;
    h = h.wrapping_mul(0xFF51_AFD7_ED55_8CCD);
    h ^= h >> 33;
    h = h.wrapping_mul(0xC4CE_B9FE_1A85_EC53);
    h ^= h >> 33;
    h as u32
}

fn unit(seed: u64, x: i32, y: i32) -> f32 {
    hash(seed, x, y) as f32 / u32::MAX as f32
}

/// Smooth value noise in `[0, 1]`, with a lattice of `cells` per `period` pixels, wrapping at `period`.
pub fn tileable_value(seed: u64, x: u32, y: u32, period: u32, cells: u32) -> f32 {
    let cells = cells.max(1);
    let fx = x as f32 * cells as f32 / period as f32;
    let fy = y as f32 * cells as f32 / period as f32;
    let (x0, y0) = (fx.floor() as i32, fy.floor() as i32);
    let (tx, ty) = (smooth(fx - x0 as f32), smooth(fy - y0 as f32));
    let wrap = |v: i32| v.rem_euclid(cells as i32);
    let corner = |dx: i32, dy: i32| unit(seed, wrap(x0 + dx), wrap(y0 + dy));
    let top = lerp(corner(0, 0), corner(1, 0), tx);
    let bottom = lerp(corner(0, 1), corner(1, 1), tx);
    lerp(top, bottom, ty)
}

/// Sum of octaves of `tileable_value`, normalized to `[0, 1]`.
pub fn tileable_fractal(seed: u64, x: u32, y: u32, period: u32, cells: u32, octaves: u32) -> f32 {
    let (mut total, mut weight, mut amplitude) = (0.0, 0.0, 1.0);
    for octave in 0..octaves.max(1) {
        let octave_seed = seed.wrapping_add(u64::from(octave) * 0x5851_F42D);
        total += amplitude * tileable_value(octave_seed, x, y, period, cells << octave);
        weight += amplitude;
        amplitude *= 0.5;
    }
    total / weight
}

fn smooth(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn noise_wraps_at_period() {
        for y in 0..16 {
            let left = tileable_value(3, 0, y, 16, 4);
            let wrapped = tileable_value(3, 16, y, 16, 4);
            assert!((left - wrapped).abs() < 1e-6);
        }
    }

    #[test]
    fn fractal_stays_in_unit_range() {
        for (x, y) in (0..16).flat_map(|x| (0..16).map(move |y| (x, y))) {
            let value = tileable_fractal(9, x, y, 16, 2, 3);
            assert!((0.0..=1.0).contains(&value));
        }
    }
}
