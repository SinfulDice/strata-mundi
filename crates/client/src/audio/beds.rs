//! Ambience beds in the mixer: looping buffers heard all around, each faded to the gain
//! the game asks (the game does the slow cross-fades, the mixer smooths each block).

use super::mixer::Volumes;
use super::trash::Trash;
use std::sync::Arc;
use strata_core::audio::SoundCategory;
use strata_core::content::SoundId;

/// Most beds looping at once (the game holds back more).
pub const MAX_BEDS: usize = 12;
/// Room in the mixer: beds let go still fade out for a block while new ones start.
pub(super) const BED_SLOTS: usize = 2 * MAX_BEDS;

/// What the game asks of one bed.
#[derive(Debug, Clone)]
pub struct BedVolume {
    pub sound: SoundId,
    pub buffer: Arc<[f32]>,
    pub category: SoundCategory,
    pub gain: f32,
}

pub(super) struct Bed {
    asked: BedVolume,
    /// Position in the buffer, in (fractional) samples.
    cursor: f64,
    /// Gain at the end of the last block.
    gain: f32,
}

impl Bed {
    /// Starts, changes or lets go a bed (an unknown one at gain 0 is ignored).
    pub fn set(beds: &mut Vec<Bed>, asked: BedVolume, trash: &Trash) {
        if asked.buffer.is_empty() {
            trash.buffer(asked.buffer);
            return;
        }
        if let Some(bed) = beds.iter_mut().find(|b| b.asked.sound == asked.sound) {
            if bed.asked.buffer.len() != asked.buffer.len() {
                bed.cursor = 0.0;
            }
            let old = std::mem::replace(&mut bed.asked, asked);
            trash.buffer(old.buffer);
            return;
        }
        if asked.gain > 0.0 && beds.len() < BED_SLOTS {
            beds.push(Bed {
                asked,
                cursor: 0.0,
                gain: 0.0,
            });
        } else {
            trash.buffer(asked.buffer);
        }
    }

    /// Adds every bed to `out`, centred; silent beds that were let go are dropped.
    /// `step`: samples of a buffer read per output frame.
    pub fn mix_all(
        beds: &mut Vec<Bed>,
        volumes: &Volumes,
        out: &mut [f32],
        step: f64,
        trash: &Trash,
    ) {
        let mut index = 0;
        while index < beds.len() {
            let bed = &mut beds[index];
            bed.mix(volumes, out, step);
            if bed.gain > 0.0 || bed.asked.gain > 0.0 {
                index += 1;
            } else {
                trash.buffer(beds.swap_remove(index).asked.buffer);
            }
        }
    }

    fn mix(&mut self, volumes: &Volumes, out: &mut [f32], step: f64) {
        let buffer = &self.asked.buffer;
        let (len, frames) = (buffer.len(), out.len() / 2);
        let target = self.asked.gain.max(0.0)
            * volumes.get(self.asked.category)
            * std::f32::consts::FRAC_1_SQRT_2;
        let start = self.gain;
        if (start <= 0.0 && target <= 0.0) || len == 0 {
            self.gain = 0.0;
            return;
        }
        for frame in 0..frames {
            let ramp = (frame + 1) as f32 / frames as f32;
            let index = (self.cursor as usize).min(len - 1);
            let fraction = (self.cursor - index as f64) as f32;
            let (a, b) = (buffer[index], buffer[(index + 1) % len]);
            let sample = (a + (b - a) * fraction) * (start + (target - start) * ramp);
            out[frame * 2] += sample;
            out[frame * 2 + 1] += sample;
            self.cursor = (self.cursor + step) % len as f64;
        }
        self.gain = target;
    }
}
