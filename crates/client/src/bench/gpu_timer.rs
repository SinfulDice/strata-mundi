use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};

const SLOTS: usize = 4;
const RESOLVE_STRIDE: u64 = wgpu::QUERY_RESOLVE_BUFFER_ALIGNMENT;
const FREE: u8 = 0;
const IN_FLIGHT: u8 = 1;
const READY: u8 = 2;

/// Measures GPU time of the world pass with timestamp queries, read back a few frames later
/// so measuring never stalls the pipeline.
pub struct GpuTimer {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    slots: Vec<Slot>,
    current: Option<usize>,
    next: usize,
    period_ns: f32,
}

struct Slot {
    readback: wgpu::Buffer,
    state: Arc<AtomicU8>,
}

impl GpuTimer {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let queries = device.create_query_set(&wgpu::QuerySetDescriptor {
            label: Some("gpu timer"),
            ty: wgpu::QueryType::Timestamp,
            count: (SLOTS * 2) as u32,
        });
        let resolve = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpu timer resolve"),
            size: RESOLVE_STRIDE * SLOTS as u64,
            usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let slots = (0..SLOTS)
            .map(|_| Slot {
                readback: device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("gpu timer readback"),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                state: Arc::new(AtomicU8::new(FREE)),
            })
            .collect();
        Self {
            queries,
            resolve,
            slots,
            current: None,
            next: 0,
            period_ns: queue.get_timestamp_period(),
        }
    }

    /// Timestamp writes for this frame's pass, or `None` when every slot is still being read.
    pub fn begin_frame(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let index = self.next;
        if self.slots[index].state.load(Ordering::Acquire) != FREE {
            self.current = None;
            return None;
        }
        self.current = Some(index);
        self.next = (index + 1) % SLOTS;
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(index as u32 * 2),
            end_of_pass_write_index: Some(index as u32 * 2 + 1),
        })
    }

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        let Some(index) = self.current else {
            return;
        };
        let first = index as u32 * 2;
        let offset = RESOLVE_STRIDE * index as u64;
        encoder.resolve_query_set(&self.queries, first..first + 2, &self.resolve, offset);
        encoder.copy_buffer_to_buffer(&self.resolve, offset, &self.slots[index].readback, 0, 16);
    }

    /// Call after `queue.submit`.
    pub fn after_submit(&mut self) {
        let Some(index) = self.current.take() else {
            return;
        };
        let state = Arc::clone(&self.slots[index].state);
        state.store(IN_FLIGHT, Ordering::Release);
        self.slots[index]
            .readback
            .map_async(wgpu::MapMode::Read, .., move |result| {
                state.store(if result.is_ok() { READY } else { FREE }, Ordering::Release);
            });
    }

    /// Collects finished measurements, in milliseconds.
    pub fn collect(&mut self, device: &wgpu::Device, out: &mut Vec<f32>) {
        let _ = device.poll(wgpu::PollType::Poll);
        for slot in &self.slots {
            if slot.state.load(Ordering::Acquire) != READY {
                continue;
            }
            if let Ok(view) = slot.readback.get_mapped_range(..) {
                let ticks: [u64; 2] = bytemuck::pod_read_unaligned(&view[..16]);
                let elapsed = ticks[1].saturating_sub(ticks[0]) as f32 * self.period_ns;
                out.push(elapsed / 1_000_000.0);
            }
            slot.readback.unmap();
            slot.state.store(FREE, Ordering::Release);
        }
    }
}
