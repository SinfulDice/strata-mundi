use bytemuck::{Pod, Zeroable};

/// 8-byte vertex: position inside the section (0..=16), face, ambient occlusion, texture
/// repeat (0..=16) and texture layer, decoded by the shader.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct PackedVertex {
    /// x:5 y:5 z:5 face:3 ao:2 u:5 v:5
    pub geometry: u32,
    /// layer:16
    pub texture: u32,
}

impl PackedVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Uint32, 1 => Uint32],
    };

    pub fn new(position: [u32; 3], face: usize, ao: u8, uv: [u32; 2], layer: u16) -> Self {
        debug_assert!(position.iter().chain(&uv).all(|&c| c <= 16) && ao < 4 && face < 6);
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
        }
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
    /// ambient occlusion interpolates without the classic anisotropy artefact.
    pub fn push_quad(&mut self, corners: [PackedVertex; 4]) {
        let base = self.vertices.len() as u32;
        let ao = corners.map(|c| u32::from(c.ao()));
        self.vertices.extend_from_slice(&corners);
        let indices = if ao[0] + ao[2] < ao[1] + ao[3] {
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
    }
}
