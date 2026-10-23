use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct BlockVertex {
    pub position: [f32; 3],
    pub uv: [f32; 2],
    pub layer: u32,
    /// Directional shading baked per face (top brightest), like Minecraft.
    pub shade: f32,
}

impl BlockVertex {
    pub const LAYOUT: wgpu::VertexBufferLayout<'static> = wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Self>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 1 => Float32x2, 2 => Uint32, 3 => Float32],
    };
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct MeshData {
    pub vertices: Vec<BlockVertex>,
    pub indices: Vec<u32>,
}

/// A mesh uploaded to the GPU.
pub struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl GpuMesh {
    pub fn upload(device: &wgpu::Device, data: &MeshData) -> Self {
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh vertices"),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("mesh indices"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Self {
            vertices,
            indices,
            index_count: data.indices.len() as u32,
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}

/// Unit cube from (0,0,0) to (1,1,1), every face using texture `layer`.
pub fn cube(layer: u32) -> MeshData {
    const FACES: [([[f32; 3]; 4], f32); 6] = [
        (
            [[0., 1., 1.], [1., 1., 1.], [1., 1., 0.], [0., 1., 0.]],
            1.0,
        ),
        (
            [[0., 0., 0.], [1., 0., 0.], [1., 0., 1.], [0., 0., 1.]],
            0.5,
        ),
        (
            [[0., 1., 0.], [1., 1., 0.], [1., 0., 0.], [0., 0., 0.]],
            0.8,
        ),
        (
            [[1., 1., 1.], [0., 1., 1.], [0., 0., 1.], [1., 0., 1.]],
            0.8,
        ),
        (
            [[1., 1., 0.], [1., 1., 1.], [1., 0., 1.], [1., 0., 0.]],
            0.6,
        ),
        (
            [[0., 1., 1.], [0., 1., 0.], [0., 0., 0.], [0., 0., 1.]],
            0.6,
        ),
    ];
    const UVS: [[f32; 2]; 4] = [[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let mut mesh = MeshData::default();
    for (corners, shade) in FACES {
        let base = mesh.vertices.len() as u32;
        for (position, uv) in corners.into_iter().zip(UVS) {
            mesh.vertices.push(BlockVertex {
                position,
                uv,
                layer,
                shade,
            });
        }
        mesh.indices
            .extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }
    mesh
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::Vec3;

    #[test]
    fn cube_faces_wind_counter_clockwise_outward() {
        let mesh = cube(0);
        assert_eq!(mesh.indices.len(), 36);
        for triangle in mesh.indices.chunks(3) {
            let [a, b, c] =
                [0, 1, 2].map(|i| Vec3::from(mesh.vertices[triangle[i] as usize].position));
            let normal = (b - a).cross(c - a);
            let outward = (a + b + c) / 3.0 - Vec3::splat(0.5);
            assert!(normal.dot(outward) > 0.0);
        }
    }
}
