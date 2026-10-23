#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod cli;
mod content;
mod logging;
mod offline;
mod paths;

use anyhow::Context;
use clap::Parser;
use glam::DVec3;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strata_client::bench::{BenchConfig, BenchScene};
use strata_client::camera::Camera;
use strata_client::camera::EYE_HEIGHT;
use strata_client::screenshot::{View, capture};
use strata_client::settings::{GraphicsSettings, Preset};
use strata_client::texture_sheet::write_texture_sheet;
use strata_client::{ClientConfig, DEFAULT_FOV, run};
use strata_core::dimension::DimensionId;
use strata_server::{IntegratedServer, ServerConfig};
use strata_worldgen::{Generator, Seed};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    logging::init()?;
    tracing::info!(version = env!("CARGO_PKG_VERSION"), preset = %args.preset, "starting Strata Mundi");
    let dimension = DimensionId(args.dimension.clone());
    let loaded = content::load(&args, &dimension)?;
    let graphics = GraphicsSettings::for_preset(args.preset);
    if let Some(path) = &args.texture_sheet {
        return Ok(write_texture_sheet(
            &loaded.content,
            &content::preview(&loaded),
            path,
        )?);
    }
    let seed = args
        .seed
        .as_deref()
        .map_or_else(random_seed, Seed::from_text);
    tracing::info!(seed = seed.0 as i64, "world seed");
    let generator = Arc::new(Generator::new(seed, Arc::clone(&loaded.worldgen)));
    if let Some(path) = &args.screenshot {
        return screenshot(&args, &loaded, &generator, graphics, path);
    }
    let bench = args
        .bench_scene
        .as_deref()
        .map(|name| load_bench(&loaded.assets, name, args.preset, args.report_dir.clone()))
        .transpose()?;
    let server = ServerConfig {
        dimension,
        generator,
        generation_threads: graphics.generation_threads,
        view_distance: graphics.render_distance,
    };
    play(
        ClientConfig {
            content: Arc::new(loaded.content),
            language: args.lang.clone(),
            camera: args.pos.map(|pos| camera(pos, &args)),
            vsync: bench.is_none(),
            graphics,
            bench,
        },
        server,
    )
}

/// With `--seed`, a freshly generated world around `--pos` (or the spawn point);
/// without it, the preview diorama.
fn screenshot(
    args: &cli::Args,
    loaded: &content::Loaded,
    generator: &Arc<Generator>,
    graphics: GraphicsSettings,
    path: &Path,
) -> anyhow::Result<()> {
    let (world, position) = if args.seed.is_some() {
        let position = args.pos.unwrap_or_else(|| {
            let spawn = generator.spawn_point();
            [
                f64::from(spawn.x) + 0.5,
                f64::from(spawn.y) + EYE_HEIGHT,
                f64::from(spawn.z) + 0.5,
            ]
        });
        let world = offline::generate_around(
            generator,
            position[0],
            position[2],
            graphics.render_distance,
        );
        (world, position)
    } else {
        (
            content::preview(loaded),
            args.pos.unwrap_or([-12.0, 15.5, 14.0]),
        )
    };
    let view = View {
        camera: camera(position, args),
        size: args.size,
        graphics,
    };
    Ok(capture(&loaded.content, &world, &view, path)?)
}

fn play(client: ClientConfig, server: ServerConfig) -> anyhow::Result<()> {
    let (client_end, server_end) = strata_protocol::local_pair();
    let server =
        IntegratedServer::spawn(server_end, server).context("could not start the server thread")?;
    let result = run(client, client_end);
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

fn camera([x, y, z]: [f64; 3], args: &cli::Args) -> Camera {
    let [yaw, pitch] = args.look.unwrap_or([0.0, -20.0]);
    Camera::new(DVec3::new(x, y, z), yaw, pitch, DEFAULT_FOV)
}

/// Like Minecraft, a world without a chosen seed gets a random one.
fn random_seed() -> Seed {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    Seed((nanos as u64) ^ ((nanos >> 64) as u64)).mixed()
}
