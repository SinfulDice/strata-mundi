//! What particles need to know about each block and each particle, from the content.

use strata_core::content::{
    BlockId, BlockParticles, BlockTint, Content, Face, Particle, ParticleId, TextureId,
};

#[derive(Debug, Clone, Copy)]
pub struct ParticleBlock {
    /// Stops particles (and the player).
    pub solid: bool,
    pub liquid: bool,
    /// Taken by a placed block like air (plants).
    pub replaceable: bool,
    pub particles: BlockParticles,
    pub faces: [TextureId; 6],
    pub tint: Option<BlockTint>,
}

impl Default for ParticleBlock {
    fn default() -> Self {
        Self {
            solid: false,
            liquid: false,
            replaceable: false,
            particles: BlockParticles::default(),
            faces: [TextureId::MISSING; 6],
            tint: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParticleTable {
    blocks: Vec<ParticleBlock>,
    particles: Vec<Particle>,
}

impl ParticleTable {
    pub fn from_content(content: &Content) -> Self {
        let blocks = content
            .blocks
            .iter()
            .map(|(_, _, block)| ParticleBlock {
                solid: block.solid,
                liquid: block.fluid.is_some(),
                replaceable: block.replaceable,
                particles: block.particles,
                faces: block.faces,
                tint: block.tint,
            })
            .collect();
        let particles = content
            .particles
            .iter()
            .map(|(_, _, particle)| particle.clone())
            .collect();
        Self { blocks, particles }
    }

    #[cfg(test)]
    pub fn new(blocks: Vec<ParticleBlock>, particles: Vec<Particle>) -> Self {
        Self { blocks, particles }
    }

    /// Air for unknown ids.
    pub fn block(&self, id: BlockId) -> ParticleBlock {
        self.blocks
            .get(usize::from(id.0))
            .copied()
            .unwrap_or_default()
    }

    pub fn particle(&self, id: ParticleId) -> Option<&Particle> {
        self.particles.get(usize::from(id.0))
    }

    /// The particles `old` throws when it becomes `new`: broken by the player, burnt, or
    /// washed away (a liquid taking its place); never for liquids themselves.
    pub fn breaking(&self, old: BlockId, new: BlockId) -> Option<ParticleId> {
        let (before, after) = (self.block(old), self.block(new));
        let freed = new == BlockId::AIR || after.replaceable || after.liquid;
        (old != new && freed && !before.liquid)
            .then_some(before.particles.breaking)
            .flatten()
    }
}

impl ParticleBlock {
    pub fn texture(&self, face: Face) -> TextureId {
        self.faces[face.index()]
    }
}
