use super::greedy::texture_turned;
use super::variants::TextureVariants;
use strata_core::content::{Block, BlockId, Content, Face, RenderKind, surface_sixteenths};
use strata_core::light::{PackedLight, SectionLight, section_index};

pub const SIZE: usize = 16;
const PADDED: usize = SIZE + 2;

/// Copy of a section plus a one-block border taken from its neighbours, so meshing a
/// section never needs to look anything up elsewhere (and can run on any thread).
#[derive(Debug, Clone)]
pub struct Neighborhood {
    blocks: Vec<BlockId>,
    light: Light,
    /// World position of the section's (0, 0, 0) block.
    origin: [i32; 3],
    contents: Contents,
}

/// Kinds of block a section may hold that need their own meshing pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Contents {
    pub liquid: bool,
    pub cross: bool,
}

/// The 3×3×3 sections around a section, by `[dy][dz][dx]` (each 0 to 2).
pub type LightSections = Box<[[[SectionLight; 3]; 3]; 3]>;

#[derive(Debug, Clone)]
enum Light {
    /// Open sky everywhere.
    None,
    /// As taken from the world (cheap shared copies), until a meshing thread resolves it.
    Sections(LightSections),
    /// Same layout as `blocks`.
    Cells(Vec<PackedLight>),
}

impl Contents {
    /// Nothing known: every pass runs.
    const ANY: Self = Self {
        liquid: true,
        cross: true,
    };
}

impl Neighborhood {
    /// `block(x, y, z)` with coordinates from -1 to 16 relative to the section.
    pub fn build(block: impl Fn(i32, i32, i32) -> BlockId) -> Self {
        let mut blocks = Vec::with_capacity(PADDED * PADDED * PADDED);
        for y in -1..=SIZE as i32 {
            for z in -1..=SIZE as i32 {
                for x in -1..=SIZE as i32 {
                    blocks.push(block(x, y, z));
                }
            }
        }
        Self {
            blocks,
            light: Light::None,
            origin: [0; 3],
            contents: Contents::ANY,
        }
    }

    /// Adds the light of the section and its border, `light(x, y, z)` as in `build`.
    pub fn with_light(mut self, light: impl Fn(i32, i32, i32) -> PackedLight) -> Self {
        let mut cells = Vec::with_capacity(PADDED * PADDED * PADDED);
        for y in -1..=SIZE as i32 {
            for z in -1..=SIZE as i32 {
                for x in -1..=SIZE as i32 {
                    cells.push(light(x, y, z));
                }
            }
        }
        self.light = Light::Cells(cells);
        self
    }

    /// Adds the light of the sections around, resolved per cell later by `resolve_light`
    /// (on a meshing thread, not the render thread).
    pub fn with_light_sections(mut self, sections: LightSections) -> Self {
        self.light = Light::Sections(sections);
        self
    }

    /// Copies the light of every cell out of the sections, for fast reads while meshing.
    pub fn resolve_light(&mut self) {
        if let Light::Sections(sections) = &self.light {
            let mut cells = Vec::with_capacity(PADDED * PADDED * PADDED);
            for y in -1..=SIZE as i32 {
                for z in -1..=SIZE as i32 {
                    for x in -1..=SIZE as i32 {
                        cells.push(section_light(sections, x, y, z));
                    }
                }
            }
            self.light = Light::Cells(cells);
        }
    }

    /// Where the section sits in the world, and what it may hold given its palette (every
    /// block it may contain, a superset).
    pub fn of_section(mut self, origin: [i32; 3], palette: &[BlockId], table: &BlockTable) -> Self {
        self.origin = origin;
        self.contents = Contents {
            liquid: palette.iter().any(|&id| table.family(id).is_some()),
            cross: palette
                .iter()
                .any(|&id| table.get(id).render == RenderKind::Cross),
        };
        self
    }

    pub fn origin(&self) -> [i32; 3] {
        self.origin
    }

    /// False only for the kinds the section surely does not hold.
    pub fn contents(&self) -> Contents {
        self.contents
    }

    pub fn get(&self, x: i32, y: i32, z: i32) -> BlockId {
        self.blocks[padded_index(x, y, z)]
    }

    /// Coordinates from -1 to 16, like `get`.
    pub fn light(&self, x: i32, y: i32, z: i32) -> PackedLight {
        match &self.light {
            Light::None => PackedLight::OPEN_SKY,
            Light::Sections(sections) => section_light(sections, x, y, z),
            Light::Cells(cells) => cells
                .get(padded_index(x, y, z))
                .copied()
                .unwrap_or(PackedLight::OPEN_SKY),
        }
    }
}

/// Coordinates from -1 to 16 relative to the centre section.
#[inline]
fn section_light(sections: &LightSections, x: i32, y: i32, z: i32) -> PackedLight {
    let split = |c: i32| ((c + 16).div_euclid(16) as usize, c.rem_euclid(16) as u32);
    let ((dx, lx), (dy, ly), (dz, lz)) = (split(x), split(y), split(z));
    sections
        .get(dy)
        .and_then(|plane| plane.get(dz))
        .and_then(|row| row.get(dx))
        .map_or(PackedLight::OPEN_SKY, |section| {
            section.get(section_index(lx, ly, lz))
        })
}

#[inline]
fn padded_index(x: i32, y: i32, z: i32) -> usize {
    let p = |c: i32| (c + 1) as usize;
    (p(y) * PADDED + p(z)) * PADDED + p(x)
}

const UNKNOWN: MeshBlock = MeshBlock {
    render: RenderKind::Invisible,
    cull_same: true,
    faces: [0; 6],
    turned: [false; 6],
    fluid: None,
    solid_cube: false,
    lamp: false,
    varied: false,
};

/// What the mesher needs to know about each block, indexed by `BlockId`.
#[derive(Debug, Clone)]
pub struct BlockTable {
    entries: Vec<MeshBlock>,
    variants: TextureVariants,
}

#[derive(Debug, Clone, Copy)]
pub struct MeshBlock {
    pub render: RenderKind,
    pub cull_same: bool,
    pub faces: [u16; 6],
    /// Faces whose texture is turned a quarter, for blocks lying on their side (logs).
    pub turned: [bool; 6],
    pub fluid: Option<MeshFluid>,
    /// Opaque and not a liquid: hides every neighbouring face.
    pub solid_cube: bool,
    /// Gives off light: a plane-shaped lamp (torch) is not vegetation, never thinned out.
    pub lamp: bool,
    /// Some face has random variants or connects to its neighbours.
    pub varied: bool,
}

/// A liquid cell: its family (source block), surface height and animation code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshFluid {
    pub family: BlockId,
    /// Sixteenths of a block (2 to 16).
    pub height: u8,
    pub animation: u8,
}

impl BlockTable {
    pub fn new<'a>(blocks: impl Iterator<Item = &'a Block>) -> Self {
        let entries = blocks
            .map(|block| MeshBlock {
                render: block.render,
                cull_same: block.cull_same,
                faces: Face::ALL.map(|face| block.texture(face).0),
                turned: Face::ALL.map(|face| {
                    block
                        .orientation
                        .is_some_and(|o| texture_turned(face, o.axis))
                }),
                solid_cube: block.render == RenderKind::Opaque && block.fluid.is_none(),
                lamp: block.light_emission > 0,
                varied: false,
                fluid: block.fluid.map(|state| MeshFluid {
                    family: state.source,
                    height: surface_sixteenths(state.level),
                    animation: 0,
                }),
            })
            .collect();
        Self {
            entries,
            variants: TextureVariants::default(),
        }
    }

    /// Adds the random and connected textures of the faces.
    pub fn with_variants(mut self, variants: TextureVariants) -> Self {
        for entry in &mut self.entries {
            entry.varied = entry.render != RenderKind::Cross
                && entry.faces.iter().any(|&layer| variants.varies(layer));
        }
        self.variants = variants;
        self
    }

    /// Every block of the content, liquids with their surface animation.
    pub fn from_content(content: &Content) -> Self {
        let mut table = Self::new(content.blocks.iter().map(|(_, _, block)| block))
            .with_variants(TextureVariants::from_library(&content.textures));
        for entry in &mut table.entries {
            if let Some(fluid) = &mut entry.fluid
                && let Some(def) = content.fluids.get(&fluid.family)
            {
                fluid.animation = def.view.animation.code();
            }
        }
        table
    }

    pub fn variants(&self) -> &TextureVariants {
        &self.variants
    }

    #[inline]
    pub fn get(&self, id: BlockId) -> &MeshBlock {
        self.entries.get(usize::from(id.0)).unwrap_or(&UNKNOWN)
    }

    #[inline]
    pub fn is_opaque(&self, id: BlockId) -> bool {
        self.get(id).render == RenderKind::Opaque
    }

    #[inline]
    pub fn family(&self, id: BlockId) -> Option<BlockId> {
        self.get(id).fluid.map(|fluid| fluid.family)
    }
}
