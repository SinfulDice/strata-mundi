//! Which settings each tab of the options screen lists, in order.

use super::values::Setting;

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
    Setting::MaxTextureSize,
    Setting::Anisotropy,
    Setting::Vegetation,
    Setting::Particles,
    Setting::ItemDistance,
    Setting::MapPreview,
    Setting::MeshThreads,
    Setting::WorldThreads,
];
pub const SOUND: &[Setting] = &[
    Setting::MasterVolume,
    Setting::BlocksVolume,
    Setting::AmbientVolume,
    Setting::MusicVolume,
    Setting::MusicFrequency,
    Setting::Subtitles,
    Setting::SpatialAudio,
];
pub const INTERFACE: &[Setting] = &[
    Setting::Language,
    Setting::Scale,
    Setting::Font,
    Setting::ColorFilter,
    Setting::ReduceCamera,
    Setting::HurtShake,
    Setting::HurtFlash,
    Setting::ShowMana,
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
