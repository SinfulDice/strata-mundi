#![allow(clippy::unwrap_used)]

use super::output::Output;
use super::tests::{block, content, sound_id};
use super::*;
use strata_core::audio::{SAMPLE_RATE, write_wav};
use strata_core::content::BiomeTints;
use strata_core::id::Identifier;

/// A lava lake of 12 by 12 blocks, 3 deep, its surface at y = 20.
fn lava_lake() -> ClientWorld {
    let mut world = ClientWorld::default();
    for x in 0..12 {
        for z in 0..12 {
            for y in 18..=20 {
                world.set_block(x, y, z, block("lava"));
            }
        }
    }
    world
}

#[test]
fn a_lava_lake_bubbles_at_its_surface_within_hearing() {
    let (world, table) = (lava_lake(), SoundTable::from_content(content()));
    let bubble = sound_id("lava_bubble");
    let ears = DVec3::new(-3.0, 22.6, 6.0);
    let mut ambient = AmbientBlocks::default();
    let (mut rng, mut cues) = (strata_core::audio::Rng::new(9), Vec::new());
    for _ in 0..600 {
        ambient.update(0.05, ears, (&world, &table), &mut rng, &mut cues);
    }
    let range = f64::from(content().sounds.get(bubble).unwrap().def.range);
    assert!(cues.len() >= 15, "{} bubbles in 30 s", cues.len());
    assert!(cues.len() <= 120, "{} bubbles in 30 s", cues.len());
    for cue in &cues {
        assert_eq!(cue.sound, bubble);
        assert!(cue.subtitled);
        assert_eq!(cue.at.y, 21.1, "only the surface bubbles, from above it");
        assert!(cue.at.distance(ears) < range, "heard: {:?}", cue.at);
    }
}

#[test]
fn a_bubble_is_subtitled_when_heard() {
    let settings = SoundSettings {
        subtitles: true,
        ..Default::default()
    };
    let mut audio = Audio::silent(content(), &settings);
    audio.listener = listener_at(DVec3::new(-3.0, 22.6, 6.0), 0.0);
    let bubble = |x| Cue {
        sound: sound_id("lava_bubble"),
        at: DVec3::new(x, 20.5, 6.5),
        subtitled: true,
    };
    audio.play(bubble(40.5));
    assert!(audio.subtitles().is_empty(), "out of hearing");
    audio.play(bubble(8.5));
    assert_eq!(audio.subtitles()[0].key, "subtitle-lava-bubbles");
}

/// Loudest amplitude factor of a sound in the pack's ambiences (bed volume, sound volume,
/// altitude), and the shortest wait between two of its events.
fn loudest_and_most_often(name: &str) -> (f32, f32) {
    let id = sound_id(name);
    let volume = content().sounds.get(id).unwrap().def.volume;
    let mut loudest = 0.0f32;
    let mut often = f32::INFINITY;
    for ambience in &content().sounds.ambiences {
        for bed in ambience.beds.iter().filter(|bed| bed.sound == id) {
            let altitude = bed.altitude.map_or(1.0, |a| a.gain.0.max(a.gain.1));
            loudest = loudest.max(bed.volume * altitude * volume);
        }
        for event in ambience.events.iter().filter(|event| event.sound == id) {
            loudest = loudest.max(volume);
            often = often.min(event.every.0);
        }
    }
    (loudest, often)
}

#[test]
fn the_wind_is_soft_and_rare() {
    let (bed, _) = loudest_and_most_often("ambient_wind");
    assert!(bed > 0.0 && bed <= 0.2, "wind bed at {bed}");
    let (gust, often) = loudest_and_most_often("wind");
    assert!(gust > 0.0 && gust <= 0.25, "gust at {gust}");
    assert!(often >= 30.0, "a gust every {often} s at most");
}

#[test]
fn the_ambiences_slider_holds_every_ambience_and_no_block() {
    let sounds = &content().sounds;
    let category = |id| sounds.get(id).unwrap().def.category;
    for ambience in &sounds.ambiences {
        let beds = ambience.beds.iter().map(|bed| bed.sound);
        for id in beds.chain(ambience.events.iter().map(|event| event.sound)) {
            assert_eq!(category(id), SoundCategory::Ambient, "{id:?}");
        }
    }
    assert_eq!(category(sound_id("lava_bubble")), SoundCategory::Blocks);
    let muted = SoundSettings {
        ambient: 0,
        ..Default::default()
    };
    assert_eq!(muted.gain(SoundCategory::Ambient), 0.0);
    assert!(muted.gain(SoundCategory::Blocks) > 0.0);
}

/// The plains' wind bed, as loud as at sea level.
fn plains_wind() -> f32 {
    let plains = Identifier::new("strata", "plains").unwrap();
    let mut ambiences = content().sounds.ambiences.iter();
    let ambience = ambiences.find(|a| a.biomes.contains(&plains)).unwrap();
    let bed = ambience
        .beds
        .iter()
        .find(|b| b.sound == sound_id("ambient_wind"));
    let bed = bed.unwrap();
    bed.volume * bed.altitude.map_or(1.0, |a| a.gain.0)
}

/// Six seconds on the shore of a lava lake in the plains (facing it), a gust included, through the
/// whole chain (triggers, bank, mixer; the wind bed given by hand); written to `lava-wind.wav` in `STRATA_MIX_DIR`.
#[test]
fn a_lava_shore_in_the_wind_can_be_listened_to() {
    let mut world = lava_lake();
    world.set_biome_tints(vec![BiomeTints::default()].into());
    let biomes = [BiomeInfo {
        id: Identifier::new("strata", "plains").unwrap(),
        tints: BiomeTints::default(),
    }];
    let (output, commands) = Output::detached();
    let settings = super::bank_tests::quiet();
    let mut audio = Audio::with_output(Some(output), content(), &settings, Default::default());
    audio.bank.wait_ready();
    let ears = listener_at(DVec3::new(-3.0, 22.6, 6.0), std::f32::consts::FRAC_PI_2);
    let hearing = Hearing {
        listener: ears,
        player: None,
        world: &world,
        biomes: &biomes,
        daylight: 1.0,
        place: MusicPlace::Title,
        paused: false,
    };
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Listener(ears));
    let wind = sound_id("ambient_wind");
    mixer.apply(Command::Bed(BedVolume {
        sound: wind,
        buffer: audio.bank.bed(wind).unwrap(),
        category: SoundCategory::Ambient,
        gain: plains_wind(),
    }));
    let frame = SAMPLE_RATE as usize / 20;
    let mut stereo = vec![0.0; frame * 2];
    let mut mono: Vec<f32> = Vec::new();
    let mut bubbles = 0;
    for step in 0..120 {
        if step == 40 {
            audio.play(Cue {
                sound: sound_id("wind"),
                at: DVec3::new(-12.0, 24.0, -4.0),
                subtitled: true,
            });
        }
        audio.update(0.05, &hearing);
        for command in commands.try_iter() {
            if let Command::Play(voice) = &command {
                bubbles += usize::from(voice.category == SoundCategory::Blocks);
            }
            mixer.apply(command);
        }
        mixer.render(&mut stereo);
        mono.extend(stereo.chunks(2).map(|pair| (pair[0] + pair[1]) * 0.5));
    }
    assert!(bubbles >= 2, "{bubbles} bubbles in 6 s");
    assert!(mono.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
    if let Some(dir) = std::env::var_os("STRATA_MIX_DIR").map(std::path::PathBuf::from) {
        std::fs::create_dir_all(&dir).unwrap();
        write_wav(&dir.join("lava-wind.wav"), &mono).unwrap();
    }
}
