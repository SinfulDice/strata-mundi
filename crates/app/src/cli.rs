use clap::Parser;
use std::path::PathBuf;
use strata_client::settings::Preset;
use strata_core::id::Identifier;

/// Strata Mundi — voxel sandbox.
#[derive(Debug, Parser)]
#[command(name = "strata", version, about)]
pub struct Args {
    /// Render a single frame without a window and save it as PNG, then quit.
    #[arg(long, value_name = "PNG")]
    pub screenshot: Option<PathBuf>,

    /// World seed: a number or any text. Random if omitted.
    #[arg(long, allow_hyphen_values = true)]
    pub seed: Option<String>,

    /// Dimension to play in.
    #[arg(long, default_value = "strata:terra")]
    pub dimension: Identifier,

    /// Camera position `x,y,z` (screenshot and initial position).
    #[arg(long, value_name = "X,Y,Z", value_parser = parse_vec3, allow_hyphen_values = true)]
    pub pos: Option<[f64; 3]>,

    /// Camera orientation `yaw,pitch` in degrees (0,0 looks north, toward -Z).
    #[arg(long, value_name = "YAW,PITCH", value_parser = parse_vec2, allow_hyphen_values = true)]
    pub look: Option<[f32; 2]>,

    /// Save every texture side by side above a render of the preview scene, then quit.
    #[arg(long, value_name = "PNG")]
    pub texture_sheet: Option<PathBuf>,

    /// Activate a resource pack from the resource packs folder (repeatable; last wins).
    #[arg(long, value_name = "NAME")]
    pub resource_pack: Vec<String>,

    /// Folder containing resource packs.
    #[arg(long, value_name = "DIR", default_value = "resourcepacks")]
    pub resource_packs_dir: PathBuf,

    /// Language: fr, en or lol (otherwise the one chosen in the options).
    #[arg(long, value_parser = strata_core::lang::LANGUAGES)]
    pub lang: Option<String>,

    /// Folder for saves, backups and settings (default: the system's data folder).
    #[arg(long, value_name = "DIR")]
    pub data_dir: Option<PathBuf>,

    /// Run a scripted scene from `assets/bench-scenes/<SCENE>.ron` and write a report
    /// in `perf-reports/`.
    #[arg(long, value_name = "SCENE")]
    pub bench_scene: Option<String>,

    /// Folder where `--bench-scene` writes its report.
    #[arg(long, value_name = "DIR", default_value = "perf-reports")]
    pub report_dir: PathBuf,

    /// Quality preset: very-low, low, medium, high, ultra, ds. For the game, it replaces
    /// the graphics settings; captures and benchmarks default to `high`.
    #[arg(long)]
    pub preset: Option<Preset>,

    /// Screenshot size `WIDTHxHEIGHT`.
    #[arg(long, value_name = "WxH", value_parser = parse_size, default_value = "1280x720")]
    pub size: (u32, u32),
}

fn parse_numbers<T: std::str::FromStr, const N: usize>(text: &str) -> Result<[T; N], String> {
    let parts: Vec<T> = text
        .split(',')
        .map(|part| {
            part.trim()
                .parse::<T>()
                .map_err(|_| format!("invalid number: `{part}`"))
        })
        .collect::<Result<_, _>>()?;
    parts
        .try_into()
        .map_err(|_| format!("expected {N} comma-separated numbers"))
}

fn parse_vec3(text: &str) -> Result<[f64; 3], String> {
    parse_numbers(text)
}

fn parse_vec2(text: &str) -> Result<[f32; 2], String> {
    parse_numbers(text)
}

fn parse_size(text: &str) -> Result<(u32, u32), String> {
    let (width, height) = text
        .split_once(['x', 'X'])
        .ok_or_else(|| "expected WIDTHxHEIGHT".to_string())?;
    let parse = |v: &str| {
        v.trim()
            .parse::<u32>()
            .ok()
            .filter(|&n| (1..=8192).contains(&n))
    };
    match (parse(width), parse(height)) {
        (Some(w), Some(h)) => Ok((w, h)),
        _ => Err("width and height must be between 1 and 8192".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_negative_positions_and_sizes() {
        let args = Args::parse_from([
            "strata",
            "--pos",
            "-1.5,64,2",
            "--look",
            "-90,-30",
            "--size",
            "640x360",
        ]);
        assert_eq!(args.pos, Some([-1.5, 64.0, 2.0]));
        assert_eq!(args.look, Some([-90.0, -30.0]));
        assert_eq!(args.size, (640, 360));
    }

    #[test]
    fn rejects_malformed_values() {
        assert!(Args::try_parse_from(["strata", "--pos", "1,2"]).is_err());
        assert!(Args::try_parse_from(["strata", "--size", "0x10"]).is_err());
    }
}
