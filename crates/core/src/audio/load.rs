//! Loading sounds (`sounds/<name>.ron` recipes, replaced by `sounds/<name>.ogg` when a
//! pack gives one) and sound sets (`sound_sets/<name>.ron`, what a block sounds like).

use super::recipe::{SoundDef, bound};
use crate::content::{Registry, SoundId};
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced, category_files};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct Sound {
    pub def: SoundDef,
    /// A recorded file that replaces the recipe (decoded by the client).
    pub ogg: Option<PathBuf>,
    /// Base of the variants' seeds, from the identifier: the same on every run.
    pub seed: u64,
}

/// What a block sounds like, as written in `sound_sets/<name>.ron`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename = "SoundSet", deny_unknown_fields)]
pub struct SoundSetDef {
    /// Walking on it.
    #[serde(default)]
    pub step: Option<Identifier>,
    #[serde(default)]
    pub breaking: Option<Identifier>,
    #[serde(default)]
    pub place: Option<Identifier>,
    /// Falling into it (liquids).
    #[serde(default)]
    pub enter: Option<Identifier>,
    /// Swimming strokes in it (liquids).
    #[serde(default)]
    pub swim: Option<Identifier>,
    /// Given off now and then by the block itself (lava bubbling, fire crackling).
    #[serde(default)]
    pub ambient: Option<Identifier>,
    /// Chance that the block gives its ambient sound when picked among the blocks around
    /// the player (a few hundred are tried each second).
    #[serde(default = "ambient_chance")]
    pub ambient_chance: f32,
}

fn ambient_chance() -> f32 {
    0.05
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SoundSet {
    pub step: Option<SoundId>,
    pub breaking: Option<SoundId>,
    pub place: Option<SoundId>,
    pub enter: Option<SoundId>,
    pub swim: Option<SoundId>,
    pub ambient: Option<SoundId>,
    pub ambient_chance: f32,
}

#[derive(Debug, Clone, Default)]
pub struct Sounds {
    pub registry: Registry<SoundId, Sound>,
    pub sets: BTreeMap<Identifier, SoundSet>,
}

impl Sounds {
    pub fn get(&self, id: SoundId) -> Option<&Sound> {
        self.registry.get(id)
    }

    /// Fluent keys of every subtitle.
    pub fn subtitle_keys(&self) -> impl Iterator<Item = &String> {
        self.registry
            .iter()
            .filter_map(|(_, _, sound)| sound.def.subtitle.as_ref())
    }
}

/// `packs` is the joined set (content and resource packs): sounds are presentation.
pub fn load_sounds(packs: &PackSet, errors: &mut Vec<PackError>) -> Sounds {
    let defs = packs.layered::<SoundDef>("sounds", errors);
    let oggs = recordings(packs, errors);
    let mut registry = Registry::default();
    for (id, Sourced { mut value, source }) in defs {
        let mut problems = Vec::new();
        bound(&mut value, &mut problems);
        for problem in problems {
            errors.push(PackError::invalid(&source, problem));
        }
        let ogg = oggs
            .get(&id)
            .filter(|(layer, _)| *layer >= layer_of(packs, &source))
            .map(|(_, path)| path.clone());
        let seed = seed_of(&id);
        registry.insert(
            id,
            Sound {
                def: value,
                ogg,
                seed,
            },
        );
    }
    for (id, (_, path)) in &oggs {
        if registry.key(id).is_none() {
            let message = format!("no `sounds/{}.ron` defines `{id}`", id.name());
            errors.push(PackError::invalid(path, message));
        }
    }
    let sets = packs
        .layered::<SoundSetDef>("sound_sets", errors)
        .into_iter()
        .map(|(id, Sourced { value, source })| {
            let set = resolve_set(&value, &registry, &source, errors);
            (id, set)
        })
        .collect();
    Sounds { registry, sets }
}

/// The `.ogg` of each sound in the highest pack that has one, with that pack's rank.
fn recordings(
    packs: &PackSet,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, (usize, PathBuf)> {
    let mut found = BTreeMap::new();
    for (layer, pack) in packs.packs().iter().enumerate() {
        for (id, path) in category_files(pack, "sounds", "ogg", errors) {
            found.insert(id, (layer, path));
        }
    }
    found
}

fn layer_of(packs: &PackSet, source: &Path) -> usize {
    packs
        .packs()
        .iter()
        .rposition(|pack| source.starts_with(&pack.root))
        .unwrap_or(0)
}

fn resolve_set(
    def: &SoundSetDef,
    registry: &Registry<SoundId, Sound>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> SoundSet {
    let mut find = |id: &Option<Identifier>| {
        let id = id.as_ref()?;
        let key = registry.key(id);
        if key.is_none() {
            errors.push(PackError::invalid(source, format!("unknown sound `{id}`")));
        }
        key
    };
    SoundSet {
        step: find(&def.step),
        breaking: find(&def.breaking),
        place: find(&def.place),
        enter: find(&def.enter),
        swim: find(&def.swim),
        ambient: find(&def.ambient),
        ambient_chance: def.ambient_chance.clamp(0.0, 1.0),
    }
}

/// FNV-1a of the identifier.
fn seed_of(id: &Identifier) -> u64 {
    id.to_string()
        .bytes()
        .fold(0xCBF2_9CE4_8422_2325, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01B3)
        })
}

/// Seed of variant `index` of a sound.
pub fn variant_seed(sound: &Sound, index: u8) -> u64 {
    sound.seed ^ (u64::from(index) + 1).wrapping_mul(0x9E37_79B9_7F4A_7C15)
}
