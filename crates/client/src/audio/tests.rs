#![allow(clippy::unwrap_used)]

use super::bank::SoundBank;
use super::decode::{Resampler, resample};
use super::*;
use glam::DVec3;
use std::sync::{Arc, OnceLock};
use strata_core::audio::{SAMPLE_RATE, SoundCategory};
use strata_core::content::SoundId;
use strata_core::id::Identifier;
use strata_core::pack::PackSet;

pub(super) fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        Content::load(&packs, &PackSet::default()).0
    })
}

pub(super) fn sound_id(name: &str) -> SoundId {
    let id = Identifier::new("strata", name).unwrap();
    content().sounds.registry.key(&id).unwrap()
}

pub(super) fn block(name: &str) -> BlockId {
    let id = Identifier::new("strata", name).unwrap();
    content().blocks.key(&id).unwrap()
}

fn voice(category: SoundCategory, at: Option<DVec3>) -> Voice {
    Voice {
        id: 0,
        buffer: vec![0.5; 4410].into(),
        category,
        gain: 1.0,
        pitch: 1.0,
        at,
        range: 16.0,
        muffle: 0.0,
    }
}

pub(super) fn hearing<'a>(
    listener: Listener,
    player: Option<&'a Body>,
    world: &'a ClientWorld,
) -> Hearing<'a> {
    Hearing {
        listener,
        player,
        world,
        biomes: &[],
        daylight: 1.0,
        place: MusicPlace::Title,
        paused: false,
    }
}

fn peaks(mixer: &mut Mixer, frames: usize) -> [f32; 2] {
    let mut out = vec![0.0; frames * 2];
    mixer.render(&mut out);
    let mut peaks = [0.0f32; 2];
    for frame in out.chunks(2) {
        peaks[0] = peaks[0].max(frame[0].abs());
        peaks[1] = peaks[1].max(frame[1].abs());
    }
    peaks
}

#[test]
fn categories_are_mixed_by_their_volume() {
    let mut settings = crate::settings::SoundSettings {
        ambient: 0,
        ..Default::default()
    };
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Volumes(Volumes::of(&settings)));
    mixer.apply(Command::Play(voice(SoundCategory::Ambient, None)));
    assert_eq!(mixer.voices(), 0, "a muted category plays nothing");
    mixer.apply(Command::Play(voice(SoundCategory::Blocks, None)));
    let full = peaks(&mut mixer, 256);
    assert!((full[0] - 0.5 * std::f32::consts::FRAC_1_SQRT_2).abs() < 1e-3);
    settings.master = 50;
    mixer.apply(Command::Volumes(Volumes::of(&settings)));
    peaks(&mut mixer, 256);
    let half = peaks(&mut mixer, 256);
    assert!((half[0] - full[0] * 0.25).abs() < 1e-3, "{half:?}");
}

#[test]
fn distance_attenuates_and_side_pans() {
    assert_eq!(attenuation(0.5, 16.0), 1.0);
    assert!(attenuation(4.0, 16.0) < attenuation(2.0, 16.0));
    assert_eq!(attenuation(16.0, 16.0), 0.0);
    assert!(attenuation(15.0, 16.0) < 0.1 * attenuation(10.0, 16.0) + 0.05);
    // Yaw 0 looks towards −z: +x is on the right.
    let ears = listener_at(DVec3::ZERO, 0.0);
    let [left, right] = stereo_gains(&ears, DVec3::new(4.0, 0.0, 0.0), 16.0);
    assert!(right > 0.2 && left < 0.01, "{left} {right}");
    let [left, right] = stereo_gains(&ears, DVec3::new(-4.0, 0.0, 0.0), 16.0);
    assert!(left > right);
    let front = stereo_gains(&ears, DVec3::new(0.0, 0.0, -4.0), 16.0);
    let behind = stereo_gains(&ears, DVec3::new(0.0, 0.0, 4.0), 16.0);
    assert!((front[0] - front[1]).abs() < 1e-5 && behind[0] < front[0]);
    // Turned a quarter to the right (looking towards +x), the same source is in front.
    let turned = listener_at(DVec3::ZERO, std::f32::consts::FRAC_PI_2);
    let [left, right] = stereo_gains(&turned, DVec3::new(4.0, 0.0, 0.0), 16.0);
    assert!((left - right).abs() < 1e-4);
    assert_eq!(
        stereo_gains(&ears, DVec3::new(40.0, 0.0, 0.0), 16.0),
        [0.0; 2]
    );
}

#[test]
fn moving_the_listener_moves_the_sound() {
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Play(voice(
        SoundCategory::Blocks,
        Some(DVec3::new(3.0, 0.0, 0.0)),
    )));
    let [left, right] = peaks(&mut mixer, 128);
    assert!(right > left);
    mixer.apply(Command::Listener(listener_at(
        DVec3::new(6.0, 0.0, 0.0),
        0.0,
    )));
    peaks(&mut mixer, 128);
    let [left, right] = peaks(&mut mixer, 128);
    assert!(left > right);
}

#[test]
fn voices_are_limited_and_pitch_changes_length() {
    let mut mixer = Mixer::new(SAMPLE_RATE);
    let far = Some(DVec3::new(10.0, 0.0, 0.0));
    for _ in 0..MAX_VOICES + 5 {
        mixer.apply(Command::Play(voice(SoundCategory::Blocks, far)));
    }
    assert_eq!(mixer.voices(), MAX_VOICES);
    mixer.apply(Command::Play(voice(SoundCategory::Blocks, None)));
    let [left, _] = peaks(&mut mixer, 64);
    assert!(left > 0.35, "the near sound replaced a far one: {left}");
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Play(Voice {
        pitch: 2.0,
        ..voice(SoundCategory::Blocks, None)
    }));
    peaks(&mut mixer, 2200);
    assert_eq!(mixer.voices(), 1);
    peaks(&mut mixer, 10);
    assert_eq!(mixer.voices(), 0, "twice as fast, half as long");
}

#[test]
fn subtitles_tell_the_direction_and_expire() {
    let ears = listener_at(DVec3::ZERO, 0.0);
    assert_eq!(
        Direction::of(&ears, DVec3::new(5.0, 0.0, 1.0)),
        Direction::Right
    );
    assert_eq!(
        Direction::of(&ears, DVec3::new(-5.0, 2.0, 0.0)),
        Direction::Left
    );
    assert_eq!(
        Direction::of(&ears, DVec3::new(1.0, 0.0, -6.0)),
        Direction::Front
    );
    assert_eq!(
        Direction::of(&ears, DVec3::new(0.0, 0.0, 6.0)),
        Direction::Behind
    );
    assert_eq!(
        Direction::of(&ears, DVec3::new(0.5, -1.0, 0.0)),
        Direction::Here
    );
    let mut subtitles = Subtitles::default();
    subtitles.heard("subtitle-splash", Direction::Left);
    subtitles.update(LIFETIME * 0.6);
    subtitles.heard("subtitle-footsteps", Direction::Front);
    subtitles.update(LIFETIME * 0.6);
    let keys: Vec<&str> = subtitles.lines().iter().map(|l| l.key.as_str()).collect();
    assert_eq!(keys, ["subtitle-footsteps"], "the splash expired");
    subtitles.heard("subtitle-footsteps", Direction::Behind);
    assert_eq!(subtitles.lines()[0].direction, Direction::Behind);
    assert_eq!(
        subtitles.lines().len(),
        1,
        "heard again: refreshed, not added"
    );
    for index in 0..MAX_LINES + 2 {
        subtitles.heard(&format!("key-{index}"), Direction::Here);
        subtitles.update(0.01);
    }
    assert_eq!(subtitles.lines().len(), MAX_LINES);
    assert_eq!(
        subtitles.lines().last().unwrap().key,
        format!("key-{}", MAX_LINES + 1)
    );
}

#[test]
fn two_plays_differ_and_buffers_are_cached() {
    let mut bank = SoundBank::new(content().sounds.clone());
    let mut rng = strata_core::audio::Rng::new(3);
    let step = sound_id("step_stone");
    assert!(
        bank.pick(step, &mut rng).is_none(),
        "made in the background"
    );
    assert_eq!(bank.cached_samples(), 0);
    for _ in 0..50 {
        bank.pick(step, &mut rng);
        bank.wait_ready();
    }
    let a = bank.pick(step, &mut rng).unwrap();
    let b = bank.pick(step, &mut rng).unwrap();
    assert!(!Arc::ptr_eq(&a.buffer, &b.buffer), "another variant");
    assert_ne!(a.pitch, b.pitch);
    let variants = usize::from(bank.sound(step).unwrap().def.variants);
    let cached = bank.cached_samples();
    assert!(cached >= a.buffer.len() + b.buffer.len());
    assert!(cached <= variants * (0.14 * SAMPLE_RATE as f32) as usize);
    assert_eq!(resample(&[0.0, 1.0, 0.0, -1.0], SAMPLE_RATE * 2).len(), 2);
}

#[test]
fn block_changes_break_and_place() {
    let table = SoundTable::from_content(content());
    let pos = BlockPos::new(3, 70, -2);
    let (stone, air, water) = (block("stone"), BlockId::AIR, block("water"));
    let broken = table.changed(pos, stone, air).unwrap();
    assert_eq!(broken.sound, sound_id("break_stone"));
    assert_eq!(broken.at, DVec3::new(3.5, 70.5, -1.5));
    assert_eq!(
        table.changed(pos, air, stone).unwrap().sound,
        sound_id("place_stone")
    );
    assert_eq!(
        table.changed(pos, block("glass"), water).unwrap().sound,
        sound_id("break_glass")
    );
    assert_eq!(
        table.changed(pos, air, water),
        None,
        "flowing water is silent"
    );
}

#[test]
fn steps_follow_the_ground_and_water_splashes() {
    let table = SoundTable::from_content(content());
    let mut world = ClientWorld::default();
    for x in 0..40 {
        world.set_block(x, 10, 0, block("gravel"));
        world.set_block(x, 20, 0, block("water"));
    }
    let mut steps = Footsteps::default();
    let mut cues = Vec::new();
    let mut body = Body::at(DVec3::new(0.5, 11.0, 0.5));
    body.on_ground = true;
    for _ in 0..69 {
        body.position.x += 0.25;
        steps.update(&body, &world, &table, &mut cues);
    }
    assert_eq!(cues.len(), 10, "one step per 1.6 blocks over 17");
    assert!(
        cues.iter()
            .all(|c| c.sound == sound_id("step_gravel") && !c.subtitled)
    );
    cues.clear();
    body.on_ground = false;
    body.velocity.y = -8.0;
    body.position = DVec3::new(0.5, 22.0, 0.5);
    steps.update(&body, &world, &table, &mut cues);
    body.position.y = 20.5;
    steps.update(&body, &world, &table, &mut cues);
    assert_eq!(cues.len(), 1);
    assert_eq!(cues[0].sound, sound_id("water_splash"));
}

#[test]
fn without_a_device_nothing_panics() {
    let settings = crate::settings::SoundSettings {
        subtitles: true,
        ..Default::default()
    };
    for mut audio in [
        Audio::silent(content(), &settings),
        Audio::start(
            content(),
            &settings,
            std::env::temp_dir().join("strata-music-test"),
        ),
    ] {
        let world = ClientWorld::default();
        let ears = listener_at(DVec3::new(0.0, 70.0, 0.0), 1.0);
        audio.block_changed(BlockPos::new(2, 70, 0), block("stone"), BlockId::AIR);
        let body = Body::at(DVec3::ZERO);
        audio.update(0.05, &hearing(ears, Some(&body), &world));
        audio.apply_settings(&crate::settings::SoundSettings::default());
        audio.update(0.05, &hearing(ears, None, &world));
        audio.set_content(content());
    }
    let mut audio = Audio::silent(content(), &settings);
    audio.block_changed(BlockPos::new(-3, 70, 0), block("stone"), BlockId::AIR);
    let ears = listener_at(DVec3::new(0.5, 70.5, 0.5), 0.0);
    audio.update(0.05, &hearing(ears, None, &ClientWorld::default()));
    let line = &audio.subtitles()[0];
    assert_eq!(
        (line.key.as_str(), line.direction),
        ("subtitle-block-broken", Direction::Left)
    );
}

#[test]
fn a_stream_resampled_block_by_block_matches_the_whole() {
    let input: Vec<f32> = (0..10_000).map(|i| (i as f32 * 0.01).sin()).collect();
    for rate in [22_050, 32_000, 48_000, 96_000] {
        let whole = resample(&input, rate);
        for block in [1, 7, 512, 4410] {
            let mut resampler = Resampler::new(rate);
            let mut streamed = Vec::new();
            for chunk in input.chunks(block) {
                resampler.push(chunk, &mut streamed);
            }
            assert_eq!(streamed.len(), whole.len(), "{rate} Hz by {block}");
            let worst = streamed
                .iter()
                .zip(&whole)
                .fold(0.0f32, |w, (a, b)| w.max((a - b).abs()));
            assert!(worst < 1e-5, "{rate} Hz by {block}: {worst}");
        }
    }
}

#[test]
fn a_huge_fill_starts_few_sounds() {
    let settings = SoundSettings::default();
    let mut audio = Audio::silent(content(), &settings);
    for x in 0..100 {
        for z in 0..100 {
            audio.block_changed(BlockPos::new(x, 70, z), block("stone"), BlockId::AIR);
        }
    }
    assert!(audio.cues.len() <= MAX_STARTS);
    let near = audio
        .cues
        .iter()
        .filter(|cue| cue.at.distance(DVec3::new(0.5, 70.5, 0.5)) < 3.0)
        .count();
    assert_eq!(near, 1, "neighbouring breaks are heard as one");
}
