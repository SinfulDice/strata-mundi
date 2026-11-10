use super::*;

#[test]
fn changing_a_graphics_value_leaves_the_preset() {
    let mut settings = Settings::default();
    step_setting(Setting::Fov, &mut settings, 1);
    assert_eq!((settings.graphics.fov, settings.preset), (75, None));
    step_setting(Setting::Preset, &mut settings, 1);
    assert!(settings.preset.is_some());
    step_setting(Setting::RenderDistance, &mut settings, -100);
    assert_eq!(settings.graphics.render_distance, 2);
    step_setting(Setting::Preset, &mut settings, 1);
    step_setting(Setting::Brightness, &mut settings, 1);
    assert!(settings.preset.is_some(), "brightness keeps the preset");
    step_setting(Setting::Preset, &mut settings, 1);
    assert_eq!(
        settings.graphics.brightness, 60,
        "and every preset keeps it"
    );
}

#[test]
fn clouds_cycle_through_their_modes() {
    let mut settings = Settings::default();
    settings.apply_preset(crate::settings::Preset::Ds);
    assert_eq!(settings.graphics.clouds, CloudMode::Flat);
    step_setting(Setting::Clouds, &mut settings, 1);
    assert_eq!(
        (settings.graphics.clouds, settings.preset),
        (CloudMode::Full, None)
    );
    step_setting(Setting::Clouds, &mut settings, 1);
    assert_eq!(settings.graphics.clouds, CloudMode::Off);
    step_setting(Setting::Clouds, &mut settings, -1);
    assert_eq!(settings.graphics.clouds, CloudMode::Full);
}

#[test]
fn particles_cycle_off_reduced_full() {
    let mut settings = Settings::default();
    settings.apply_preset(crate::settings::Preset::Ds);
    assert_eq!(settings.graphics.particles, ParticleMode::Reduced);
    step_setting(Setting::Particles, &mut settings, 1);
    assert_eq!(
        (settings.graphics.particles, settings.preset),
        (ParticleMode::Full, None)
    );
    step_setting(Setting::Particles, &mut settings, 1);
    assert_eq!(settings.graphics.particles, ParticleMode::Off);
}

#[test]
fn volumes_step_by_ten_and_keep_the_preset() {
    let mut settings = Settings::default();
    step_setting(Setting::MasterVolume, &mut settings, -1);
    step_setting(Setting::MusicVolume, &mut settings, 5);
    step_setting(Setting::Subtitles, &mut settings, 1);
    let sound = &settings.sound;
    assert_eq!(
        (sound.master, sound.music, sound.subtitles),
        (90, 100, true)
    );
    assert!(
        settings.preset.is_some(),
        "volumes are not part of the presets"
    );
    settings.apply_preset(Preset::Ds);
    assert_eq!(settings.sound.master, 90, "and no preset changes them");
    assert_eq!(
        settings.sound.spatial,
        SpatialAudio::Simple,
        "but the DS is plain stereo"
    );
    step_setting(Setting::SpatialAudio, &mut settings, 1);
    assert_eq!(
        (settings.sound.spatial, settings.preset),
        (SpatialAudio::Advanced, None)
    );
}
