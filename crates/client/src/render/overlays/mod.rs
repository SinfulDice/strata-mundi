//! What is drawn over the world's blocks: the outline of the aimed block and the cracks
//! of the blocks being broken.

mod cracks;
mod outline;

pub use cracks::{CrackDraw, MAX_CRACKS};

use super::Gpu;
use super::block_pipeline::BlockPipeline;
use crate::camera::Camera;
use cracks::CrackPipeline;
use glam::DVec3;
use outline::OutlinePipeline;
use strata_core::coords::BlockPos;

pub struct Overlays {
    outline_pipeline: OutlinePipeline,
    crack_pipeline: CrackPipeline,
    /// Block drawn with an outline (the one the player aims at).
    pub outline: Option<BlockPos>,
    /// Blocks being broken, drawn cracked this frame.
    pub cracks: Vec<CrackDraw>,
}

impl Overlays {
    pub(super) fn new(
        device: &wgpu::Device,
        color_format: wgpu::TextureFormat,
        blocks: &BlockPipeline,
    ) -> Self {
        Self {
            outline_pipeline: OutlinePipeline::new(device, color_format, &blocks.camera_layout),
            crack_pipeline: CrackPipeline::new(device, color_format, blocks),
            outline: None,
            cracks: Vec::new(),
        }
    }

    /// Uploads this frame's cracks; call before the render pass begins.
    pub(super) fn prepare(&mut self, gpu: &Gpu) {
        self.crack_pipeline.prepare(gpu, &self.cracks);
    }

    pub(super) fn draw(
        &self,
        gpu: &Gpu,
        pass: &mut wgpu::RenderPass<'_>,
        camera: &Camera,
        blocks: &BlockPipeline,
    ) {
        self.crack_pipeline.draw(pass, blocks);
        if let Some(block) = self.outline {
            let corner = DVec3::new(f64::from(block.x), f64::from(block.y), f64::from(block.z));
            self.outline_pipeline
                .draw(&gpu.queue, pass, camera.relative(corner));
        }
    }

    pub(super) fn clear(&mut self) {
        self.outline = None;
        self.cracks.clear();
    }
}
