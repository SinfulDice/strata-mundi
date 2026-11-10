#![allow(clippy::unwrap_used)]

use super::*;
use crate::settings::MusicFrequency;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use strata_core::audio::music::Theme;
use strata_core::id::Identifier;
use strata_core::pack::PackSet;

fn terra_short() -> Theme {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let packs = PackSet::discover(&dir, &mut Vec::new());
    let content = Content::load(&packs, &PackSet::default()).0;
    let mut theme = content.sounds.themes.into_iter().next().unwrap();
    theme.def.length = (12.0, 13.0);
    theme.def.rests = (1.0, 2.0);
    theme.def.pieces = 2;
    theme
}

fn temp_dir(name: &str) -> PathBuf {
    let dir =
        std::env::temp_dir().join(format!("strata-client-music-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn terra() -> MusicPlace {
    MusicPlace::World(Some(DimensionId(
        Identifier::new("strata", "terra").unwrap(),
    )))
}

fn moment(seconds: f32, place: &MusicPlace, enabled: bool) -> MusicMoment<'_> {
    MusicMoment {
        seconds,
        place,
        paused: false,
        enabled,
        frequency: MusicFrequency::Normal,
    }
}

use strata_core::dimension::DimensionId;

#[test]
fn the_silence_between_pieces_stays_within_the_setting() {
    for frequency in MusicFrequency::ALL {
        let (low, high) = frequency.wait();
        for seed in 0..50 {
            let mut schedule = MusicSchedule::new((1.0, 2.0), seed);
            schedule.started();
            assert!(!schedule.update(10_000.0, false), "nothing while playing");
            schedule.finished(frequency);
            let wait = schedule.wait().unwrap();
            assert!((low..=high).contains(&wait), "{frequency:?}: {wait}");
            assert!(!schedule.update(low - 1.0, false));
            assert!(schedule.update(high, false));
        }
    }
    assert!(MusicFrequency::Rare.wait().0 > MusicFrequency::Often.wait().1);
}

#[test]
fn the_silence_is_not_counted_while_paused() {
    let mut schedule = MusicSchedule::new((5.0, 5.0), 1);
    assert!(!schedule.update(100.0, true));
    assert_eq!(schedule.wait(), Some(5.0));
    assert!(schedule.update(5.0, false));
}

#[test]
fn disabled_music_renders_and_plays_nothing() {
    let dir = temp_dir("off");
    let mut player = MusicPlayer::new(dir.clone(), vec![terra_short()]);
    let mut commands = 0;
    for _ in 0..100 {
        player.update(&moment(60.0, &MusicPlace::Title, false), &mut |_| {
            commands += 1;
            true
        });
    }
    assert_eq!(commands, 0);
    assert!(!player.is_rendering() && !player.is_playing());
    assert!(!dir.exists(), "nothing written to the cache");
}

#[test]
fn the_first_piece_of_a_world_comes_after_a_short_silence() {
    let mut player = MusicPlayer::new(temp_dir("first"), Vec::new());
    let place = terra();
    player.update(&moment(0.0, &place, true), &mut |_| true);
    let (low, high) = MusicFrequency::Normal.wait();
    let wait = player.wait().unwrap();
    assert!((low * 0.1..=high * 0.1).contains(&wait), "{wait}");
    player.update(&moment(0.0, &MusicPlace::Title, true), &mut |_| true);
    let wait = player.wait().unwrap();
    assert!((TITLE_WAIT.0..=TITLE_WAIT.1).contains(&wait), "{wait}");
}

/// Updates the player once per simulated second until it starts a piece.
fn wait_for_start(player: &mut MusicPlayer, place: &MusicPlace) -> MusicFeed {
    for _ in 0..600 {
        let mut started = None;
        player.update(&moment(1.0, place, true), &mut |command| {
            if let Command::Music(MusicCommand::Start(feed)) = command {
                started = Some(feed);
            }
            true
        });
        if let Some(feed) = started {
            return feed;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    panic!("no piece started");
}

#[test]
fn a_piece_is_rendered_in_the_background_then_streamed_and_faded_out() {
    let dir = temp_dir("play");
    let mut player = MusicPlayer::new(dir.clone(), vec![terra_short()]);
    let feed = wait_for_start(&mut player, &MusicPlace::Title);
    assert!(player.is_playing() && player.wait().is_none());
    let cached: Vec<_> = std::fs::read_dir(&dir).unwrap().flatten().collect();
    assert_eq!(cached.len(), 1, "one piece in the cache");
    let done = Arc::clone(&feed.done);
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Music(MusicCommand::Start(feed)));
    let mut heard = 0.0f32;
    let mut out = vec![0.0; 2 * 4410];
    // A late chunk is silence that does not move the piece on: wait for the reader.
    for _ in 0..10_000 {
        mixer.render(&mut out);
        heard = out.iter().fold(heard, |p, s| p.max(s.abs()));
        if heard > 0.01 {
            break;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
    assert!(heard > 0.01, "the piece is heard: {heard}");
    let mut sent = Vec::new();
    player.update(&moment(0.1, &terra(), true), &mut |c| {
        sent.push(c);
        true
    });
    assert!(matches!(
        sent.as_slice(),
        [Command::Music(MusicCommand::FadeOut)]
    ));
    mixer.apply(Command::Music(MusicCommand::FadeOut));
    for _ in 0..(MUSIC_FADE as usize * 10 + 2) {
        mixer.render(&mut out);
    }
    assert!(!mixer.music_playing() && done.load(Ordering::Acquire));
}

fn feed_of(seconds: f32) -> (MusicFeed, Arc<AtomicBool>) {
    let (sender, chunks) = crossbeam_channel::unbounded();
    for _ in 0..(seconds * 2.0) as usize {
        sender.send(vec![0.5; SAMPLE_RATE as usize / 2]).unwrap();
    }
    let done = Arc::new(AtomicBool::new(false));
    let feed = MusicFeed {
        chunks,
        recycle: crossbeam_channel::unbounded().0,
        done: Arc::clone(&done),
    };
    (feed, done)
}

fn block_peak(mixer: &mut Mixer, seconds: f32) -> f32 {
    let mut out = vec![0.0; 2 * (seconds * SAMPLE_RATE as f32) as usize];
    mixer.render(&mut out);
    out.iter().fold(0.0f32, |p, s| p.max(s.abs()))
}

#[test]
fn music_fades_in_plays_to_the_end_then_is_let_go() {
    let (feed, done) = feed_of(5.0);
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Music(MusicCommand::Start(feed)));
    let first = block_peak(&mut mixer, 0.5);
    let _ = block_peak(&mut mixer, MUSIC_FADE);
    let later = block_peak(&mut mixer, 0.5);
    assert!(first < later * 0.5, "{first} then {later}");
    assert!(mixer.music_playing());
    let _ = block_peak(&mut mixer, 2.0);
    assert!(!mixer.music_playing() && done.load(Ordering::Acquire));
}

#[test]
fn music_at_volume_zero_is_silent() {
    let (feed, _) = feed_of(5.0);
    let mut mixer = Mixer::new(SAMPLE_RATE);
    let settings = SoundSettings {
        music: 0,
        ..SoundSettings::default()
    };
    mixer.apply(Command::Volumes(Volumes::of(&settings)));
    mixer.apply(Command::Music(MusicCommand::Start(feed)));
    assert_eq!(block_peak(&mut mixer, 4.0), 0.0);
}

#[test]
fn a_refused_start_is_asked_again_and_a_dropped_feed_ends_the_piece() {
    let dir = temp_dir("lost");
    let mut player = MusicPlayer::new(dir, vec![terra_short()]);
    let place = MusicPlace::Title;
    let mut refused = 0;
    for _ in 0..600 {
        player.update(&moment(1.0, &place, true), &mut |command| {
            refused += usize::from(matches!(command, Command::Music(MusicCommand::Start(_))));
            false
        });
        if refused > 0 {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(refused, 1, "a piece was offered");
    assert!(!player.is_playing() && player.wait() == Some(0.0));
    let feed = wait_for_start(&mut player, &place);
    assert!(player.is_playing());
    drop(feed);
    player.update(&moment(0.1, &place, true), &mut |_| true);
    assert!(!player.is_playing());
    assert!(
        player.wait().is_some_and(|wait| wait > 0.0),
        "silence again"
    );
}

#[test]
fn controls_are_kept_and_never_queued() {
    let controls = controls::Controls::new(Volumes::default(), true);
    let listener = listener_at(DVec3::new(1.0, 2.0, 3.0), 0.5);
    assert!(controls.take(Command::Listener(listener)).is_none());
    assert!(controls.take(Command::Room(Room::DRY)).is_none());
    assert!(
        controls
            .take(Command::Music(MusicCommand::FadeOut))
            .is_some()
    );
    let mut mixer = Mixer::new(SAMPLE_RATE);
    let now = controls.read();
    let mut applied = 0;
    now.changes(None, &mut |command| {
        applied += 1;
        mixer.apply(command);
    });
    assert_eq!(applied, 4);
    now.changes(Some(&now), &mut |_| applied += 1);
    assert_eq!(applied, 4, "nothing changed, nothing applied");
}

#[test]
fn the_audio_thread_frees_nothing_itself() {
    let (bin, trash) = trash::Trash::new();
    let mut mixer = Mixer::with_trash(SAMPLE_RATE, bin);
    let (sender, chunks) = crossbeam_channel::unbounded();
    let (recycle, spares) = crossbeam_channel::bounded(2);
    for _ in 0..4 {
        sender.send(vec![0.5; 1000]).unwrap();
    }
    drop(sender);
    let done = Arc::new(AtomicBool::new(false));
    let feed = MusicFeed {
        chunks,
        recycle,
        done: Arc::clone(&done),
    };
    mixer.apply(Command::Music(MusicCommand::Start(feed)));
    mixer.apply(Command::Play(Voice {
        id: 1,
        buffer: vec![0.5; 100].into(),
        category: SoundCategory::Ambient,
        gain: 1.0,
        pitch: 1.0,
        at: None,
        range: 16.0,
        muffle: 0.0,
    }));
    let mut out = vec![0.0; 2 * 8192];
    mixer.render(&mut out);
    assert!(!mixer.music_playing() && mixer.voices() == 0);
    assert!(done.load(Ordering::Acquire));
    assert_eq!(
        spares.try_iter().count(),
        2,
        "played chunks go back to the reader"
    );
    let thrown = trash.try_iter().count();
    assert_eq!(
        thrown, 3,
        "the voice, a chunk the reader had no room for, the stream"
    );
}

#[test]
fn a_reload_keeps_the_one_render_thread() {
    let mut player = MusicPlayer::new(temp_dir("reload"), vec![terra_short()]);
    player.update(&moment(0.0, &MusicPlace::Title, true), &mut |_| true);
    assert!(player.is_rendering());
    player.set_themes(vec![terra_short()]);
    assert!(
        player.is_rendering(),
        "the same thread renders for the new packs"
    );
    drop(wait_for_start(&mut player, &MusicPlace::Title));
}
