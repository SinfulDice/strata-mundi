//! Cost of mixing one output block (512 frames, about 12 ms at 44.1 kHz) with 32 voices
//! and 4 ambience beds: advanced spatial audio (reverberation of a cavern, muffled
//! voices) against plain stereo.

use criterion::{Criterion, criterion_group, criterion_main};
use glam::DVec3;
use std::hint::black_box;
use strata_client::audio::{BedVolume, Command, MAX_VOICES, Mixer, Space, Voice};
use strata_core::audio::{Rng, SAMPLE_RATE, SoundCategory};
use strata_core::content::SoundId;

const BLOCK_FRAMES: usize = 512;

fn noise(seconds: f32, seed: u64) -> std::sync::Arc<[f32]> {
    let mut rng = Rng::new(seed);
    let len = (seconds * SAMPLE_RATE as f32) as usize;
    (0..len).map(|_| rng.signed() * 0.3).collect()
}

/// A mixer with every voice taken by a long sound (none ends during the bench).
fn busy(spatial: bool) -> Mixer {
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Spatial(spatial));
    mixer.apply(Command::Space(Space::LargeCavern));
    let buffer = noise(600.0, 1);
    for index in 0..MAX_VOICES {
        let angle = index as f64 * 0.7;
        mixer.apply(Command::Play(Voice {
            id: index as u32,
            buffer: buffer.clone(),
            category: SoundCategory::Blocks,
            gain: 1.0,
            pitch: 1.0 + index as f32 * 0.01,
            at: Some(DVec3::new(angle.cos() * 6.0, 0.0, angle.sin() * 6.0)),
            range: 32.0,
            muffle: if index % 2 == 0 { 0.6 } else { 0.0 },
        }));
    }
    for bed in 0..4 {
        mixer.apply(Command::Bed(BedVolume {
            sound: SoundId(bed),
            buffer: noise(8.0, u64::from(bed) + 2),
            category: SoundCategory::Ambient,
            gain: 0.5,
        }));
    }
    let mut out = vec![0.0; BLOCK_FRAMES * 2];
    for _ in 0..50 {
        mixer.render(&mut out);
    }
    mixer
}

fn mix_block(c: &mut Criterion) {
    let mut out = vec![0.0; BLOCK_FRAMES * 2];
    for (name, spatial) in [("advanced", true), ("simple", false)] {
        let mut mixer = busy(spatial);
        c.bench_function(&format!("audio/mix_block_32_voices_{name}"), |b| {
            b.iter(|| {
                mixer.render(&mut out);
                black_box(&out);
            });
        });
    }
}

criterion_group!(benches, mix_block);
criterion_main!(benches);
