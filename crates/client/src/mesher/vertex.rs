use bytemuck::{Pod, Zeroable};
use strata_core::content::Tint;
use strata_core::light::PackedLight;

/// Value of the face field for plant planes, after the six cube faces.
pub const CROSS_FACE: usize = 6;

/// 12-byte vertex: position inside the section (0..=16), face, ambient occlusion, texture
/// repeat (0..=16), texture layer, liquid surface, light and biome colour, decoded by the
/// shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct PackedVertex {
    /// x:5 y:5 z:5 face:3 ao:2 u:5 v:5
    pub geometry: u32,
    /// layer:16 drop:4 animation:2 free:2 block_light:4 sky_light:4. `drop` lowers the
    /// vertex by sixteenths of a block (liquid surfaces).
    pub texture: u32,
    /// r:8 g:8 b:8 (`Tint` factors, 128 = ×1) masked:1 free:7. `masked`: pixels with alpha
    /// `UNTINTED_ALPHA` keep their colour (grass fringe on the sides).
    pub tint: u32,
}

const NEUTRAL_TINT: u32 = 0x0080_8080;
const MASKED: u32 = 1 << 24;

/// Lowest a vertex can be pushed down, in sixteenths.
pub const MAX_DROP: u8 = 15;
const SURFACE_BITS: u32 = 0x003f_0000;
const LIGHT_BITS: u32 = 0xff00_0000;

impl PackedVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Uint32, 1 => Uint32, 2 => Uint32],
    };

    pub fn new(position: [u32; 3], face: usize, ao: u8, uv: [u32; 2], layer: u16) -> Self {
        debug_assert!(position.iter().chain(&uv).all(|&c| c <= 16) && ao < 4 && face <= CROSS_FACE);
        let geometry = position[0]
            | (position[1] << 5)
            | (position[2] << 10)
            | ((face as u32) << 15)
            | (u32::from(ao) << 18)
            | (uv[0] << 20)
            | (uv[1] << 25);
        Self {
            geometry,
            texture: u32::from(layer),
            tint: NEUTRAL_TINT,
        }
    }

    pub fn with_tint(self, tint: Tint, masked: bool) -> Self {
        let [r, g, b] = tint.0.map(u32::from);
        Self {
            tint: r | (g << 8) | (b << 16) | if masked { MASKED } else { 0 },
            ..self
        }
    }

    pub fn tint(self) -> (Tint, bool) {
        let channel = |shift: u32| (self.tint >> shift) as u8;
        let color = Tint([channel(0), channel(8), channel(16)]);
        (color, self.tint & MASKED != 0)
    }

    /// A liquid vertex: lowered by `drop` sixteenths, animated with `animation` (0 = still).
    pub fn with_surface(self, drop: u8, animation: u8) -> Self {
        debug_assert!(drop <= MAX_DROP && animation < 4);
        Self {
            texture: (self.texture & !SURFACE_BITS)
                | (u32::from(drop.min(MAX_DROP)) << 16)
                | (u32::from(animation & 3) << 20),
            ..self
        }
    }

    pub fn with_light(self, light: PackedLight) -> Self {
        Self {
            texture: (self.texture & !LIGHT_BITS) | (u32::from(light.0) << 24),
            ..self
        }
    }

    pub fn light(self) -> PackedLight {
        PackedLight((self.texture >> 24) as u8)
    }

    pub fn drop(self) -> u8 {
        ((self.texture >> 16) & 15) as u8
    }

    pub fn animation(self) -> u8 {
        ((self.texture >> 20) & 3) as u8
    }

    pub fn position(self) -> [u32; 3] {
        [
            self.geometry & 31,
            (self.geometry >> 5) & 31,
            (self.geometry >> 10) & 31,
        ]
    }

    pub fn ao(self) -> u8 {
        ((self.geometry >> 18) & 3) as u8
    }
}

/// Vertices and triangle indices of one mesh, before upload.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct MeshData {
    pub vertices: Vec<PackedVertex>,
    pub indices: Vec<u32>,
}

impl MeshData {
    pub fn is_empty(&self) -> bool {
        self.indices.is_empty()
    }

    pub fn quad_count(&self) -> usize {
        self.indices.len() / 6
    }

    /// Corners in counter-clockwise order seen from outside. The diagonal is chosen so that
    /// ambient occlusion and light interpolate without the classic anisotropy artefact.
    pub fn push_quad(&mut self, corners: [PackedVertex; 4]) {
        let base = self.vertices.len() as u32;
        let shade = corners.map(|c| {
            let light = c.light();
            (u32::from(c.ao()), u32::from(light.sky().max(light.block())))
        });
        self.vertices.extend_from_slice(&corners);
        let sum = |i: usize, j: usize| (shade[i].0 + shade[j].0, shade[i].1 + shade[j].1);
        let indices = if sum(0, 2) < sum(1, 3) {
            [base + 1, base + 2, base + 3, base + 1, base + 3, base]
        } else {
            [base, base + 1, base + 2, base, base + 2, base + 3]
        };
        self.indices.extend_from_slice(&indices);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packing_round_trips() {
        let vertex = PackedVertex::new([16, 0, 7], 4, 3, [16, 5], 900);
        assert_eq!(vertex.position(), [16, 0, 7]);
        assert_eq!(vertex.ao(), 3);
        assert_eq!(vertex.texture, 900);
        assert_eq!((vertex.geometry >> 15) & 7, 4);
        assert_eq!((vertex.geometry >> 20) & 31, 16);
        assert_eq!((vertex.geometry >> 25) & 31, 5);
        let surface = vertex.with_surface(MAX_DROP, 2);
        assert_eq!((surface.drop(), surface.animation()), (MAX_DROP, 2));
        assert_eq!(surface.texture & 0xffff, 900);
        let lit = surface.with_light(PackedLight::new(12, 5));
        assert_eq!(lit.light(), PackedLight::new(12, 5));
        assert_eq!((lit.drop(), lit.animation()), (MAX_DROP, 2));
        assert_eq!(lit.with_surface(3, 1).light(), PackedLight::new(12, 5));
        assert_eq!(lit.tint(), (Tint::NEUTRAL, false));
        let tinted = lit.with_tint(Tint([1, 200, 255]), true);
        assert_eq!(tinted.tint(), (Tint([1, 200, 255]), true));
        assert_eq!(tinted.light(), PackedLight::new(12, 5));
    }
}
