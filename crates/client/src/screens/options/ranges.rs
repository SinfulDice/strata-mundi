//! Settings that are a number between two bounds, moved by steps or set from a gauge.

use super::values::{BACKUPS, Setting};
use crate::settings::{
    BIOME_BLEND, BRIGHTNESS, DEAD_ZONE, FOV, MAX_SCALE, RENDER_DISTANCE, RENDER_SCALE, SENSITIVITY,
    Settings, VERTICAL_DISTANCE, VOLUME,
};

/// A range setting's value, its bounds and the size of one step.
pub struct RangeSlot<'a> {
    value: &'a mut u32,
    bounds: (u32, u32),
    unit: u32,
}

impl RangeSlot<'_> {
    pub fn step(self, step: i32) {
        let next = i64::from(*self.value) + i64::from(step) * i64::from(self.unit);
        *self.value = next.clamp(i64::from(self.bounds.0), i64::from(self.bounds.1)) as u32;
    }

    /// The step nearest `fraction` of the way between the bounds.
    pub fn set_fraction(self, fraction: f32) {
        let (min, max) = self.bounds;
        let target = fraction.clamp(0.0, 1.0) * (max - min) as f32;
        let steps = (target / self.unit.max(1) as f32).round() as u32;
        *self.value = (min + steps * self.unit).min(max);
    }
}

pub fn range_slot(setting: Setting, s: &mut Settings) -> Option<RangeSlot<'_>> {
    let (g, sound, c) = (&mut s.graphics, &mut s.sound, &mut s.controls);
    let (value, bounds, unit) = match setting {
        Setting::RenderDistance => (&mut g.render_distance, RENDER_DISTANCE, 1),
        Setting::VerticalDistance => (&mut g.vertical_distance, VERTICAL_DISTANCE, 1),
        Setting::RenderScale => (&mut g.render_scale, RENDER_SCALE, 10),
        Setting::Fov => (&mut g.fov, FOV, 5),
        Setting::Brightness => (&mut g.brightness, BRIGHTNESS, 10),
        Setting::BiomeBlend => (&mut g.biome_blend, BIOME_BLEND, 1),
        Setting::MasterVolume => (&mut sound.master, VOLUME, 10),
        Setting::BlocksVolume => (&mut sound.blocks, VOLUME, 10),
        Setting::AmbientVolume => (&mut sound.ambient, VOLUME, 10),
        Setting::MusicVolume => (&mut sound.music, VOLUME, 10),
        Setting::Scale => (&mut s.interface.scale, (0, MAX_SCALE), 1),
        Setting::MouseSensitivity => (&mut c.mouse_sensitivity, SENSITIVITY, 10),
        Setting::StickSensitivity => (&mut c.stick_sensitivity, SENSITIVITY, 10),
        Setting::DeadZone => (&mut c.dead_zone, DEAD_ZONE, 5),
        Setting::Backups => (&mut s.game.backups_kept, BACKUPS, 1),
        _ => return None,
    };
    Some(RangeSlot {
        value,
        bounds,
        unit,
    })
}
