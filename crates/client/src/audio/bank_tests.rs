#![allow(clippy::unwrap_used)]

use super::bank::SoundBank;
use super::output::Output;
use super::tests::{block, content, hearing};
use super::*;
use std::time::{Duration, Instant};
use strata_core::audio::Rng;

#[test]
fn no_sound_is_missing_after_loading() {
    let mut bank = SoundBank::new(content().sounds.clone());
    bank.prepare_all();
    bank.wait_ready();
    let megabytes = bank.cached_samples() as f32 * 4.0 / 1e6;
    assert!(bank.is_complete());
    assert!(megabytes < 64.0, "{megabytes} MB");
    let mut rng = Rng::new(5);
    for (id, _, sound) in content().sounds.registry.iter() {
        for _ in 0..sound.def.variants * 4 {
            assert!(bank.pick(id, &mut rng).is_some(), "{id:?}");
        }
    }
}

#[test]
fn a_sound_asked_on_the_first_frame_is_played() {
    let (output, commands) = Output::detached();
    let settings = SoundSettings::default();
    let mut audio = Audio::with_output(Some(output), content(), &settings, Default::default());
    let world = ClientWorld::default();
    let ears = listener_at(DVec3::new(0.5, 70.5, 0.5), 0.0);
    audio.block_changed(BlockPos::new(2, 70, 0), block("stone"), BlockId::AIR);
    let started = Instant::now();
    let mut frame = started;
    let mut played = false;
    while !played && started.elapsed() < Duration::from_secs(2) {
        let seconds = frame.elapsed().as_secs_f32();
        frame = Instant::now();
        audio.update(seconds, &hearing(ears, None, &world));
        played = commands.try_iter().any(|command| {
            matches!(command, Command::Play(voice) if voice.category == SoundCategory::Blocks)
        });
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(played, "the break was dropped");
}

#[test]
fn a_sound_too_late_is_dropped() {
    let (output, commands) = Output::detached();
    let settings = SoundSettings::default();
    let mut audio = Audio::with_output(Some(output), content(), &settings, Default::default());
    let world = ClientWorld::default();
    let ears = listener_at(DVec3::new(0.5, 70.5, 0.5), 0.0);
    audio.block_changed(BlockPos::new(2, 70, 0), block("stone"), BlockId::AIR);
    audio.update(0.0, &hearing(ears, None, &world));
    let first = commands
        .try_iter()
        .filter(|c| matches!(c, Command::Play(_)))
        .count();
    audio.update(MAX_LATE * 2.0, &hearing(ears, None, &world));
    audio.bank.wait_ready();
    audio.update(0.0, &hearing(ears, None, &world));
    let later = commands
        .try_iter()
        .filter(|c| matches!(c, Command::Play(_)))
        .count();
    assert!(audio.late.is_empty());
    assert!(first + later <= 1, "played at most once");
}
