#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod cli;
mod logging;
mod paths;

use anyhow::Context;
use clap::Parser;
use glam::DVec3;
use std::path::PathBuf;
use strata_client::bench::{BenchConfig, BenchScene};
use strata_client::camera::Camera;
use strata_client::screenshot::{ScreenshotRequest, capture};
use strata_client::{ClientConfig, run};
use strata_server::IntegratedServer;

const DEFAULT_FOV: f32 = 70.0;

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    logging::init()?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), preset = %args.preset, "starting Strata Mundi");
    let camera = initial_camera(&args);
    if let Some(path) = args.screenshot {
        return Ok(capture(&ScreenshotRequest {
            path,
            size: args.size,
            camera,
        })?);
    }
    let bench = args
        .bench_scene
        .as_deref()
        .map(|name| load_bench(name, args.preset, args.report_dir.clone()))
        .transpose()?;
    play(ClientConfig {
        camera,
        vsync: bench.is_none(),
        bench,
    })
}

fn play(config: ClientConfig) -> anyhow::Result<()> {
    let (client, server) = strata_protocol::local_pair();
    let server = IntegratedServer::spawn(server).context("could not start the server thread")?;
    let result = run(config, client);
    if server.join().is_err() {
        tracing::error!("server thread panicked");
    }
    Ok(result?)
}

fn load_bench(
    name: &str,
    preset: strata_client::settings::Preset,
    report_dir: PathBuf,
) -> anyhow::Result<BenchConfig> {
    let assets = paths::assets_dir().context("assets folder not found")?;
    let scene = BenchScene::load(&assets.join("bench-scenes").join(format!("{name}.ron")))?;
    Ok(BenchConfig {
        scene_name: name.to_owned(),
        scene,
        preset: preset.id().to_owned(),
        report_dir,
    })
}

fn initial_camera(args: &cli::Args) -> Camera {
    let [x, y, z] = args.pos.unwrap_or([0.5, 0.5, 3.0]);
    let [yaw, pitch] = args.look.unwrap_or([0.0, 0.0]);
    Camera::new(DVec3::new(x, y, z), yaw, pitch, DEFAULT_FOV)
}
