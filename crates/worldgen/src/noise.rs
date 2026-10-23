use crate::Seed;
use fastnoise_lite::{FastNoiseLite, FractalType, NoiseType};

/// Fractal OpenSimplex2 noise in `[-1, 1]`, seeded from the world seed and a salt so that
/// each use (temperature, height, caves…) is independent.
pub struct Noise {
    inner: FastNoiseLite,
}

impl Noise {
    pub fn new(seed: Seed, salt: u64, frequency: f64, octaves: i32) -> Self {
        let mut inner = FastNoiseLite::with_seed(seed.derive(salt));
        inner.set_noise_type(Some(NoiseType::OpenSimplex2));
        inner.set_frequency(Some(frequency as f32));
        if octaves > 1 {
            inner.set_fractal_type(Some(FractalType::FBm));
            inner.set_fractal_octaves(Some(octaves));
        }
        Self { inner }
    }

    pub fn at(&self, x: f64, z: f64) -> f64 {
        f64::from(self.inner.get_noise_2d(x, z))
    }

    pub fn at_3d(&self, x: f64, y: f64, z: f64) -> f64 {
        f64::from(self.inner.get_noise_3d(x, y, z))
    }
}
