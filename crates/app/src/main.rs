#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod cli;
mod content;
mod launcher;
mod logging;
mod offline;
mod paths;

use clap::Parser;
use glam::DVec3;
use launcher::GameLauncher;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strata_client::bench::{BenchConfig, BenchScene};
use strata_client::camera::Camera;
use strata_client::interface_settings::InterfaceSettings;
use strata_client::screenshot::{View, capture};
use strata_client::settings::{ControlSettings, GraphicsSettings, Preset};
use strata_client::texture_sheet::write_texture_sheet;
use strata_client::{ClientConfig, DEFAULT_FOV, run};
use strata_core::dimension::DimensionId;
use strata_core::physics::EYE_HEIGHT;
use strata_server::TICKS_PER_SECOND;
use strata_worldgen::{Generator, Seed};

/// Copies of a world kept, made each time it is opened.
const BACKUPS_KEPT: usize = 3;
const AUTOSAVE_MINUTES: u64 = 5;

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
    let data = paths::data_dir(args.data_dir.clone());
    tracing::info!(path = %data.display(), "data folder");
    let interface_path = data.join("config/interface.ron");
    let mut interface = InterfaceSettings::load(&interface_path);
    if let Some(language) = &args.lang {
        interface.language.clone_from(language);
    }
    let content = Arc::new(loaded.content);
    let launcher = GameLauncher {
        content: Arc::clone(&content),
        worldgen: Arc::clone(&loaded.worldgen),
        dimension,
        saves: data.join("saves"),
        backups: data.join("backups"),
        backups_kept: BACKUPS_KEPT,
        autosave_ticks: AUTOSAVE_MINUTES * 60 * u64::from(TICKS_PER_SECOND),
        generation_threads: graphics.generation_threads,
        view_distance: graphics.render_distance,
    };
    let direct = bench.is_some() || args.seed.is_some();
    let start = direct.then(|| launcher.temporary(seed)).transpose()?;
    Ok(run(ClientConfig {
        content,
        controls: ControlSettings::default(),
        ui: loaded.ui,
        interface,
        interface_path,
        vsync: bench.is_none(),
        graphics,
        bench,
        launcher: Box::new(launcher),
        start,
    })?)
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
