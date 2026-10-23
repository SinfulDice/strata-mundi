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

/// A GPU buffer rewritten every frame, reallocated only when it must grow (by doubling).
pub struct StreamBuffer {
    buffer: Option<wgpu::Buffer>,
    usage: wgpu::BufferUsages,
    label: &'static str,
}

impl StreamBuffer {
    pub fn new(label: &'static str, usage: wgpu::BufferUsages) -> Self {
        Self {
            buffer: None,
            usage: usage | wgpu::BufferUsages::COPY_DST,
            label,
        }
    }

    /// Uploads `data` (a multiple of 4 bytes) and returns the buffer holding it.
    pub fn write(&mut self, gpu: &super::Gpu, data: &[u8]) -> &wgpu::Buffer {
        let needed = (data.len() as u64).max(4);
        let buffer = match self.buffer.take().filter(|b| b.size() >= needed) {
            Some(buffer) => buffer,
            None => gpu.device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(self.label),
                size: needed.next_power_of_two(),
                usage: self.usage,
                mapped_at_creation: false,
            }),
        };
        gpu.queue.write_buffer(&buffer, 0, data);
        self.buffer.insert(buffer)
    }
}
