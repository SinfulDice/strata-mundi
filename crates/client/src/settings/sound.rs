//! Sound settings: volumes by category, subtitles, spatial audio (the only one the
//! presets set: plain stereo on the DS and the very low preset) and how often music comes.

use super::Preset;
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
    pub spatial: SpatialAudio,
    /// How long the silence between two pieces of music lasts.
    pub music_frequency: MusicFrequency,
}

/// How often a piece of music comes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum MusicFrequency {
    Rare,
    #[default]
    Normal,
    Often,
}

impl MusicFrequency {
    pub const ALL: [MusicFrequency; 3] = [Self::Rare, Self::Normal, Self::Often];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Rare => "rare",
            Self::Normal => "normal",
            Self::Often => "often",
        }
    }

    /// Seconds of silence between two pieces, at least and at most.
    pub const fn wait(self) -> (f32, f32) {
        match self {
            Self::Rare => (600.0, 1200.0),
            Self::Normal => (300.0, 600.0),
            Self::Often => (120.0, 300.0),
        }
    }
}

/// How sounds are placed in space.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SpatialAudio {
    /// Reverberation of the place and sounds muffled behind walls.
    #[default]
    Advanced,
    /// Distance and left/right only: less work for the processor.
    Simple,
}

impl SpatialAudio {
    pub const ALL: [SpatialAudio; 2] = [Self::Advanced, Self::Simple];

    pub const fn id(self) -> &'static str {
        match self {
            Self::Advanced => "advanced",
            Self::Simple => "simple",
        }
    }

    pub fn for_preset(preset: Preset) -> Self {
        match preset {
            Preset::VeryLow | Preset::Ds => Self::Simple,
            Preset::Low | Preset::Medium | Preset::High | Preset::Ultra => Self::Advanced,
        }
    }

    pub fn is_advanced(self) -> bool {
        self == Self::Advanced
    }
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            master: 100,
            blocks: 100,
            ambient: 100,
            music: 70,
            subtitles: false,
            spatial: SpatialAudio::Advanced,
            music_frequency: MusicFrequency::Normal,
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
