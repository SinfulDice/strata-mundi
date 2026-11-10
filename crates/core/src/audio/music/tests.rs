use super::cache::stale_name;
use super::theme::bound;
use super::*;
use crate::audio::{SAMPLE_RATE, load_sounds};
use crate::id::Identifier;
use crate::pack::PackSet;
use std::path::{Path, PathBuf};

fn terra() -> Theme {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&dir, &mut errors);
    let sounds = load_sounds(&packs, &mut errors);
    assert!(errors.is_empty(), "{errors:#?}");
    let id = Identifier::new("strata", "terra").unwrap();
    sounds.themes.into_iter().find(|t| t.id == id).unwrap()
}

/// Terra's theme cut short, to render quickly in tests.
fn short_terra() -> Theme {
    let mut theme = terra();
    theme.def.length = (14.0, 16.0);
    theme.def.rests = (2.0, 3.0);
    theme
}

fn piece(theme: &Theme, index: u8) -> Piece<'_> {
    Piece {
        theme,
        index,
        mood: Mood::Calm,
    }
}

fn score(theme: &Theme, index: u8) -> Score {
    compose(&theme.def, piece(theme, index).seed(), Mood::Calm)
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("strata-music-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// Share of 100 ms windows quieter than −46 dB.
fn silence_share(samples: &[f32]) -> f32 {
    let window = SAMPLE_RATE as usize / 10;
    let windows: Vec<f32> = samples
        .chunks(window)
        .map(|w| (w.iter().map(|s| s * s).sum::<f32>() / w.len() as f32).sqrt())
        .collect();
    windows.iter().filter(|rms| **rms < 0.005).count() as f32 / windows.len().max(1) as f32
}

#[test]
fn terra_has_its_theme_heard_on_the_title_screen() {
    let theme = terra();
    assert!(theme.heard_in(&Identifier::new("strata", "terra").unwrap()));
    assert!(theme.def.title);
    assert!(theme.recordings.is_empty());
    assert!(theme.def.pieces >= 3);
}

#[test]
fn composition_is_deterministic_for_a_seed() {
    let theme = terra();
    assert_eq!(score(&theme, 0), score(&theme, 0));
    assert_ne!(score(&theme, 0), score(&theme, 1));
}

#[test]
fn every_note_is_in_the_scale_of_the_theme() {
    let theme = terra();
    let steps = theme.def.scale.steps();
    for index in 0..theme.def.pieces {
        for note in &score(&theme, index).notes {
            let step = (i32::from(note.pitch) - i32::from(theme.def.root)).rem_euclid(12);
            assert!(steps.contains(&(step as u8)), "{note:?}");
        }
    }
}

#[test]
fn pieces_last_one_to_three_minutes_and_their_notes_fit() {
    let theme = terra();
    for index in 0..theme.def.pieces {
        let score = score(&theme, index);
        assert!((60.0..=180.0).contains(&score.seconds), "{}", score.seconds);
        assert!(score.notes.len() >= 10, "{} notes", score.notes.len());
        for note in &score.notes {
            assert!(note.start >= 0.0 && note.end(&theme.def) <= score.seconds);
        }
    }
}

#[test]
fn a_piece_is_mostly_silence_and_never_clips() {
    let theme = terra();
    let samples = piece(&theme, 0).render();
    let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
    assert!((60.0..=180.0).contains(&seconds));
    let peak = samples.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    assert!(peak <= 1.0 && peak > 0.05, "peak {peak}");
    let silence = silence_share(&samples);
    assert!(silence >= 0.3, "silence {silence}");
    assert!(silence <= 0.9, "silence {silence}");
}

#[test]
fn streaming_render_matches_the_whole_render() {
    let theme = short_terra();
    let piece = piece(&theme, 2);
    let whole = piece.render();
    let def = &theme.def;
    let mut renderer =
        PieceRenderer::new(def, compose(def, piece.seed(), Mood::Calm), piece.seed());
    let mut streamed = Vec::new();
    let mut block = vec![0.0; 777];
    loop {
        let count = renderer.fill(&mut block);
        if count == 0 {
            break;
        }
        streamed.extend_from_slice(&block[..count]);
    }
    assert_eq!(whole, streamed);
}

#[test]
fn pieces_are_rendered_once_then_read_from_the_cache() {
    let dir = temp_dir("cache");
    let theme = short_terra();
    let first = cached_piece(&dir, piece(&theme, 0)).unwrap();
    assert!(first.rendered);
    let again = cached_piece(&dir, piece(&theme, 0)).unwrap();
    assert_eq!(again.path, first.path);
    assert!(!again.rendered);
    let mut stream = WavStream::open(&first.path).unwrap();
    let whole = piece(&theme, 0).render();
    assert_eq!(stream.frames(), whole.len());
    let mut read = Vec::new();
    loop {
        let block = stream.read(10_000).unwrap();
        if block.is_empty() {
            break;
        }
        read.extend(block);
    }
    assert_eq!(read.len(), whole.len());
    let worst = read
        .iter()
        .zip(&whole)
        .fold(0.0f32, |w, (a, b)| w.max((a - b).abs()));
    assert!(worst < 1e-4, "{worst}");
}

#[test]
fn a_new_algorithm_or_new_settings_make_the_piece_again() {
    let dir = temp_dir("stale");
    let theme = short_terra();
    let stale = dir.join(stale_name(piece(&theme, 0)));
    std::fs::write(&stale, b"old").unwrap();
    std::fs::write(dir.join("strata-other-calm-0-a1-0.wav"), b"other theme").unwrap();
    let cached = cached_piece(&dir, piece(&theme, 0)).unwrap();
    assert!(cached.rendered);
    let mut changed = theme.clone();
    changed.def.tempo += 1.0;
    assert_ne!(piece(&changed, 0).file_name(), piece(&theme, 0).file_name());
    assert_eq!(prune(&dir, std::slice::from_ref(&theme)).unwrap(), 1);
    assert!(!stale.exists() && cached.path.exists());
    assert!(dir.join("strata-other-calm-0-a1-0.wav").exists());
}

#[test]
fn a_damaged_cache_file_is_refused() {
    let dir = temp_dir("damaged");
    let path = dir.join("bad.wav");
    std::fs::write(&path, [0u8; 100]).unwrap();
    assert!(WavStream::open(&path).is_err());
}

#[test]
fn out_of_range_theme_values_are_corrected() {
    let mut def = terra().def;
    def.tempo = 1000.0;
    def.length = (500.0, 5.0);
    def.scale = Scale::Custom(vec![3, 20]);
    def.motifs[0].beats.pop();
    let mut problems = Vec::new();
    bound(&mut def, &mut problems);
    assert!(problems.len() >= 4, "{problems:?}");
    assert_eq!(def.tempo, 240.0);
    assert!(def.length.0 <= def.length.1 && def.length.1 <= MAX_PIECE_SECONDS);
    assert_eq!(def.scale, Scale::Major);
    assert_eq!(def.motifs[0].beats.len(), def.motifs[0].degrees.len());
}
