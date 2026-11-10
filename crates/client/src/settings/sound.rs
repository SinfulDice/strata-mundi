//! Sound settings: volumes by category and subtitles. Presets never touch them.

use serde::{Deserialize, Serialize};
use strata_core::audio::SoundCategory;

/// Percent range of every volume slider.
pub const VOLUME: (u32, u32) = (0, 100);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SoundSettings {
    pub master: u32,
    pub blocks: u32,
    pub ambient: u32,
    pub music: u32,
    /// Text at the bottom right for every sound heard, with its direction.
    pub subtitles: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            master: 100,
            blocks: 100,
            ambient: 100,
            music: 70,
            subtitles: false,
        }
    }
}

impl SoundSettings {
    pub fn sanitize(&mut self) {
        for volume in [
            &mut self.master,
            &mut self.blocks,
            &mut self.ambient,
            &mut self.music,
        ] {
            *volume = (*volume).min(VOLUME.1);
        }
    }

    /// Amplitude factor of a category, master included. Squared: half the slider
    /// sounds about half as loud.
    pub fn gain(&self, category: SoundCategory) -> f32 {
        let percent = match category {
            SoundCategory::Blocks => self.blocks,
            SoundCategory::Ambient => self.ambient,
            SoundCategory::Music => self.music,
        };
        curve(self.master) * curve(percent)
    }
}

fn curve(percent: u32) -> f32 {
    let fraction = percent.min(VOLUME.1) as f32 / VOLUME.1 as f32;
    fraction * fraction
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gains_follow_master_and_category() {
        let mut settings = SoundSettings {
            master: 50,
            blocks: 200,
            ..SoundSettings::default()
        };
        settings.sanitize();
        assert_eq!(settings.blocks, 100);
        assert!((settings.gain(SoundCategory::Blocks) - 0.25).abs() < 1e-6);
        assert!((settings.gain(SoundCategory::Music) - 0.25 * 0.49).abs() < 1e-6);
        settings.master = 0;
        assert_eq!(settings.gain(SoundCategory::Ambient), 0.0);
    }
}
