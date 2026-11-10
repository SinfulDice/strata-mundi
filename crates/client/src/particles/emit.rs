//! Where particles come from: breaking blocks, emitters sampled around the camera, and
//! the splashes of those that land.

use super::pool::{LiveParticle, Look, Rng};
use super::table::ParticleBlock;
use super::{ParticleSystem, Spawner};
use crate::world::ClientWorld;
use glam::{DVec3, Vec3};
use strata_core::content::{BlockId, Face, ParticleId, TintFaces};
use strata_core::coords::BlockPos;

/// Blocks around the camera where drips and bubbles appear (horizontally, vertically).
const REACH: (i32, i32) = (12, 8);
/// Bounds the work of a slow frame.
const MAX_SAMPLES_PER_FRAME: u32 = 96;
/// Edge of a piece of block texture, in texture coordinates (4 × 4 of 16 pixels).
const PIECE: f32 = 0.25;

impl ParticleSystem {
    /// A block changed (server decision): a broken one throws its fragments.
    pub fn block_changed(
        &mut self,
        pos: BlockPos,
        old: BlockId,
        new: BlockId,
        world: &ClientWorld,
    ) {
        if self.budget.capacity == 0 {
            return;
        }
        let Some(kind) = self.table.breaking(old, new) else {
            return;
        };
        let Some(def) = self.table.particle(kind) else {
            return;
        };
        let count = def.burst.div_ceil(self.budget.burst_divisor);
        let block = self.table.block(old);
        let mut spawner = self.spawner();
        for _ in 0..count {
            let face = Face::ALL[(spawner.rng.next_u64() % 6) as usize];
            let source = block_look(&block, face, pos, world);
            let inside = DVec3::new(
                f64::from(spawner.rng.between([0.15, 0.85])),
                f64::from(spawner.rng.between([0.1, 0.8])),
                f64::from(spawner.rng.between([0.15, 0.85])),
            );
            spawner.spawn(kind, block_corner(pos) + inside, source);
        }
    }

    /// Looks at a few random cells around `camera` (a budget per second, carried over
    /// between frames) for drips and bubbles.
    pub(super) fn sample_emitters(&mut self, dt: f32, camera: DVec3, world: &ClientWorld) {
        self.samples_due += dt * self.budget.samples_per_second;
        let samples = (self.samples_due as u32).min(MAX_SAMPLES_PER_FRAME);
        self.samples_due = (self.samples_due - samples as f32).min(1.0);
        let centre = super::physics::cell_of(camera);
        let mut spawner = self.spawner();
        for _ in 0..samples {
            let cell = BlockPos {
                x: centre.x + spawner.rng.offset(REACH.0),
                y: centre.y + spawner.rng.offset(REACH.1),
                z: centre.z + spawner.rng.offset(REACH.0),
            };
            spawner.sample(cell, world);
        }
    }

    pub(super) fn spawner(&mut self) -> Spawner<'_> {
        Spawner {
            table: &self.table,
            pool: &mut self.pool,
            rng: &mut self.rng,
        }
    }
}

impl Spawner<'_> {
    /// `source`: the look of the block it comes from, for particles without a texture.
    fn spawn(&mut self, kind: ParticleId, position: DVec3, source: Look) -> bool {
        let Some(def) = self.table.particle(kind) else {
            return false;
        };
        if self.pool.is_full() {
            return false;
        }
        let look = match def.texture {
            Some(texture) => Look {
                layer: u32::from(texture.0),
                uv: [0.0, 0.0, 1.0],
                color: linear(def.color),
            },
            None => Look {
                uv: random_piece(self.rng),
                color: (Vec3::from(source.color) * Vec3::from(linear(def.color))).to_array(),
                ..source
            },
        };
        let speed = def.speed * self.rng.between([0.5, 1.0]);
        self.pool.push(LiveParticle {
            kind,
            position,
            velocity: self.rng.direction() * speed,
            age: 0.0,
            lifetime: self.rng.between(def.lifetime),
            size: self.rng.between(def.size),
            look,
            on_ground: false,
        })
    }

    /// The impact particles of one that just ended, with its look.
    pub(super) fn splash(&mut self, ended: &LiveParticle) {
        let Some((kind, count)) = self.table.particle(ended.kind).and_then(|def| def.impact) else {
            return;
        };
        let at = ended.position + DVec3::Y * 0.02;
        for _ in 0..count {
            if !self.spawn(kind, at, ended.look) {
                return;
            }
        }
    }

    /// Drips under a full block with a seeping block above it, bubbles in a liquid
    /// cell over a full block.
    fn sample(&mut self, cell: BlockPos, world: &ClientWorld) {
        let here = self.table.block(world.block_at(cell));
        if let Some(kind) = here.particles.ambient {
            let below = self.table.block(world.block_at(cell.neighbour(Face::Down)));
            if below.solid {
                self.emit_chance(kind, cell, &here, 0.05, world);
            }
        }
        if !here.solid || here.liquid {
            return;
        }
        let under = cell.neighbour(Face::Down);
        let open = self.table.block(world.block_at(under));
        if open.solid || open.liquid {
            return;
        }
        let above = self.table.block(world.block_at(cell.neighbour(Face::Up)));
        if let Some(kind) = above.particles.drip {
            self.emit_chance(kind, under, &above, 0.96, world);
        }
    }

    /// One particle at height `y` (fraction of the block) of `cell`, with its chance.
    fn emit_chance(
        &mut self,
        kind: ParticleId,
        cell: BlockPos,
        source: &ParticleBlock,
        y: f64,
        world: &ClientWorld,
    ) {
        let chance = self.table.particle(kind).map_or(0.0, |def| def.chance);
        if self.rng.unit() >= chance {
            return;
        }
        let look = block_look(source, Face::Down, cell, world);
        let offset = DVec3::new(
            f64::from(self.rng.between([0.15, 0.85])),
            y,
            f64::from(self.rng.between([0.15, 0.85])),
        );
        self.spawn(kind, block_corner(cell) + offset, look);
    }
}

fn block_corner(pos: BlockPos) -> DVec3 {
    DVec3::new(f64::from(pos.x), f64::from(pos.y), f64::from(pos.z))
}

/// The texture of `face` and, for tinted blocks, the colour of the biome at `pos`.
fn block_look(block: &ParticleBlock, face: Face, pos: BlockPos, world: &ClientWorld) -> Look {
    let tinted = block
        .tint
        .filter(|tint| tint.faces == TintFaces::All || face == Face::Up);
    let color = tinted
        .and_then(|tint| {
            let biome = world.biome_at(pos.x, pos.z)?;
            let tints = world.biome_tints().get(usize::from(biome.0))?;
            Some(
                Vec3::from(tints.get(tint.kind).factors())
                    .powf(2.2)
                    .to_array(),
            )
        })
        .unwrap_or([1.0; 3]);
    Look {
        layer: u32::from(block.texture(face).0),
        uv: [0.0, 0.0, PIECE],
        color,
    }
}

fn random_piece(rng: &mut Rng) -> [f32; 3] {
    let corner = 1.0 - PIECE;
    [rng.unit() * corner, rng.unit() * corner, PIECE]
}

/// sRGB colour to a linear factor.
fn linear([r, g, b, _]: [u8; 4]) -> [f32; 3] {
    [r, g, b].map(|c| (f32::from(c) / 255.0).powf(2.2))
}
