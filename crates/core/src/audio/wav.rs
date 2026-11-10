//! 16-bit mono WAV files of rendered buffers, to listen to recipes outside the game (and
//! the cache of the music).

use super::SAMPLE_RATE;
use std::path::Path;

pub const WAV_HEADER: usize = 44;

/// Header of a 16-bit mono file of `frames` samples at `SAMPLE_RATE`.
pub fn wav_header(frames: usize) -> [u8; WAV_HEADER] {
    let data_len = (frames.min(u32::MAX as usize / 2 - 64) * 2) as u32;
    let mut bytes = [0; WAV_HEADER];
    let fields: [&[u8]; 12] = [
        b"RIFF",
        &(36 + data_len).to_le_bytes(),
        b"WAVEfmt ",
        &16u32.to_le_bytes(),
        &1u16.to_le_bytes(),
        &1u16.to_le_bytes(),
        &SAMPLE_RATE.to_le_bytes(),
        &(SAMPLE_RATE * 2).to_le_bytes(),
        &2u16.to_le_bytes(),
        &16u16.to_le_bytes(),
        b"data",
        &data_len.to_le_bytes(),
    ];
    let mut at = 0;
    for field in fields {
        bytes[at..at + field.len()].copy_from_slice(field);
        at += field.len();
    }
    bytes
}

/// Samples as 16-bit little-endian integers, clamped to [−1, 1].
pub fn pcm16(samples: &[f32], bytes: &mut Vec<u8>) {
    for sample in samples {
        let value = (sample.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        bytes.extend_from_slice(&value.to_le_bytes());
    }
}

pub fn wav_bytes(samples: &[f32]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(WAV_HEADER + samples.len() * 2);
    bytes.extend_from_slice(&wav_header(samples.len()));
    pcm16(samples, &mut bytes);
    bytes
}

pub fn write_wav(path: &Path, samples: &[f32]) -> std::io::Result<()> {
    std::fs::write(path, wav_bytes(samples))
}
