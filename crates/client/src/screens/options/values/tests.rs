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

#[test]
fn music_frequency_cycles_and_is_not_a_preset_setting() {
    let mut settings = Settings::default();
    step_setting(Setting::MusicFrequency, &mut settings, 1);
    assert_eq!(settings.sound.music_frequency, MusicFrequency::Often);
    step_setting(Setting::MusicFrequency, &mut settings, 1);
    assert_eq!(settings.sound.music_frequency, MusicFrequency::Rare);
    assert!(settings.preset.is_some());
    settings.apply_preset(Preset::Ds);
    assert_eq!(settings.sound.music_frequency, MusicFrequency::Rare);
}

#[test]
fn a_click_on_a_gauge_sets_the_nearest_step_bounds_included() {
    let mut settings = Settings::default();
    set_setting_fraction(Setting::MasterVolume, &mut settings, 0.0);
    assert_eq!(settings.sound.master, 0);
    set_setting_fraction(Setting::MasterVolume, &mut settings, 1.0);
    assert_eq!(settings.sound.master, 100);
    set_setting_fraction(Setting::MasterVolume, &mut settings, 0.44);
    assert_eq!(settings.sound.master, 40);
    set_setting_fraction(Setting::MasterVolume, &mut settings, 0.46);
    assert_eq!(settings.sound.master, 50);
    set_setting_fraction(Setting::Fov, &mut settings, 1.0);
    assert_eq!(settings.graphics.fov, 110);
    set_setting_fraction(Setting::Fov, &mut settings, 0.0);
    assert_eq!(settings.graphics.fov, 50);
    set_setting_fraction(Setting::Fov, &mut settings, 0.5);
    assert_eq!(settings.graphics.fov, 80);
    set_setting_fraction(Setting::DeadZone, &mut settings, 7.0);
    assert_eq!(settings.controls.dead_zone, 50, "past the end clamps");
}

#[test]
fn a_gauge_set_by_the_pointer_leaves_the_preset_like_a_step() {
    let mut settings = Settings::default();
    set_setting_fraction(Setting::RenderDistance, &mut settings, 0.0);
    assert_eq!(
        (settings.graphics.render_distance, settings.preset),
        (2, None)
    );
    step_setting(Setting::Preset, &mut settings, 1);
    set_setting_fraction(Setting::Brightness, &mut settings, 1.0);
    assert!(settings.preset.is_some(), "brightness keeps the preset");
    let before = settings.graphics.vsync;
    set_setting_fraction(Setting::Vsync, &mut settings, 1.0);
    assert_eq!(settings.graphics.vsync, before, "not a range: unchanged");
}

#[test]
fn dragging_along_a_gauge_follows_the_pointer() {
    let mut settings = Settings::default();
    let mut seen = Vec::new();
    for tenth in 0..=10 {
        set_setting_fraction(Setting::MusicVolume, &mut settings, tenth as f32 / 10.0);
        seen.push(settings.sound.music);
    }
    assert_eq!(seen, (0..=10).map(|t| t * 10).collect::<Vec<_>>());
    for tenth in (0..=10).rev() {
        set_setting_fraction(Setting::RenderDistance, &mut settings, tenth as f32 / 10.0);
        assert_eq!(settings.graphics.render_distance, 2 + 3 * tenth);
    }
}
