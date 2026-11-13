//! Sounds asked before their buffer was made: started as soon as it is, if still in time.

use super::{Audio, Command, Cue, MAX_STARTS, Voice};
use strata_core::audio::SAMPLE_RATE;

/// A sound whose buffer was not made yet is started when it is, if it waited less than
/// this (later it would not match what is seen).
pub const MAX_LATE: f32 = 0.1;

/// A sound waiting for its buffer (the variant drawn, hurried once).
pub(super) struct Late {
    cue: Cue,
    muffle: f32,
    variant: usize,
    waited: f32,
}

impl Audio {
    /// Starts the sound, or keeps it waiting if its buffer is on its way.
    pub(super) fn start_or_wait(&mut self, cue: Cue, muffle: f32) {
        if !self.audible(cue) {
            return;
        }
        match self.bank.pick(cue.sound, &mut self.rng) {
            Ok(pick) => self.start_voice(cue, muffle, pick),
            Err(Some(variant)) if self.late.len() < MAX_STARTS => self.late.push(Late {
                cue,
                muffle,
                variant,
                waited: 0.0,
            }),
            Err(_) => {}
        }
    }

    /// Starts the sounds whose buffer came in time, forgets those too late.
    pub(super) fn start_late(&mut self, seconds: f32) {
        let mut late = std::mem::take(&mut self.late);
        late.retain_mut(|waiting| {
            waiting.waited += seconds;
            let (id, variant) = (waiting.cue.sound, waiting.variant);
            if waiting.waited > MAX_LATE || !self.bank.is_asked(id, variant) {
                if let Some(pick) = self.bank.pick_ready(id, variant, &mut self.rng)
                    && waiting.waited <= MAX_LATE
                {
                    self.start_voice(waiting.cue, waiting.muffle, pick);
                }
                return false;
            }
            true
        });
        self.late = late;
    }

    fn audible(&self, cue: Cue) -> bool {
        self.output.is_some()
            && self
                .bank
                .sound(cue.sound)
                .is_some_and(|sound| self.settings.gain(sound.def.category) > 0.0)
    }

    fn start_voice(&mut self, cue: Cue, muffle: f32, pick: super::bank::Pick) {
        let Some(sound) = self.bank.sound(cue.sound) else {
            return;
        };
        let (category, range) = (sound.def.category, sound.def.range);
        self.next_voice = self.next_voice.wrapping_add(1);
        let seconds = pick.buffer.len() as f32 / SAMPLE_RATE as f32 / pick.pitch.max(0.01);
        let sent = self.send(Command::Play(Voice {
            id: self.next_voice,
            buffer: pick.buffer,
            category,
            gain: 1.0,
            pitch: pick.pitch,
            at: Some(cue.at),
            range,
            muffle,
        }));
        if sent {
            self.environment
                .track(self.next_voice, cue.at, seconds, muffle);
        }
    }
}
