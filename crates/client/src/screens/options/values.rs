//! Every setting of the options screen: how its value reads and how a step changes it.

use super::super::menu::Shared;
use crate::settings::MAX_SCALE;
use crate::settings::{
    ANISOTROPY, BIOME_BLEND, BRIGHTNESS, DEAD_ZONE, FOV, RENDER_DISTANCE, RENDER_SCALE,
    SENSITIVITY, VERTICAL_DISTANCE, VOLUME,
};
use crate::settings::{
    AUTO_THREADS, CloudMode, ColorFilter, FontChoice, Lighting, ParticleMode, Preset, Settings,
    Vegetation, default_worker_threads,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Preset,
    RenderDistance,
    VerticalDistance,
    RenderScale,
    FpsLimit,
    Vsync,
    Fov,
    AmbientOcclusion,
    Lighting,
    SmoothLighting,
    Brightness,
    Fog,
    AnimatedTextures,
    TextureVariants,
    BiomeBlend,
    Stars,
    Clouds,
    Mipmaps,
    Anisotropy,
    Vegetation,
    Particles,
    MeshThreads,
    WorldThreads,
    MasterVolume,
    BlocksVolume,
    AmbientVolume,
    MusicVolume,
    Subtitles,
    Language,
    Scale,
    Font,
    ColorFilter,
    ReduceCamera,
    PauseInMenus,
    BlockName,
    EditHud,
    MouseSensitivity,
    StickSensitivity,
    InvertY,
    DeadZone,
    AutoJump,
    StepUp,
    ResetKeys,
    CheckUpdates,
    Autosave,
    Backups,
    IdleUnfocused,
}

pub const GRAPHICS: &[Setting] = &[
    Setting::Preset,
    Setting::RenderDistance,
    Setting::VerticalDistance,
    Setting::RenderScale,
    Setting::FpsLimit,
    Setting::Vsync,
    Setting::Fov,
    Setting::AmbientOcclusion,
    Setting::Lighting,
    Setting::SmoothLighting,
    Setting::Brightness,
    Setting::Fog,
    Setting::AnimatedTextures,
    Setting::TextureVariants,
    Setting::BiomeBlend,
    Setting::Stars,
    Setting::Clouds,
    Setting::Mipmaps,
    Setting::Anisotropy,
    Setting::Vegetation,
    Setting::Particles,
    Setting::MeshThreads,
    Setting::WorldThreads,
];
pub const SOUND: &[Setting] = &[
    Setting::MasterVolume,
    Setting::BlocksVolume,
    Setting::AmbientVolume,
    Setting::MusicVolume,
    Setting::Subtitles,
];
pub const INTERFACE: &[Setting] = &[
    Setting::Language,
    Setting::Scale,
    Setting::Font,
    Setting::ColorFilter,
    Setting::ReduceCamera,
    Setting::PauseInMenus,
    Setting::BlockName,
    Setting::EditHud,
];
pub const CONTROLS: &[Setting] = &[
    Setting::MouseSensitivity,
    Setting::StickSensitivity,
    Setting::InvertY,
    Setting::DeadZone,
    Setting::AutoJump,
    Setting::StepUp,
];
pub const GAME: &[Setting] = &[
    Setting::CheckUpdates,
    Setting::Autosave,
    Setting::Backups,
    Setting::IdleUnfocused,
];

const FPS_LIMITS: [u32; 6] = [0, 30, 60, 120, 144, 240];
const BACKUPS: (u32, u32) = (0, 10);
const AUTOSAVE: [u32; 7] = [0, 1, 2, 5, 10, 15, 30];

pub fn setting_key(setting: Setting) -> String {
    kebab_key("option", &format!("{setting:?}"))
}

/// `("action", "Hotbar1")` → `action-hotbar-1`.
pub fn kebab_key(prefix: &str, name: &str) -> String {
    let mut key = String::from(prefix);
    for ch in name.chars() {
        if ch.is_uppercase() || ch.is_ascii_digit() {
            key.push('-');
        }
        key.extend(ch.to_lowercase());
    }
    key
}

fn on_off(shared: &Shared<'_>, value: bool) -> String {
    shared.tr(if value { "ui-on" } else { "ui-off" })
}

fn threads_label(shared: &Shared<'_>, threads: usize) -> String {
    if threads == AUTO_THREADS {
        shared.tr("option-auto")
    } else {
        threads.to_string()
    }
}

/// Text of the value and, for ranges, the gauge position.
pub fn value(setting: Setting, s: &Settings, shared: &Shared<'_>) -> (String, Option<f32>) {
    let (g, i, c, game) = (&s.graphics, &s.interface, &s.controls, &s.game);
    let gauge =
        |v: u32, (min, max): (u32, u32)| Some(v.saturating_sub(min) as f32 / (max - min) as f32);
    match setting {
        Setting::Preset => (
            shared.tr(&s
                .preset
                .map_or("preset-custom".to_owned(), |p| format!("preset-{}", p.id()))),
            None,
        ),
        Setting::RenderDistance => (
            g.render_distance.to_string(),
            gauge(g.render_distance, RENDER_DISTANCE),
        ),
        Setting::VerticalDistance => (
            g.vertical_distance.to_string(),
            gauge(g.vertical_distance, VERTICAL_DISTANCE),
        ),
        Setting::RenderScale => (
            format!("{} %", g.render_scale),
            gauge(g.render_scale, RENDER_SCALE),
        ),
        Setting::FpsLimit if g.fps_limit == 0 => (shared.tr("option-unlimited"), None),
        Setting::FpsLimit => (g.fps_limit.to_string(), None),
        Setting::Vsync => (on_off(shared, g.vsync), None),
        Setting::Fov => (format!("{}°", g.fov), gauge(g.fov, FOV)),
        Setting::AmbientOcclusion => (on_off(shared, g.ambient_occlusion), None),
        Setting::Lighting => (shared.tr(&format!("lighting-{}", g.lighting.id())), None),
        Setting::SmoothLighting => (on_off(shared, g.smooth_lighting), None),
        Setting::Brightness => (
            format!("{} %", g.brightness),
            gauge(g.brightness, BRIGHTNESS),
        ),
        Setting::Fog => (on_off(shared, g.fog), None),
        Setting::AnimatedTextures => (on_off(shared, g.animated_textures), None),
        Setting::TextureVariants => (on_off(shared, g.texture_variants), None),
        Setting::BiomeBlend if g.biome_blend == 0 => (shared.tr("ui-off"), None),
        Setting::BiomeBlend => (g.biome_blend.to_string(), gauge(g.biome_blend, BIOME_BLEND)),
        Setting::Stars => (on_off(shared, g.stars), None),
        Setting::Clouds => (shared.tr(&format!("clouds-{}", g.clouds.id())), None),
        Setting::Mipmaps => (on_off(shared, g.mipmaps), None),
        Setting::Anisotropy if g.anisotropy <= 1 => (shared.tr("ui-off"), None),
        Setting::Anisotropy => (format!("{}x", g.anisotropy), None),
        Setting::Vegetation => (
            shared.tr(&format!("vegetation-{}", g.vegetation.id())),
            None,
        ),
        Setting::Particles => (shared.tr(&format!("particles-{}", g.particles.id())), None),
        Setting::MeshThreads => (threads_label(shared, g.worker_threads), None),
        Setting::WorldThreads => (threads_label(shared, g.generation_threads), None),
        Setting::MasterVolume
        | Setting::BlocksVolume
        | Setting::AmbientVolume
        | Setting::MusicVolume => {
            let sound = &s.sound;
            let volume = match setting {
                Setting::BlocksVolume => sound.blocks,
                Setting::AmbientVolume => sound.ambient,
                Setting::MusicVolume => sound.music,
                _ => sound.master,
            };
            (format!("{volume} %"), gauge(volume, VOLUME))
        }
        Setting::Subtitles => (on_off(shared, s.sound.subtitles), None),
        Setting::Language => (shared.tr("language-name"), None),
        Setting::Scale if i.scale == 0 => (shared.tr("ui-scale-auto"), None),
        Setting::Scale => (
            shared.tr_with("ui-scale-times", &[("scale", i.scale.to_string())]),
            None,
        ),
        Setting::Font => (
            shared.tr(match i.font {
                FontChoice::Pixel => "font-pixel",
                FontChoice::Readable => "font-readable",
            }),
            None,
        ),
        Setting::ColorFilter => (
            shared.tr(match i.color_filter {
                ColorFilter::None => "filter-none",
                ColorFilter::Protanopia => "filter-protanopia",
                ColorFilter::Deuteranopia => "filter-deuteranopia",
                ColorFilter::Tritanopia => "filter-tritanopia",
            }),
            None,
        ),
        Setting::ReduceCamera => (on_off(shared, i.reduce_camera_effects), None),
        Setting::PauseInMenus => (on_off(shared, i.pause_in_menus), None),
        Setting::BlockName => (on_off(shared, c.show_block_name), None),
        Setting::EditHud | Setting::ResetKeys => (String::new(), None),
        Setting::MouseSensitivity => (
            format!("{} %", c.mouse_sensitivity),
            gauge(c.mouse_sensitivity, SENSITIVITY),
        ),
        Setting::StickSensitivity => (
            format!("{} %", c.stick_sensitivity),
            gauge(c.stick_sensitivity, SENSITIVITY),
        ),
        Setting::InvertY => (on_off(shared, c.invert_y), None),
        Setting::DeadZone => (format!("{} %", c.dead_zone), gauge(c.dead_zone, DEAD_ZONE)),
        Setting::AutoJump => (on_off(shared, c.auto_jump), None),
        Setting::StepUp => (on_off(shared, c.step_up), None),
        Setting::CheckUpdates => (on_off(shared, game.check_updates), None),
        Setting::Autosave if game.autosave_minutes == 0 => (shared.tr("ui-off"), None),
        Setting::Autosave => (
            shared.tr_with(
                "ui-play-time-minutes",
                &[("minutes", game.autosave_minutes.to_string())],
            ),
            None,
        ),
        Setting::Backups => (
            game.backups_kept.to_string(),
            gauge(game.backups_kept, BACKUPS),
        ),
        Setting::IdleUnfocused => (on_off(shared, game.idle_when_unfocused), None),
    }
}

pub fn step_setting(setting: Setting, s: &mut Settings, step: i32) {
    // Brightness is a taste, kept by every preset.
    let graphics_changed =
        !matches!(setting, Setting::Preset | Setting::Brightness) && GRAPHICS.contains(&setting);
    let cores = default_worker_threads() + 2;
    let (g, i, c, game) = (
        &mut s.graphics,
        &mut s.interface,
        &mut s.controls,
        &mut s.game,
    );
    match setting {
        Setting::Preset => {
            let next = cycle(&Preset::ALL, s.preset.unwrap_or(Preset::High), step);
            s.apply_preset(next);
        }
        Setting::RenderDistance => range(&mut g.render_distance, RENDER_DISTANCE, 1, step),
        Setting::VerticalDistance => range(&mut g.vertical_distance, VERTICAL_DISTANCE, 1, step),
        Setting::RenderScale => range(&mut g.render_scale, RENDER_SCALE, 10, step),
        Setting::FpsLimit => g.fps_limit = cycle(&FPS_LIMITS, g.fps_limit, step),
        Setting::Vsync => g.vsync = !g.vsync,
        Setting::Fov => range(&mut g.fov, FOV, 5, step),
        Setting::AmbientOcclusion => g.ambient_occlusion = !g.ambient_occlusion,
        Setting::Lighting => g.lighting = cycle(&Lighting::ALL, g.lighting, step),
        Setting::SmoothLighting => g.smooth_lighting = !g.smooth_lighting,
        Setting::Brightness => range(&mut g.brightness, BRIGHTNESS, 10, step),
        Setting::Fog => g.fog = !g.fog,
        Setting::AnimatedTextures => g.animated_textures = !g.animated_textures,
        Setting::TextureVariants => g.texture_variants = !g.texture_variants,
        Setting::BiomeBlend => range(&mut g.biome_blend, BIOME_BLEND, 1, step),
        Setting::Stars => g.stars = !g.stars,
        Setting::Clouds => g.clouds = cycle(&CloudMode::ALL, g.clouds, step),
        Setting::Mipmaps => g.mipmaps = !g.mipmaps,
        Setting::Anisotropy => g.anisotropy = cycle(&ANISOTROPY, g.anisotropy, step),
        Setting::Vegetation => g.vegetation = cycle(&Vegetation::ALL, g.vegetation, step),
        Setting::Particles => g.particles = cycle(&ParticleMode::ALL, g.particles, step),
        Setting::MeshThreads => g.worker_threads = cycle_threads(g.worker_threads, cores, step),
        Setting::WorldThreads => {
            g.generation_threads = cycle_threads(g.generation_threads, cores, step)
        }
        Setting::MasterVolume
        | Setting::BlocksVolume
        | Setting::AmbientVolume
        | Setting::MusicVolume => range(volume_of(setting, s), VOLUME, 10, step),
        Setting::Subtitles => s.sound.subtitles = !s.sound.subtitles,
        Setting::Language => i.cycle_language(),
        Setting::Scale => range(&mut i.scale, (0, MAX_SCALE), 1, step),
        Setting::Font => {
            i.font = match i.font {
                FontChoice::Pixel => FontChoice::Readable,
                FontChoice::Readable => FontChoice::Pixel,
            }
        }
        Setting::ColorFilter => i.color_filter = cycle(&ColorFilter::ALL, i.color_filter, step),
        Setting::ReduceCamera => i.reduce_camera_effects = !i.reduce_camera_effects,
        Setting::PauseInMenus => i.pause_in_menus = !i.pause_in_menus,
        Setting::BlockName => c.show_block_name = !c.show_block_name,
        Setting::MouseSensitivity => range(&mut c.mouse_sensitivity, SENSITIVITY, 10, step),
        Setting::StickSensitivity => range(&mut c.stick_sensitivity, SENSITIVITY, 10, step),
        Setting::InvertY => c.invert_y = !c.invert_y,
        Setting::DeadZone => range(&mut c.dead_zone, DEAD_ZONE, 5, step),
        Setting::AutoJump => c.auto_jump = !c.auto_jump,
        Setting::StepUp => c.step_up = !c.step_up,
        Setting::CheckUpdates => game.check_updates = !game.check_updates,
        Setting::Autosave => game.autosave_minutes = cycle(&AUTOSAVE, game.autosave_minutes, step),
        Setting::Backups => range(&mut game.backups_kept, BACKUPS, 1, step),
        Setting::IdleUnfocused => game.idle_when_unfocused = !game.idle_when_unfocused,
        Setting::EditHud | Setting::ResetKeys => {}
    }
    if graphics_changed {
        s.preset = None;
    }
}

/// The volume a slider row stands for (the master for any other row).
fn volume_of(setting: Setting, s: &mut Settings) -> &mut u32 {
    let sound = &mut s.sound;
    match setting {
        Setting::BlocksVolume => &mut sound.blocks,
        Setting::AmbientVolume => &mut sound.ambient,
        Setting::MusicVolume => &mut sound.music,
        _ => &mut sound.master,
    }
}

fn range(value: &mut u32, (min, max): (u32, u32), unit: u32, step: i32) {
    let next = *value as i64 + i64::from(step) * i64::from(unit);
    *value = next.clamp(i64::from(min), i64::from(max)) as u32;
}

/// The next (or previous) value of `choices`, wrapping; an unknown value restarts at the first.
fn cycle<T: Copy + PartialEq>(choices: &[T], current: T, step: i32) -> T {
    let len = choices.len() as i32;
    let index = choices
        .iter()
        .position(|c| *c == current)
        .map_or(0, |i| i as i32);
    choices[(index + step.signum()).rem_euclid(len) as usize]
}

/// Automatic, then 1 to the number of cores.
fn cycle_threads(current: usize, cores: usize, step: i32) -> usize {
    let choices: Vec<usize> = std::iter::once(AUTO_THREADS).chain(1..=cores).collect();
    cycle(&choices, current, step)
}

#[cfg(test)]
mod tests;
