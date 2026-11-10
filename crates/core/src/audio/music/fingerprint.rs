//! The fingerprint of a theme in the names of its cached pieces: every value the sound
//! depends on, field by field (FNV-1a), so that renaming a type or reordering a debug
//! print never changes it, and any change of a value does.

use super::theme::{Motif, Pad, Piano, Room, Scale, ThemeDef};

struct Fnv(u64);

impl Fnv {
    fn bytes(&mut self, bytes: &[u8]) -> &mut Self {
        for &byte in bytes {
            self.0 = (self.0 ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3);
        }
        self
    }

    fn u8(&mut self, value: u8) -> &mut Self {
        self.bytes(&[value])
    }

    fn f32(&mut self, value: f32) -> &mut Self {
        self.bytes(&value.to_le_bytes())
    }

    fn pair(&mut self, (a, b): (f32, f32)) -> &mut Self {
        self.f32(a).f32(b)
    }

    /// The length first, so that `[1, 2] [3]` and `[1] [2, 3]` differ.
    fn len(&mut self, len: usize) -> &mut Self {
        self.bytes(&(len as u64).to_le_bytes())
    }

    fn degrees(&mut self, degrees: &[i8]) -> &mut Self {
        self.len(degrees.len());
        for &degree in degrees {
            self.bytes(&degree.to_le_bytes());
        }
        self
    }
}

pub(super) fn fingerprint(def: &ThemeDef) -> u64 {
    let mut hash = Fnv(0xCBF2_9CE4_8422_2325);
    hash.len(def.dimensions.len());
    for dimension in &def.dimensions {
        hash.bytes(dimension.to_string().as_bytes()).u8(0);
    }
    hash.u8(u8::from(def.title)).u8(def.root);
    scale(&mut hash, &def.scale);
    hash.f32(def.tempo)
        .u8(def.pieces)
        .pair(def.length)
        .f32(def.density)
        .pair(def.rests)
        .degrees(&def.chords)
        .len(def.motifs.len());
    def.motifs.iter().for_each(|m| motif(&mut hash, m));
    piano(&mut hash, &def.piano);
    match &def.pad {
        Some(p) => pad(hash.u8(1), p),
        None => {
            hash.u8(0);
        }
    }
    room(&mut hash, &def.room);
    hash.f32(def.volume);
    hash.0
}

fn scale(hash: &mut Fnv, scale: &Scale) {
    hash.len(scale.steps().len()).bytes(scale.steps());
}

fn motif(hash: &mut Fnv, motif: &Motif) {
    hash.degrees(&motif.degrees).len(motif.beats.len());
    for &beat in &motif.beats {
        hash.f32(beat);
    }
}

fn piano(hash: &mut Fnv, piano: &Piano) {
    hash.f32(piano.volume)
        .f32(piano.decay)
        .f32(piano.brightness)
        .bytes(&piano.octave.to_le_bytes())
        .f32(piano.bass);
}

fn pad(hash: &mut Fnv, pad: &Pad) {
    hash.f32(pad.volume)
        .f32(pad.attack)
        .f32(pad.release)
        .f32(pad.cutoff)
        .bytes(&pad.octave.to_le_bytes())
        .f32(pad.chance);
}

fn room(hash: &mut Fnv, room: &Room) {
    hash.f32(room.size).f32(room.wet);
}
