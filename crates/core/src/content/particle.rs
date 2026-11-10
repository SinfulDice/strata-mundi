//! Particles (block fragments, drips, splashes, bubbles) and the blocks that emit them.
//! Only the client draws them: they never touch the world.

use super::registry::{ParticleId, Registry};
use super::textures::TextureLibrary;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use crate::texture::HexColor;
use serde::Deserialize;
use std::path::Path;

/// A particle as written in `particles/<name>.ron`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Particle", deny_unknown_fields)]
pub struct ParticleDef {
    /// Drawn whole; absent, a random piece of the texture of the block that emits it.
    #[serde(default)]
    pub texture: Option<Identifier>,
    /// Multiplies the texture (after the biome colour of tinted blocks).
    #[serde(default = "white")]
    pub color: HexColor,
    /// Seconds, picked between the two.
    pub lifetime: (f32, f32),
    /// Edge of the square, in blocks, picked between the two.
    pub size: (f32, f32),
    /// Blocks per second², downwards; negative rises (bubbles).
    #[serde(default)]
    pub gravity: f32,
    /// Starting speed in a random direction, in blocks per second.
    #[serde(default)]
    pub speed: f32,
    /// Fraction of the speed kept after a second.
    #[serde(default = "one")]
    pub drag: f32,
    /// Seconds held still before moving (a drop swelling under a ceiling).
    #[serde(default)]
    pub hang: f32,
    /// Stops on full blocks instead of going through them.
    #[serde(default = "yes")]
    pub collides: bool,
    /// Shines by itself (lava): as bright in the dark.
    #[serde(default)]
    pub glows: bool,
    /// Lives only inside liquids (bubbles).
    #[serde(default)]
    pub in_liquid: bool,
    /// How many are thrown at once by a breaking block (with every particle on).
    #[serde(default = "one_burst")]
    pub burst: u16,
    /// Chance that a sampled emitter (drip, ambient) gives one.
    #[serde(default = "one")]
    pub chance: f32,
    /// Particles thrown where it lands or falls into a liquid; it ends there.
    #[serde(default)]
    pub impact: Option<ImpactDef>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ImpactDef {
    pub particle: Identifier,
    pub count: u16,
}

/// A loaded particle.
#[derive(Debug, Clone, PartialEq)]
pub struct Particle {
    /// `None`: a piece of the emitting block's texture.
    pub texture: Option<super::TextureId>,
    /// sRGB with alpha.
    pub color: [u8; 4],
    pub lifetime: [f32; 2],
    pub size: [f32; 2],
    pub gravity: f32,
    pub speed: f32,
    pub drag: f32,
    pub hang: f32,
    pub collides: bool,
    pub glows: bool,
    pub in_liquid: bool,
    pub burst: u16,
    pub chance: f32,
    pub impact: Option<(ParticleId, u16)>,
}

/// `particles` field of a block: what it gives off.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ParticleTriggersDef {
    /// Thrown when the block is broken (fragments of its texture by default).
    #[serde(default = "default_breaking")]
    pub breaking: Option<Identifier>,
    /// Falls from under the full block beneath it (a liquid seeping through a ceiling).
    #[serde(default)]
    pub drip: Option<Identifier>,
    /// Appears inside the block above a full block (bubbles in water).
    #[serde(default)]
    pub ambient: Option<Identifier>,
}

impl Default for ParticleTriggersDef {
    fn default() -> Self {
        Self {
            breaking: default_breaking(),
            drip: None,
            ambient: None,
        }
    }
}

/// Particles of a loaded block.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct BlockParticles {
    pub breaking: Option<ParticleId>,
    pub drip: Option<ParticleId>,
    pub ambient: Option<ParticleId>,
}

/// Fragments every block throws unless it says otherwise; a pack without this particle
/// simply has none.
pub fn default_breaking() -> Option<Identifier> {
    Some(Identifier::builtin("strata", "block_fragments"))
}

fn white() -> HexColor {
    HexColor(crate::texture::Rgba([255; 4]))
}

fn one() -> f32 {
    1.0
}

fn one_burst() -> u16 {
    1
}

fn yes() -> bool {
    true
}

/// Most particles a breaking block may ask for.
pub const MAX_BURST: u16 = 256;

pub(super) fn load_particles(
    packs: &PackSet,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> Registry<ParticleId, Particle> {
    let defs = packs.layered::<ParticleDef>("particles", errors);
    let mut particles = Registry::default();
    for (id, Sourced { value, source }) in &defs {
        particles.insert(id.clone(), resolve(value, textures, source, errors));
    }
    for (id, Sourced { value, source }) in &defs {
        let Some(impact) = &value.impact else {
            continue;
        };
        let target = particles.key(&impact.particle);
        if target.is_none() {
            errors.push(unknown(source, &impact.particle));
        }
        if let Some(particle) = particles.key(id).and_then(|key| particles.get_mut(key)) {
            particle.impact = target.map(|target| (target, impact.count.min(MAX_BURST)));
        }
    }
    particles
}

fn resolve(
    def: &ParticleDef,
    textures: &TextureLibrary,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Particle {
    let texture = def.texture.as_ref().map(|id| {
        textures.layer(id).unwrap_or_else(|| {
            errors.push(PackError::invalid(
                source,
                format!("unknown texture `{id}`"),
            ));
            super::TextureId::MISSING
        })
    });
    let range = |(a, b): (f32, f32), min: f32| {
        let (a, b) = (a.max(min), b.max(min));
        [a.min(b), a.max(b)]
    };
    Particle {
        texture,
        color: def.color.0.0,
        lifetime: range(def.lifetime, 0.05),
        size: range(def.size, 0.01),
        gravity: def.gravity,
        speed: def.speed.max(0.0),
        drag: def.drag.clamp(0.0, 1.0),
        hang: def.hang.max(0.0),
        collides: def.collides,
        glows: def.glows,
        in_liquid: def.in_liquid,
        burst: def.burst.min(MAX_BURST),
        chance: def.chance.clamp(0.0, 1.0),
        impact: None,
    }
}

/// The particles a block names; unknown ones are reported, except the default fragments.
pub(super) fn block_particles(
    def: &ParticleTriggersDef,
    particles: &Registry<ParticleId, Particle>,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> BlockParticles {
    let mut find = |id: &Option<Identifier>| {
        let id = id.as_ref()?;
        let key = particles.key(id);
        if key.is_none() && Some(id) != default_breaking().as_ref() {
            errors.push(unknown(source, id));
        }
        key
    };
    BlockParticles {
        breaking: find(&def.breaking),
        drip: find(&def.drip),
        ambient: find(&def.ambient),
    }
}

fn unknown(source: &Path, id: &Identifier) -> PackError {
    PackError::invalid(source, format!("unknown particle `{id}`"))
}
