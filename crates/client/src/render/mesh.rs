use crate::mesher::MeshData;
use wgpu::util::DeviceExt;

/// A mesh uploaded to the GPU.
pub struct GpuMesh {
    vertices: wgpu::Buffer,
    indices: wgpu::Buffer,
    index_count: u32,
}

impl GpuMesh {
    /// `None` for an empty mesh: nothing to draw, nothing to allocate.
    pub fn upload(device: &wgpu::Device, data: &MeshData) -> Option<Self> {
        if data.is_empty() {
            return None;
        }
        let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("section vertices"),
            contents: bytemuck::cast_slice(&data.vertices),
            usage: wgpu::BufferUsages::VERTEX,
        });
        let indices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("section indices"),
            contents: bytemuck::cast_slice(&data.indices),
            usage: wgpu::BufferUsages::INDEX,
        });
        Some(Self {
            vertices,
            indices,
            index_count: data.indices.len() as u32,
        })
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.vertices.slice(..));
        pass.set_index_buffer(self.indices.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..self.index_count, 0, 0..1);
    }
}
