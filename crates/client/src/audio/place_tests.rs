#![allow(clippy::unwrap_used)]

use super::*;
use std::sync::OnceLock;
use strata_core::audio::{SAMPLE_RATE, write_wav};
use strata_core::chunk::BiomeId;
use strata_core::content::{BiomeTints, SoundId};
use strata_core::id::Identifier;
use strata_core::pack::PackSet;

fn content() -> &'static Content {
    static CONTENT: OnceLock<Content> = OnceLock::new();
    CONTENT.get_or_init(|| {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        Content::load(&packs, &PackSet::default()).0
    })
}

fn sound_id(name: &str) -> SoundId {
    let id = Identifier::new("strata", name).unwrap();
    content().sounds.registry.key(&id).unwrap()
}

const BIOMES: [&str; 5] = ["plains", "forest", "ocean", "swamp", "mountains"];

fn table() -> AmbienceTable {
    let mut table = AmbienceTable::new(&content().sounds);
    let ids: Vec<Identifier> = BIOMES
        .iter()
        .map(|name| Identifier::new("strata", name).unwrap())
        .collect();
    table.set_biomes(ids.iter());
    table
}

fn biome(name: &str) -> BiomeId {
    BiomeId(BIOMES.iter().position(|b| *b == name).unwrap() as u16)
}

fn beds(place: &Place) -> Vec<(SoundId, f32)> {
    let mut targets = bed_targets(&table(), place);
    targets.retain(|(_, gain)| *gain > 0.0);
    targets
}

fn heard(beds: &[(SoundId, f32)], name: &str) -> f32 {
    let sound = sound_id(name);
    beds.iter()
        .find(|(s, _)| *s == sound)
        .map_or(0.0, |(_, gain)| *gain)
}

/// A world made of rock below `surface` with hollow boxes in it, under a sky light.
struct Rock {
    surface: i32,
    hollows: Vec<(BlockPos, BlockPos)>,
    sky: u8,
}

impl Rock {
    fn open_air() -> Self {
        Self {
            surface: 60,
            hollows: Vec::new(),
            sky: 15,
        }
    }

    /// A closed room of `size` blocks (x, y, z) from (0, 30, 0), the surface at `surface`.
    fn room(size: (i32, i32, i32), surface: i32) -> Self {
        let max = BlockPos::new(size.0 - 1, 30 + size.1 - 1, size.2 - 1);
        Self {
            surface,
            hollows: vec![(BlockPos::new(0, 30, 0), max)],
            sky: 0,
        }
    }
}

impl Surroundings for Rock {
    fn density(&self, pos: BlockPos) -> f32 {
        let hollow = self.hollows.iter().any(|(min, max)| {
            (min.x..=max.x).contains(&pos.x)
                && (min.y..=max.y).contains(&pos.y)
                && (min.z..=max.z).contains(&pos.z)
        });
        if pos.y <= self.surface && !hollow {
            1.0
        } else {
            0.0
        }
    }

    fn sky_light(&self, _: BlockPos) -> u8 {
        self.sky
    }

    fn biome(&self, x: i32, _: i32) -> Option<BiomeId> {
        Some(if x < 100 {
            biome("plains")
        } else {
            biome("ocean")
        })
    }

    fn surface(&self, _: i32, _: i32) -> Option<i32> {
        Some(self.surface)
    }
}

fn space_of(world: &Rock, head: DVec3) -> Space {
    let mut probe = SpaceProbe::default();
    let under = underground(world, head);
    for _ in 0..4 {
        probe.update(0.03, world, head, under);
    }
    probe.space()
}

#[test]
fn each_biome_and_the_underground_has_its_ambience() {
    let table = table();
    let at = |name: &str, under: f32, daylight: f32, y: f64| {
        let biomes = [(biome(name), 1.0)];
        Place::new(&table, &biomes, under, DVec3::new(0.0, y, 0.0), daylight)
    };
    let plains = beds(&at("plains", 0.0, 1.0, 125.0));
    assert!(heard(&plains, "ambient_wind") > 0.0);
    assert_eq!(heard(&plains, "ambient_cave"), 0.0);
    let forest = beds(&at("forest", 0.0, 1.0, 125.0));
    assert!(heard(&forest, "ambient_leaves") > heard(&forest, "ambient_wind"));
    assert!(heard(&beds(&at("ocean", 0.0, 1.0, 121.0)), "ambient_waves") > 0.0);
    let peak = beds(&at("mountains", 0.0, 1.0, 330.0));
    let foot = beds(&at("mountains", 0.0, 1.0, 140.0));
    assert!(heard(&peak, "ambient_wind") > 2.0 * heard(&foot, "ambient_wind"));
    let swamp_night = beds(&at("swamp", 0.0, 0.0, 120.0));
    let swamp_day = beds(&at("swamp", 0.0, 1.0, 120.0));
    assert!(heard(&swamp_night, "ambient_insects") > 0.0);
    assert_eq!(heard(&swamp_day, "ambient_insects"), 0.0);
    let cave = beds(&at("plains", 1.0, 1.0, 40.0));
    assert_eq!(cave.len(), 1);
    assert_eq!(cave[0].0, sound_id("ambient_cave"));
    // Near the sea, part of the samples fall on it: the waves are heard a little.
    let shore = biomes_around(&Rock::open_air(), DVec3::new(95.0, 61.0, 0.0));
    let place = Place::new(&table, &shore, 0.0, DVec3::new(95.0, 61.0, 0.0), 1.0);
    let near = beds(&place);
    assert!(heard(&near, "ambient_waves") > 0.0 && heard(&near, "ambient_wind") > 0.0);
}

#[test]
fn the_underground_is_deep_and_dark() {
    let head = DVec3::new(2.5, 31.6, 2.5);
    assert!(underground(&Rock::room((5, 3, 5), 80), head) > 0.99);
    assert_eq!(
        underground(&Rock::room((5, 3, 5), 33), head),
        0.0,
        "a house"
    );
    assert_eq!(
        underground(&Rock::open_air(), DVec3::new(0.0, 62.0, 0.0)),
        0.0
    );
}

#[test]
fn ambiences_cross_fade_without_jumps_and_events_come_now_and_then() {
    let table = table();
    let mut player = AmbiencePlayer::default();
    let mut rng = strata_core::audio::Rng::new(9);
    let mut cues = Vec::new();
    let head = DVec3::new(0.0, 125.0, 0.0);
    let plains = Place::new(&table, &[(biome("plains"), 1.0)], 0.0, head, 1.0);
    let cave = Place::new(&table, &[(biome("plains"), 1.0)], 1.0, head, 1.0);
    let dt = 0.05;
    let mut last: Vec<(SoundId, f32)> = Vec::new();
    for frame in 0..400 {
        let place = if frame < 200 { &plains } else { &cave };
        player.update(dt, &table, place, &mut rng, &mut cues);
        for &(sound, level) in player.levels() {
            let before = last.iter().find(|(s, _)| *s == sound).map_or(0.0, |l| l.1);
            assert!(
                (level - before).abs() <= dt / FADE_SECONDS + 1e-5,
                "frame {frame}: {before} → {level}"
            );
        }
        last = player.levels().to_vec();
    }
    let level = |name: &str| heard(player.levels(), name);
    assert_eq!(level("ambient_wind"), 0.0, "faded out");
    assert!((level("ambient_cave") - 0.8).abs() < 1e-4, "faded in");
    let drips = cues.iter().filter(|c| c.sound == sound_id("drip")).count();
    assert!(
        (1..=5).contains(&drips),
        "{drips} drips in 10 s underground"
    );
    assert!(cues.iter().all(|c| c.subtitled));
    // In the mixer, a bed asked louder at once still rises smoothly.
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.apply(Command::Bed(BedVolume {
        sound: sound_id("ambient_cave"),
        buffer: vec![0.5; 1000].into(),
        category: strata_core::audio::SoundCategory::Ambient,
        gain: 1.0,
    }));
    let mut out = vec![0.0; 512];
    mixer.render(&mut out);
    let steepest = out
        .chunks(2)
        .zip(out.chunks(2).skip(1))
        .map(|(a, b)| (b[0] - a[0]).abs())
        .fold(0.0, f32::max);
    assert!(steepest < 0.01 && out[510] > 0.3, "{steepest}");
}

#[test]
fn the_space_is_told_by_its_walls() {
    let open = space_of(&Rock::open_air(), DVec3::new(0.0, 62.0, 0.0));
    assert_eq!(open, Space::Outdoors);
    let head = DVec3::new(2.5, 31.6, 2.5);
    assert_eq!(space_of(&Rock::room((5, 3, 5), 80), head), Space::SmallCave);
    assert_eq!(space_of(&Rock::room((5, 3, 5), 33), head), Space::Interior);
    let cavern = Rock::room((40, 24, 40), 120);
    assert_eq!(
        space_of(&cavern, DVec3::new(20.5, 34.0, 20.5)),
        Space::LargeCavern
    );
}

#[test]
fn a_wall_muffles_and_open_air_does_not() {
    let wall = Rock {
        hollows: vec![
            (BlockPos::new(-20, 30, -5), BlockPos::new(-1, 40, 5)),
            (BlockPos::new(2, 30, -5), BlockPos::new(20, 40, 5)),
        ],
        ..Rock::room((1, 1, 1), 80)
    };
    let (ears, source) = (DVec3::new(-6.5, 32.5, 0.5), DVec3::new(6.5, 32.5, 0.5));
    let through = muffling(&wall, source, ears);
    assert!(through > 0.75, "two blocks of rock: {through}");
    let beside = muffling(&wall, DVec3::new(-12.5, 32.5, 0.5), ears);
    assert_eq!(beside, 0.0, "same side of the wall");
    let up = DVec3::Y * 30.0;
    assert_eq!(muffling(&Rock::open_air(), source + up, ears + up), 0.0);
    let clear = mixed(None, |_| {}, 0.0);
    let muffled = mixed(None, |_| {}, 1.0);
    assert!(rms(&muffled) < 0.5 * rms(&clear));
    assert!(brightness(&muffled) < 0.2 * brightness(&clear), "dull");
}

/// Two seconds of a noise burst 4 blocks away, through a mixer set up by `setup`.
fn mixed(spatial: Option<bool>, setup: impl Fn(&mut Mixer), muffle: f32) -> Vec<f32> {
    let mut mixer = Mixer::new(SAMPLE_RATE);
    if let Some(spatial) = spatial {
        mixer.apply(Command::Spatial(spatial));
    }
    setup(&mut mixer);
    let mut rng = strata_core::audio::Rng::new(4);
    let burst: Vec<f32> = (0..SAMPLE_RATE as usize / 4)
        .map(|i| rng.signed() * 0.5 * (1.0 - i as f32 / 11_025.0))
        .collect();
    mixer.apply(Command::Play(Voice {
        id: 1,
        buffer: burst.into(),
        category: strata_core::audio::SoundCategory::Blocks,
        gain: 1.0,
        pitch: 1.0,
        at: Some(DVec3::new(0.0, 0.0, -4.0)),
        range: 32.0,
        muffle,
    }));
    let mut out = vec![0.0; SAMPLE_RATE as usize * 4];
    for block in out.chunks_mut(1024) {
        mixer.render(block);
    }
    out
}

fn rms(stereo: &[f32]) -> f32 {
    (stereo.iter().map(|s| s * s).sum::<f32>() / stereo.len() as f32).sqrt()
}

/// Energy of the sample-to-sample changes relative to the signal's: high for bright sounds.
fn brightness(stereo: &[f32]) -> f32 {
    let left: Vec<f32> = stereo.iter().step_by(2).copied().collect();
    let changes: f32 = left.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
    changes / left.iter().map(|s| s * s).sum::<f32>().max(1e-12)
}

fn cavern(mixer: &mut Mixer) {
    mixer.apply(Command::Space(Space::LargeCavern));
    let mut warm = vec![0.0; 1024];
    for _ in 0..60 {
        mixer.render(&mut warm);
    }
}

#[test]
fn the_reverberation_is_bounded_and_dies_out() {
    let mut reverb = Reverb::new(SAMPLE_RATE);
    reverb.set_space(Space::LargeCavern);
    let block = 512;
    let mut out = vec![0.0; block * 2];
    let silence = vec![0.0; block];
    for _ in 0..60 {
        reverb.render(&silence, &mut out);
    }
    let mut impulse = vec![0.0; block];
    impulse[0] = 1.0;
    let mut tail = Vec::new();
    out.fill(0.0);
    reverb.render(&impulse, &mut out);
    tail.extend_from_slice(&out);
    for _ in 0..(SAMPLE_RATE as usize * 6 / block) {
        out.fill(0.0);
        reverb.render(&silence, &mut out);
        tail.extend_from_slice(&out);
    }
    assert!(tail.iter().all(|s| s.is_finite() && s.abs() < 1.0));
    let second = SAMPLE_RATE as usize * 2;
    let windows: Vec<f32> = tail.chunks(second / 4).map(rms).collect();
    assert!(windows[1] > 0.0, "it rings");
    for pair in windows[1..].windows(2) {
        assert!(pair[1] < pair[0], "the tail decays: {windows:?}");
    }
    assert!(*windows.last().unwrap() < windows[1] * 1e-3);
    // Loud noise for two seconds: no runaway.
    let mut rng = strata_core::audio::Rng::new(1);
    let mut loudest = 0.0f32;
    for _ in 0..(SAMPLE_RATE as usize * 2 / block) {
        let noise: Vec<f32> = (0..block).map(|_| rng.signed()).collect();
        out.fill(0.0);
        reverb.render(&noise, &mut out);
        loudest = out.iter().fold(loudest, |l, s| l.max(s.abs()));
    }
    assert!(loudest.is_finite() && loudest < 2.0, "{loudest}");
}

#[test]
fn simple_stereo_has_no_reverberation_nor_muffling() {
    let plain = mixed(None, |_| {}, 0.0);
    let simple = mixed(Some(false), cavern, 1.0);
    assert_eq!(
        simple, plain,
        "simple stereo ignores the place and the walls"
    );
    let advanced = mixed(Some(true), cavern, 0.0);
    let tail = SAMPLE_RATE as usize; // after the burst (a quarter second) ends
    assert!(rms(&advanced[tail..]) > 1e-3, "the cavern rings on");
    assert_eq!(rms(&plain[tail..]), 0.0);
    let mut settings = crate::settings::Settings::default();
    settings.apply_preset(crate::settings::Preset::Ds);
    assert_eq!(
        settings.sound.spatial,
        crate::settings::SpatialAudio::Simple
    );
    settings.apply_preset(crate::settings::Preset::Ultra);
    assert!(settings.sound.spatial.is_advanced());
}

/// Writes a few mixes to WAV files (in `STRATA_MIX_DIR` if set, kept there).
#[test]
fn mixes_can_be_listened_to() {
    let kept = std::env::var_os("STRATA_MIX_DIR").map(std::path::PathBuf::from);
    let dir = kept.clone().unwrap_or_else(|| {
        std::env::temp_dir().join(format!("strata-mixes-{}", std::process::id()))
    });
    std::fs::create_dir_all(&dir).unwrap();
    let mixes = [
        ("dry", mixed(None, |_| {}, 0.0)),
        ("muffled", mixed(None, |_| {}, 1.0)),
        ("cave", mixed(Some(true), small_cave, 0.0)),
        ("cavern", mixed(Some(true), cavern, 0.0)),
    ];
    for (name, stereo) in &mixes {
        let left: Vec<f32> = stereo.iter().step_by(2).copied().collect();
        let path = dir.join(format!("{name}.wav"));
        write_wav(&path, &left).unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 44);
    }
    if kept.is_none() {
        std::fs::remove_dir_all(dir).unwrap();
    }
}

fn small_cave(mixer: &mut Mixer) {
    mixer.apply(Command::Space(Space::SmallCave));
    let mut warm = vec![0.0; 1024];
    for _ in 0..60 {
        mixer.render(&mut warm);
    }
}

#[test]
fn the_game_follows_the_place_without_a_device() {
    let settings = crate::settings::SoundSettings::default();
    let mut audio = Audio::silent(content(), &settings);
    let mut world = ClientWorld::default();
    world.set_biome_tints(vec![BiomeTints::default(); BIOMES.len()].into());
    let infos: Vec<BiomeInfo> = BIOMES
        .iter()
        .map(|name| BiomeInfo {
            id: Identifier::new("strata", name).unwrap(),
            tints: BiomeTints::default(),
        })
        .collect();
    let ears = listener_at(DVec3::new(0.5, 70.5, 0.5), 0.0);
    let hearing = Hearing {
        listener: ears,
        player: None,
        world: &world,
        biomes: &infos,
        daylight: 0.0,
        place: MusicPlace::Title,
        paused: false,
    };
    for _ in 0..20 {
        audio.update(0.05, &hearing);
    }
    assert_eq!(audio.environment.space(), Space::Outdoors);
}
