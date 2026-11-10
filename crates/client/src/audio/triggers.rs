//! When sounds play, deduced by the client from what it already knows: blocks changed by
//! the server, the player's own movement, and blocks around that hum by themselves.

use crate::world::ClientWorld;
use glam::DVec3;
use strata_core::audio::{Rng, SoundSet};
use strata_core::content::{Block, BlockId, Content, RenderKind, SoundId};
use strata_core::coords::BlockPos;
use strata_core::physics::Body;

/// A sound to play.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cue {
    pub sound: SoundId,
    pub at: DVec3,
    /// One's own steps and strokes are not subtitled.
    pub subtitled: bool,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BlockSounds {
    pub set: SoundSet,
    pub liquid: bool,
    /// Gives way to what is put in its place (air, plants, fire, liquids).
    pub yields: bool,
    /// How much it muffles sound through it: 0 (air, plants) to 1 (full solid block).
    pub density: f32,
}

/// Sounds of each block, by `BlockId`.
#[derive(Debug, Clone, Default)]
pub struct SoundTable {
    blocks: Vec<BlockSounds>,
}

impl SoundTable {
    pub fn from_content(content: &Content) -> Self {
        let blocks = content
            .blocks
            .iter()
            .map(|(id, _, block)| BlockSounds {
                set: block
                    .sound
                    .as_ref()
                    .and_then(|set| content.sounds.sets.get(set))
                    .copied()
                    .unwrap_or_default(),
                liquid: block.fluid.is_some(),
                yields: id == BlockId::AIR || block.replaceable || block.fluid.is_some(),
                density: density(block),
            })
            .collect();
        Self { blocks }
    }

    pub fn get(&self, block: BlockId) -> BlockSounds {
        self.blocks
            .get(usize::from(block.0))
            .copied()
            .unwrap_or_default()
    }

    /// Breaking `old` or placing `new`, like the server decided.
    pub fn changed(&self, pos: BlockPos, old: BlockId, new: BlockId) -> Option<Cue> {
        if old == new {
            return None;
        }
        let (before, after) = (self.get(old), self.get(new));
        let sound = if after.yields && !before.liquid {
            before.set.breaking
        } else if before.yields && !after.liquid {
            after.set.place
        } else {
            None
        }?;
        Some(Cue {
            sound,
            at: centre(pos),
            subtitled: true,
        })
    }
}

fn density(block: &Block) -> f32 {
    match block.render {
        _ if block.fluid.is_some() => 0.4,
        RenderKind::Opaque if block.solid => 1.0,
        RenderKind::Opaque | RenderKind::Translucent => 0.6,
        RenderKind::Cutout => 0.2,
        RenderKind::Cross | RenderKind::Invisible => 0.0,
    }
}

pub(super) fn centre(pos: BlockPos) -> DVec3 {
    DVec3::new(pos.x as f64, pos.y as f64, pos.z as f64) + 0.5
}

pub(super) fn cell(position: DVec3) -> BlockPos {
    BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    )
}

/// Ground distance between two steps (a stride), and swimming distance per stroke.
const STRIDE: f64 = 1.6;
const STROKE: f64 = 2.2;
/// Falling faster than this (blocks per second) into a liquid splashes.
const SPLASH_SPEED: f64 = 2.0;
/// A jump longer than this in one frame is a teleport, not a walk.
const TELEPORT: f64 = 4.0;

/// The local player's steps, strokes and splashes.
#[derive(Debug, Default)]
pub struct Footsteps {
    last: Option<DVec3>,
    travelled: f64,
    in_liquid: bool,
}

impl Footsteps {
    pub fn update(
        &mut self,
        body: &Body,
        world: &ClientWorld,
        table: &SoundTable,
        cues: &mut Vec<Cue>,
    ) {
        let position = body.position;
        let moved = self.last.map_or(DVec3::ZERO, |last| position - last);
        self.last = Some(position);
        let feet = table.get(world.block_at(cell(position + DVec3::Y * 0.1)));
        let entering = feet.liquid && !self.in_liquid;
        self.in_liquid = feet.liquid;
        let cue = |sound: Option<SoundId>, subtitled| {
            sound.map(|sound| Cue {
                sound,
                at: position,
                subtitled,
            })
        };
        if entering && body.velocity.y < -SPLASH_SPEED {
            cues.extend(cue(feet.set.enter, true));
        }
        if moved.length() > TELEPORT || body.flying {
            self.travelled = 0.0;
            return;
        }
        let (distance, period, sound) = if feet.liquid && !body.on_ground {
            (moved.length(), STROKE, feet.set.swim)
        } else if body.on_ground {
            let ground = if feet.liquid {
                feet
            } else {
                table.get(world.block_at(cell(position - DVec3::Y * 0.05)))
            };
            (moved.with_y(0.0).length(), STRIDE, ground.set.step)
        } else {
            return;
        };
        self.travelled += distance;
        if self.travelled >= period {
            self.travelled %= period;
            cues.extend(cue(sound, false));
        }
    }
}

/// Blocks around the listener tried per second for their ambient sound, how far, and
/// how many may start per second.
const AMBIENT_SAMPLES: f32 = 300.0;
const AMBIENT_REACH: (i32, i32) = (12, 8);
const MAX_AMBIENT_PER_SECOND: f32 = 4.0;

/// Lava bubbling, fire crackling: blocks picked at random around the listener.
#[derive(Debug, Default)]
pub struct AmbientBlocks {
    samples_due: f32,
    allowance: f32,
}

impl AmbientBlocks {
    pub fn update(
        &mut self,
        seconds: f32,
        listener: DVec3,
        sources: (&ClientWorld, &SoundTable),
        rng: &mut Rng,
        cues: &mut Vec<Cue>,
    ) {
        let (world, table) = sources;
        self.samples_due += seconds * AMBIENT_SAMPLES;
        self.allowance =
            (self.allowance + seconds * MAX_AMBIENT_PER_SECOND).min(MAX_AMBIENT_PER_SECOND);
        let here = cell(listener);
        let (h, v) = AMBIENT_REACH;
        while self.samples_due >= 1.0 {
            self.samples_due -= 1.0;
            let offset = |reach: i32, rng: &mut Rng| (rng.signed() * reach as f32).round() as i32;
            let pos = BlockPos::new(
                here.x + offset(h, rng),
                here.y + offset(v, rng),
                here.z + offset(h, rng),
            );
            let set = table.get(world.block_at(pos)).set;
            let Some(sound) = set.ambient else {
                continue;
            };
            if self.allowance >= 1.0 && rng.unit() < set.ambient_chance {
                self.allowance -= 1.0;
                cues.push(Cue {
                    sound,
                    at: centre(pos),
                    subtitled: true,
                });
            }
        }
    }
}
