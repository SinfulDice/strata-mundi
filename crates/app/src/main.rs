#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod cli;
mod content;
mod crash;
mod launcher;
mod logging;
mod offline;
mod paths;
mod updates;

use clap::Parser;
use glam::DVec3;
use launcher::GameLauncher;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strata_client::bench::{BenchConfig, BenchScene};
use strata_client::camera::Camera;
use strata_client::launcher::Launcher;
use strata_client::screenshot::{View, capture};
use strata_client::settings::{GraphicsSettings, Preset, Settings};
use strata_client::texture_sheet::write_texture_sheet;
use strata_client::{ClientConfig, DEFAULT_FOV, run};
use strata_core::dimension::DimensionId;
use strata_core::physics::EYE_HEIGHT;
use strata_worldgen::{Generator, Seed};

fn main() -> anyhow::Result<()> {
    let args = cli::Args::parse();
    let data = paths::data_dir(args.data_dir.clone());
    let crash_reports = data.join("crash-reports");
    crash::install(crash_reports.clone(), logging::init()?);
    tracing::info!(version = env!("CARGO_PKG_VERSION"), "starting Strata Mundi");
    let dimension = DimensionId(args.dimension.clone());
    let loaded = content::load(&args, &dimension)?;
    let tool_preset = args.preset.unwrap_or(Preset::High);
    let graphics = GraphicsSettings::for_preset(tool_preset);
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
        .map(|name| load_bench(&loaded.assets, name, tool_preset, args.report_dir.clone()))
        .transpose()?;
    tracing::info!(path = %data.display(), "data folder");
    let settings_path = data.join("config/settings.toml");
    let settings = settings_for(&args, &settings_path, bench.is_some());
    let content = Arc::new(loaded.content);
    let direct = bench.is_some() || args.seed.is_some();
    let mut launcher = GameLauncher {
        content: Arc::clone(&content),
        worldgen: Arc::clone(&loaded.worldgen),
        dimension,
        saves: data.join("saves"),
        backups: data.join("backups"),
        backups_kept: 0,
        autosave_ticks: 0,
        generation_threads: 1,
        view_distance: 2,
        crash_reports,
        updates: (settings.game.check_updates && !direct).then(updates::UpdateCheck::start),
    };
    launcher.configure(&settings);
    let start = direct.then(|| launcher.temporary(seed)).transpose()?;
    Ok(run(ClientConfig {
        content,
        ui: loaded.ui,
        settings,
        settings_path,
        bench,
        launcher: Box::new(launcher),
        start,
    })?)
}

/// The saved settings, with the command line on top (`--preset`, `--lang`); benchmarks
/// run without frame limit or VSync, so they measure the machine.
fn settings_for(args: &cli::Args, path: &Path, bench: bool) -> Settings {
    let mut settings = Settings::load(path);
    if let Some(preset) = args.preset {
        settings.apply_preset(preset);
    }
    if let Some(language) = &args.lang {
        settings.interface.language.clone_from(language);
    }
    if bench {
        settings.graphics.vsync = false;
        settings.graphics.fps_limit = 0;
    }
    settings
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
pub fn random_seed() -> Seed {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    Seed((nanos as u64) ^ ((nanos >> 64) as u64)).mixed()
}
