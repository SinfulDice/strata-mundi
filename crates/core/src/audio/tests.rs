use super::*;
use crate::pack::PackSet;
use std::path::{Path, PathBuf};

fn pack_sounds() -> Sounds {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
    let mut errors = Vec::new();
    let packs = PackSet::discover(&dir, &mut errors);
    let sounds = load_sounds(&packs, &mut errors);
    assert!(errors.is_empty(), "{errors:#?}");
    sounds
}

fn sound(sounds: &Sounds, name: &str) -> Sound {
    let id = crate::id::Identifier::new("strata", name).unwrap();
    sounds.registry.by_identifier(&id).cloned().unwrap()
}

fn peak(buffer: &[f32]) -> f32 {
    buffer.iter().fold(0.0, |p, s| p.max(s.abs()))
}

fn energy(buffer: &[f32]) -> f32 {
    buffer.iter().map(|s| s * s).sum::<f32>() / buffer.len().max(1) as f32
}

fn temp_dir(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("strata-audio-{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn synthesis_is_deterministic_for_a_seed() {
    let sounds = pack_sounds();
    let step = sound(&sounds, "step_gravel");
    assert_eq!(render(&step.def, 7), render(&step.def, 7));
}

#[test]
fn variants_differ() {
    let sounds = pack_sounds();
    let step = sound(&sounds, "step_stone");
    let a = render(&step.def, variant_seed(&step, 0));
    let b = render(&step.def, variant_seed(&step, 1));
    assert_ne!(a, b);
}

#[test]
fn every_pack_sound_is_bounded() {
    let sounds = pack_sounds();
    assert!(sounds.registry.len() >= 38);
    for (_, id, sound) in sounds.registry.iter() {
        assert!(
            sound.def.subtitle.is_some() != sound.def.looped,
            "{id}: a subtitle, unless it is a looping bed"
        );
        for index in 0..sound.def.variants {
            let buffer = render(&sound.def, variant_seed(sound, index));
            let longest = (sound.def.duration * SAMPLE_RATE as f32) as usize;
            assert!(!buffer.is_empty() && buffer.len() <= longest.max(1), "{id}");
            assert!(buffer.iter().all(|s| s.is_finite()), "{id}");
            let peak = peak(&buffer);
            assert!(
                peak <= sound.def.volume + 1e-6 && peak <= 1.0,
                "{id}: {peak}"
            );
            assert!(peak > 0.5 * sound.def.volume, "{id} is audible");
        }
    }
}

#[test]
fn every_block_material_has_its_sounds() {
    let sounds = pack_sounds();
    let materials = [
        "stone", "dirt", "grass", "sand", "gravel", "snow", "wood", "glass", "leaves", "metal",
        "wool",
    ];
    for name in materials {
        let set = sounds.sets[&crate::id::Identifier::new("strata", name).unwrap()];
        assert!(set.step.is_some() && set.breaking.is_some() && set.place.is_some());
    }
    let water = sounds.sets[&crate::id::Identifier::builtin("strata", "water")];
    assert!(water.enter.is_some() && water.swim.is_some());
}

#[test]
fn extreme_recipes_are_held_in_bounds() {
    let text = r#"Sound(
        category: Blocks,
        volume: 3.0,
        variants: 200,
        duration: 600.0,
        pitch: (0.0, 9.0),
        layers: [
            Noise(color: White, filter: BandPass((hz: (5.0, 90000.0), q: 0.0)),
                  envelope: (attack: -1.0, decay: (0.0, 99.0)), gain: 50.0),
            Grains(count: (10, 60000), spread: 1.0, length: (0.0, 5.0), color: Brown),
            Bubbles(count: (1, 4), spread: 0.1, hz: (1.0, 1e9), decay: (0.01, 0.1)),
        ],
    )"#;
    let mut def: SoundDef = ron::from_str(text).unwrap();
    let mut problems = Vec::new();
    recipe::bound(&mut def, &mut problems);
    assert!(problems.len() >= 8, "{problems:#?}");
    assert_eq!(
        (def.duration, def.volume, def.variants),
        (MAX_SECONDS, 1.0, MAX_VARIANTS)
    );
    assert!(matches!(
        def.layers[1],
        Layer::Grains {
            count: (10, MAX_GRAINS),
            ..
        }
    ));
    let buffer = render(&def, 1);
    assert!(buffer.len() <= (MAX_SECONDS * SAMPLE_RATE as f32) as usize);
    assert!(buffer.iter().all(|s| s.is_finite() && s.abs() <= 1.0));
}

/// Writes a few sounds to WAV files and reads them back: header, length, peak and
/// shape (struck sounds decay, the wind swells and fades).
#[test]
fn rendered_sounds_make_valid_wav_files() {
    let sounds = pack_sounds();
    let dir = temp_dir("wav");
    for name in ["step_stone", "break_glass", "water_splash", "wind"] {
        let sound = sound(&sounds, name);
        let buffer = render(&sound.def, variant_seed(&sound, 0));
        let path = dir.join(format!("{name}.wav"));
        write_wav(&path, &buffer).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(
            (&bytes[0..4], &bytes[8..16]),
            (&b"RIFF"[..], &b"WAVEfmt "[..])
        );
        let rate = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
        let data = u32::from_le_bytes(bytes[40..44].try_into().unwrap()) as usize;
        assert_eq!(
            (rate, data, bytes.len()),
            (SAMPLE_RATE, buffer.len() * 2, 44 + data)
        );
        let samples: Vec<f32> = bytes[44..]
            .chunks(2)
            .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32767.0)
            .collect();
        let seconds = samples.len() as f32 / SAMPLE_RATE as f32;
        assert!(
            seconds > 0.02 && seconds <= sound.def.duration,
            "{name}: {seconds} s"
        );
        assert!((peak(&samples) - sound.def.volume).abs() < 0.01, "{name}");
        let tenth = samples.len() / 10;
        let (head, middle, tail) = (
            energy(&samples[..tenth]),
            energy(&samples[4 * tenth..6 * tenth]),
            energy(&samples[9 * tenth..]),
        );
        if name == "wind" {
            assert!(middle > head && middle > tail, "{name} swells");
        } else {
            assert!(head > tail * 4.0, "{name} decays");
        }
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn a_recording_replaces_its_recipe() {
    let root = temp_dir("ogg");
    let write = |path: &str, text: &[u8]| {
        let path = root.join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    };
    write("base/pack.ron", b"(namespace: \"base\", priority: 0)");
    write(
        "base/sounds/knock.ron",
        b"Sound(category: Blocks, duration: 0.1, layers: [])",
    );
    write("look/pack.ron", b"(namespace: \"look\", priority: 1)");
    write("look/sounds/base/knock.ogg", b"OggS");
    write("look/sounds/lonely.ogg", b"OggS");
    let mut errors = Vec::new();
    let names = ["base".to_owned(), "look".to_owned()];
    let packs = PackSet::named(&root, &names, &mut errors);
    let sounds = load_sounds(&packs, &mut errors);
    let knock = sounds
        .registry
        .by_identifier(&crate::id::Identifier::builtin("base", "knock"))
        .unwrap();
    assert!(
        knock
            .ogg
            .as_ref()
            .unwrap()
            .ends_with("look/sounds/base/knock.ogg")
    );
    let messages: Vec<String> = errors.iter().map(ToString::to_string).collect();
    assert!(
        messages.iter().any(|m| m.contains("lonely")),
        "{messages:#?}"
    );
    assert!(
        messages.iter().any(|m| m.contains("one layer")),
        "{messages:#?}"
    );
    std::fs::remove_dir_all(root).unwrap();
}

/// A looped bed joins its end to its start without a jump, and keeps its loudness.
#[test]
fn looped_beds_are_seamless() {
    let sounds = pack_sounds();
    for name in ["ambient_wind", "ambient_waves", "ambient_cave"] {
        let sound = sound(&sounds, name);
        assert!(sound.def.looped);
        let buffer = render(&sound.def, variant_seed(&sound, 0));
        let seam = (buffer[buffer.len() - 1] - buffer[0]).abs();
        let typical = buffer
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .fold(0.0f32, f32::max);
        assert!(
            seam <= typical,
            "{name}: seam {seam}, steps up to {typical}"
        );
        let quarter = buffer.len() / 4;
        let (start, middle) = (
            energy(&buffer[..quarter / 4]),
            energy(&buffer[2 * quarter..2 * quarter + quarter / 4]),
        );
        assert!(
            start > middle * 0.2,
            "{name}: no dip where the loop is blended"
        );
    }
    let ramp: Vec<f32> = (0..400).map(|i| i as f32).collect();
    let looped = seamless(&ramp);
    assert_eq!(looped.len(), 300);
    assert!((looped[299] - 299.0).abs() < 1e-3);
}

#[test]
fn ambiences_name_their_sounds_and_places() {
    let sounds = pack_sounds();
    let underground: Vec<_> = sounds.ambiences.iter().filter(|a| a.underground).collect();
    assert_eq!(underground.len(), 1);
    assert!(underground[0].biomes.is_empty() && !underground[0].events.is_empty());
    let plains = crate::id::Identifier::new("strata", "plains").unwrap();
    let heard: Vec<_> = sounds
        .ambiences
        .iter()
        .filter(|a| a.biomes.contains(&plains))
        .collect();
    assert_eq!(heard.len(), 1, "a biome has one ambience");
    let bed = heard[0].beds[0];
    let altitude = bed.altitude.unwrap();
    assert!(
        altitude.factor(300.0) > altitude.factor(100.0),
        "windier up high"
    );
    assert_eq!(TimeOfDay::Night.factor(1.0), 0.0);
    assert_eq!(TimeOfDay::Day.factor(1.0), 1.0);
    for ambience in &sounds.ambiences {
        for event in &ambience.events {
            let def = &sounds.get(event.sound).unwrap().def;
            assert!(def.subtitle.is_some() && !def.looped, "{}", ambience.id);
        }
        for bed in &ambience.beds {
            assert!(sounds.get(bed.sound).unwrap().def.looped, "{}", ambience.id);
        }
    }
}
