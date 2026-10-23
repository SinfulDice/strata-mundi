//! `strata --bench-scene <scene> --preset <profile>`: scripted camera path, frame statistics
//! and a Markdown report in `perf-reports/`, to be measured on the player's own machine.

mod gpu_timer;
mod report;
mod scene;
mod stats;
mod system;

pub use scene::{BenchScene, SceneError};

use crate::camera::Camera;
use crate::render::Gpu;
use glam::DVec3;
use gpu_timer::GpuTimer;
use report::Report;
use stats::Samples;
use std::path::PathBuf;
use std::time::Instant;
use system::{MachineInfo, MemorySampler};

pub struct BenchConfig {
    pub scene_name: String,
    pub scene: BenchScene,
    pub preset: String,
    pub report_dir: PathBuf,
}

const MEMORY_SAMPLE_PERIOD: f32 = 0.5;

pub struct BenchRun {
    config: BenchConfig,
    elapsed: f32,
    origin: DVec3,
    frames: Samples,
    cpu: Samples,
    gpu: Vec<f32>,
    timer: Option<GpuTimer>,
    memory: MemorySampler,
    since_memory_sample: f32,
    start: Option<(Instant, u64)>,
    end: Option<(Instant, u64)>,
}

impl BenchRun {
    pub fn new(config: BenchConfig, gpu: &Gpu) -> Self {
        let timer = gpu
            .supports_timestamps()
            .then(|| GpuTimer::new(&gpu.device, &gpu.queue));
        Self {
            config,
            elapsed: 0.0,
            origin: DVec3::ZERO,
            frames: Samples::default(),
            cpu: Samples::default(),
            gpu: Vec::new(),
            timer,
            memory: MemorySampler::new(),
            since_memory_sample: MEMORY_SAMPLE_PERIOD,
            start: None,
            end: None,
        }
    }

    /// Keyframe positions are relative to this point (the world spawn).
    pub fn set_origin(&mut self, origin: DVec3) {
        self.origin = origin;
    }

    /// Whether measuring has begun (it never pauses once started).
    pub fn started(&self) -> bool {
        self.elapsed > 0.0
    }

    /// Moves the camera along the scripted path.
    pub fn advance(&mut self, dt: f32, camera: &mut Camera) {
        self.elapsed += dt;
        self.config.scene.pose(self.measured_time(), camera);
        camera.position += self.origin;
    }

    pub fn timestamp_writes(&mut self) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        if !self.is_measuring() {
            return None;
        }
        self.timer.as_mut()?.begin_frame()
    }

    pub fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        if let Some(timer) = &self.timer {
            timer.resolve(encoder);
        }
    }

    pub fn after_submit(&mut self) {
        if let Some(timer) = &mut self.timer {
            timer.after_submit();
        }
    }

    pub fn record_frame(&mut self, gpu: &Gpu, frame_ms: f32, cpu_ms: f32, server_tick: u64) {
        if let Some(timer) = &mut self.timer {
            timer.collect(&gpu.device, &mut self.gpu);
        }
        if !self.is_measuring() {
            self.gpu.clear();
            return;
        }
        let now = Instant::now();
        self.start.get_or_insert((now, server_tick));
        self.end = Some((now, server_tick));
        self.frames.push(frame_ms);
        self.cpu.push(cpu_ms);
        self.since_memory_sample += frame_ms / 1000.0;
        if self.since_memory_sample >= MEMORY_SAMPLE_PERIOD {
            self.memory.sample();
            self.since_memory_sample = 0.0;
        }
    }

    pub fn is_finished(&self) -> bool {
        self.measured_time() >= self.config.scene.duration
    }

    pub fn write_report(
        &self,
        gpu: &Gpu,
        resolution: (u32, u32),
        present_mode: wgpu::PresentMode,
    ) -> std::io::Result<PathBuf> {
        let machine = MachineInfo::detect();
        let adapter = gpu.adapter.get_info();
        let gpu_samples = (!self.gpu.is_empty()).then(|| {
            let mut samples = Samples::default();
            self.gpu.iter().for_each(|&ms| samples.push(ms));
            samples
        });
        let report = Report {
            scene: &self.config.scene_name,
            preset: &self.config.preset,
            machine: &machine,
            adapter: &adapter,
            resolution,
            present_mode: &format!("{present_mode:?}"),
            duration_seconds: self.measured_wall_seconds(),
            frames: &self.frames,
            cpu: &self.cpu,
            gpu: gpu_samples.as_ref(),
            peak_memory_bytes: self.memory.peak_bytes,
            server_ticks_per_second: self.server_tick_rate(),
        };
        let date = chrono::Local::now().format("%Y-%m-%d").to_string();
        std::fs::create_dir_all(&self.config.report_dir)?;
        let path = report.free_path(&self.config.report_dir, &date);
        std::fs::write(&path, report.to_markdown(&date))?;
        Ok(path)
    }

    fn measured_time(&self) -> f32 {
        self.elapsed - self.config.scene.warmup
    }

    fn is_measuring(&self) -> bool {
        self.measured_time() >= 0.0 && !self.is_finished()
    }

    fn measured_wall_seconds(&self) -> f32 {
        match (self.start, self.end) {
            (Some((t0, _)), Some((t1, _))) => (t1 - t0).as_secs_f32(),
            _ => 0.0,
        }
    }

    fn server_tick_rate(&self) -> f32 {
        match (self.start, self.end) {
            (Some((t0, tick0)), Some((t1, tick1))) if t1 > t0 => {
                (tick1 - tick0) as f32 / (t1 - t0).as_secs_f32()
            }
            _ => 0.0,
        }
    }
}
