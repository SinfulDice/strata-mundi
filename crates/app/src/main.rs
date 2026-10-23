#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod cli;
mod content;
mod logging;
mod paths;

use anyhow::Context;
use clap::Parser;
use glam::DVec3;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strata_client::bench::{BenchConfig, BenchScene};
use strata_client::camera::Camera;
use strata_client::screenshot::{View, capture};
use strata_client::settings::{GraphicsSettings, Preset};
use strata_client::texture_sheet::write_texture_sheet;
use strata_client::{ClientConfig, run};
use strata_server::IntegratedServer;

const DEFAULT_FOV: f32 = 70.0;

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    logging::init()?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), preset = %args.preset, "starting Strata Mundi");
    let loaded = content::load(&args)?;
    let camera = initial_camera(&args);
    if let Some(path) = &args.texture_sheet {
        return Ok(write_texture_sheet(&loaded.content, &loaded.preview, path)?);
    }
    let graphics = GraphicsSettings::for_preset(args.preset);
    if let Some(path) = &args.screenshot {
        let view = View {
            camera,
            size: args.size,
            graphics,
        };
        return Ok(capture(&loaded.content, &loaded.preview, &view, path)?);
    }
    let bench = args
        .bench_scene
        .as_deref()
        .map(|name| load_bench(&loaded.assets, name, args.preset, args.report_dir.clone()))
        .transpose()?;
    play(ClientConfig {
        content: Arc::new(loaded.content),
        world: loaded.preview,
        graphics,
        language: args.lang.clone(),
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
    assets: &Path,
    name: &str,
    preset: Preset,
    report_dir: PathBuf,
) -> anyhow::Result<BenchConfig> {
    let scene = BenchScene::load(&assets.join("bench-scenes").join(format!("{name}.ron")))?;
    Ok(BenchConfig {
        scene_name: name.to_owned(),
        scene,
        preset: preset.id().to_owned(),
        report_dir,
    })
}

fn initial_camera(args: &cli::Args) -> Camera {
    let [x, y, z] = args.pos.unwrap_or([-9.5, 13.5, 11.0]);
    let [yaw, pitch] = args.look.unwrap_or([41.0, -27.0]);
    Camera::new(DVec3::new(x, y, z), yaw, pitch, DEFAULT_FOV)
}
