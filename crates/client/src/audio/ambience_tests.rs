#![allow(clippy::unwrap_used)]

use super::tests::{block, content, sound_id};
use super::*;

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
        assert_eq!(cue.at.y, 20.5, "only the surface bubbles");
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
