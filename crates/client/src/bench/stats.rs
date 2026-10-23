/// Frame time samples in milliseconds, summarized like common benchmarking tools.
#[derive(Debug, Default, Clone)]
pub struct Samples {
    values: Vec<f32>,
}

impl Samples {
    pub fn push(&mut self, milliseconds: f32) {
        self.values.push(milliseconds);
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn mean(&self) -> Option<f32> {
        (!self.is_empty()).then(|| self.values.iter().sum::<f32>() / self.len() as f32)
    }

    /// Mean of the slowest `fraction` of samples (0.01 for "1 % low").
    pub fn worst_mean(&self, fraction: f32) -> Option<f32> {
        if self.is_empty() {
            return None;
        }
        let mut sorted = self.values.clone();
        sorted.sort_by(|a, b| b.total_cmp(a));
        let count = ((sorted.len() as f32 * fraction).ceil() as usize).clamp(1, sorted.len());
        Some(sorted[..count].iter().sum::<f32>() / count as f32)
    }
}

pub fn fps(frame_milliseconds: f32) -> f32 {
    1000.0 / frame_milliseconds.max(f32::EPSILON)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_percent_low_uses_the_slowest_frames() {
        let mut samples = Samples::default();
        for _ in 0..99 {
            samples.push(10.0);
        }
        samples.push(50.0);
        assert_eq!(samples.mean(), Some(10.4));
        assert_eq!(samples.worst_mean(0.01), Some(50.0));
        assert_eq!(fps(samples.worst_mean(0.01).unwrap()), 20.0);
    }

    #[test]
    fn empty_samples_have_no_statistics() {
        assert_eq!(Samples::default().mean(), None);
        assert_eq!(Samples::default().worst_mean(0.01), None);
    }
}
