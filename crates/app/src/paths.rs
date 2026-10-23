use std::path::PathBuf;

/// Finds the `assets/` folder: next to the executable (installed game), in the working
/// directory, or, in debug builds only, at the workspace root (`cargo run` from anywhere).
pub fn assets_dir() -> Option<PathBuf> {
    let beside_exe = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join("assets")));
    [
        beside_exe,
        Some(PathBuf::from("assets")),
        workspace_assets(),
    ]
    .into_iter()
    .flatten()
    .find(|dir| dir.is_dir())
}

#[cfg(debug_assertions)]
fn workspace_assets() -> Option<PathBuf> {
    Some(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"))
}

#[cfg(not(debug_assertions))]
fn workspace_assets() -> Option<PathBuf> {
    None
}
