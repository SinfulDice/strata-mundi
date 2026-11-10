//! `strata --render-music <dir>`: every piece of every theme as a WAV file, with a picture
//! of it (waveform above, notes found by a semitone spectrogram below) to check the
//! silences and the notes without listening.

use anyhow::Context;
use std::path::Path;
use std::time::Instant;
use strata_client::screenshot::write_png;
use strata_core::audio::music::{Mood, Piece, hertz};
use strata_core::audio::{SAMPLE_RATE, write_wav};
use strata_core::content::Content;
use strata_core::texture::{Rgba, RgbaImage};

const WIDTH: u32 = 1600;
const WAVE_HEIGHT: u32 = 200;
/// Semitones shown by the spectrogram (MIDI numbers), two pixels each.
const LOWEST: u8 = 33;
const HIGHEST: u8 = 96;
const ROW: u32 = 2;
const WINDOW: usize = 4096;
const BACKGROUND: Rgba = Rgba([24, 26, 32, 255]);
const SILENT: Rgba = Rgba([40, 30, 34, 255]);
const WAVE: Rgba = Rgba([140, 200, 255, 255]);
/// 100 ms windows quieter than this count as silence (as in the tests).
const SILENCE_RMS: f32 = 0.005;

/// `<dir>/<namespace>/<theme>-<n>.wav` and `.png` for every composed piece.
pub fn render_music(content: &Content, dir: &Path) -> anyhow::Result<()> {
    for theme in &content.sounds.themes {
        let folder = dir.join(theme.id.namespace());
        std::fs::create_dir_all(&folder)?;
        for index in 0..theme.def.pieces {
            let piece = Piece {
                theme,
                index,
                mood: Mood::Calm,
            };
            let started = Instant::now();
            let samples = piece.render();
            let render_seconds = started.elapsed().as_secs_f32();
            let base = folder.join(format!("{}-{index}", theme.id.name()));
            let wav = base.with_extension("wav");
            write_wav(&wav, &samples).with_context(|| wav.display().to_string())?;
            let png = base.with_extension("png");
            write_png(&png, &picture(&samples))?;
            let size = std::fs::metadata(&wav).map(|m| m.len()).unwrap_or(0);
            tracing::info!(
                theme = %theme.id,
                index,
                seconds = samples.len() as f32 / SAMPLE_RATE as f32,
                render_seconds,
                megabytes = size as f32 / 1_000_000.0,
                silence = silence_share(&samples),
                "piece written"
            );
        }
    }
    tracing::info!(dir = %dir.display(), "music written");
    Ok(())
}

fn silence_share(samples: &[f32]) -> f32 {
    let windows: Vec<bool> = samples
        .chunks(SAMPLE_RATE as usize / 10)
        .map(|w| rms(w) < SILENCE_RMS)
        .collect();
    windows.iter().filter(|silent| **silent).count() as f32 / windows.len().max(1) as f32
}

fn rms(window: &[f32]) -> f32 {
    (window.iter().map(|s| s * s).sum::<f32>() / window.len().max(1) as f32).sqrt()
}

fn picture(samples: &[f32]) -> RgbaImage {
    let rows = u32::from(HIGHEST - LOWEST + 1) * ROW;
    let mut image = RgbaImage::new(WIDTH, WAVE_HEIGHT + rows, BACKGROUND);
    let per_column = samples.len().div_ceil(WIDTH as usize).max(1);
    for (x, column) in samples.chunks(per_column).enumerate() {
        draw_wave(&mut image, x as u32, column);
        draw_notes(&mut image, x as u32, column);
    }
    image
}

fn draw_wave(image: &mut RgbaImage, x: u32, column: &[f32]) {
    let (low, high) = column
        .iter()
        .fold((0.0f32, 0.0f32), |(l, h), s| (l.min(*s), h.max(*s)));
    let middle = WAVE_HEIGHT as f32 / 2.0;
    let to_y = |v: f32| (middle - v * middle).clamp(0.0, WAVE_HEIGHT as f32 - 1.0) as u32;
    if rms(column) < SILENCE_RMS {
        for y in 0..WAVE_HEIGHT {
            image.set(x, y, SILENT);
        }
    }
    for y in to_y(high)..=to_y(low) {
        image.set(x, y, WAVE);
    }
}

/// Energy of each semitone in the window at the start of the column (Goertzel).
fn draw_notes(image: &mut RgbaImage, x: u32, column: &[f32]) {
    let window = &column[..column.len().min(WINDOW)];
    for note in LOWEST..=HIGHEST {
        let level = goertzel(window, hertz(note));
        let brightness = ((level.max(1e-6).log10() + 4.0) / 3.0).clamp(0.0, 1.0);
        let shade = (brightness * 255.0) as u8;
        let color = Rgba([shade, (f32::from(shade) * 0.8) as u8, 60, 255]);
        let top = WAVE_HEIGHT + u32::from(HIGHEST - note) * ROW;
        for y in top..top + ROW {
            image.set(x, y, color);
        }
    }
}

fn goertzel(window: &[f32], hz: f32) -> f32 {
    let w = std::f32::consts::TAU * hz / SAMPLE_RATE as f32;
    let coefficient = 2.0 * w.cos();
    let (mut s1, mut s2) = (0.0f32, 0.0f32);
    for (i, sample) in window.iter().enumerate() {
        // Hann window: no smearing from the cut edges.
        let hann = 0.5 - 0.5 * (std::f32::consts::TAU * i as f32 / window.len() as f32).cos();
        let s0 = sample * hann + coefficient * s1 - s2;
        s2 = s1;
        s1 = s0;
    }
    let power = s1 * s1 + s2 * s2 - coefficient * s1 * s2;
    power.max(0.0).sqrt() / window.len().max(1) as f32
}
