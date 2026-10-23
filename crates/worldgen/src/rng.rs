use crate::Seed;
use strata_core::coords::ColumnPos;

/// Small deterministic generator (SplitMix64), seeded per column and purpose, so results
/// never depend on the order in which columns are generated.
pub struct Rng(u64);

impl Rng {
    pub fn for_column(seed: Seed, pos: ColumnPos, salt: u64) -> Self {
        let mixed = seed.0
            ^ (pos.x as u32 as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
            ^ (pos.z as u32 as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F)
            ^ salt.wrapping_mul(0x1656_67B1_9E37_79F9);
        let mut rng = Self(mixed);
        rng.next_u64();
        rng
    }

    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// Uniform in `low..=high`.
    pub fn range(&mut self, low: i32, high: i32) -> i32 {
        if high <= low {
            return low;
        }
        low + (self.next_u64() % (high - low + 1) as u64) as i32
    }

    /// `expected` = 2.4 gives 2 or 3 with the right average.
    pub fn count(&mut self, expected: f64) -> u32 {
        let whole = expected.floor();
        whole as u32 + u32::from(self.unit() < expected - whole)
    }
}
