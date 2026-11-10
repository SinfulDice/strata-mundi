//! Settings take effect as soon as they change: the edited copy (owned by the screens) is
//! compared with the one in use, and only what differs is redone.

use super::Session;
use crate::mesher::MeshOptions;
use crate::render::RenderOptions;
use crate::settings::Settings;
use crate::stage::Stage;
use crate::world::requested_view_distance;
use strata_protocol::ClientMessage;

impl Session {
    pub(super) fn apply_settings(&mut self) {
        if self.screens.settings == self.applied {
            return;
        }
        let new = self.screens.settings.clone();
        let old = std::mem::replace(&mut self.applied, new);
        self.apply_graphics(&old);
        self.apply_controls(&old);
        if self.applied.sound != old.sound {
            self.audio.apply_settings(&self.applied.sound);
        }
        self.launcher.configure(&self.applied);
        if self.applied.game.resource_packs != old.game.resource_packs {
            self.apply_resource_packs(&old);
        }
    }

    /// Refused or failed: the previous packs come back, in use and in the file.
    fn apply_resource_packs(&mut self, old: &Settings) {
        let applied = self.reload_resources().unwrap_or_else(|error| {
            tracing::error!(%error, "resource packs not applied");
            false
        });
        if !applied {
            let previous = old.game.resource_packs.clone();
            self.applied.game.resource_packs.clone_from(&previous);
            self.screens
                .change_settings(|settings| settings.game.resource_packs = previous);
        }
    }

    fn apply_graphics(&mut self, old: &Settings) {
        let g = self.applied.graphics.clone();
        let before = &old.graphics;
        if g.render_distance != before.render_distance
            && let Stage::Playing(world) = &self.stage
        {
            let distance = requested_view_distance(g.render_distance);
            world.connection.send(ClientMessage::ViewDistance(distance));
        }
        if g.vsync != before.vsync {
            self.surface.set_vsync(g.vsync);
        }
        self.renderer.vertical_distance = g.vertical_distance;
        self.renderer.ambient_occlusion = f32::from(u8::from(g.ambient_occlusion));
        self.renderer.options = RenderOptions::from_settings(&self.applied);
        if (g.mipmaps, g.anisotropy) != (before.mipmaps, before.anisotropy) {
            let textures = &self.content.textures;
            self.renderer
                .set_texture_filtering(&self.gpu, textures, g.mipmaps, g.anisotropy);
        }
        if g.lighting != before.lighting {
            self.set_lighting(g.lighting);
        }
        if g.particles != before.particles
            && let Some(particles) = self.particles_mut()
        {
            particles.set_mode(g.particles);
        }
        let remesh = (MeshOptions::from_graphics(&g), g.meshing_threads())
            != (MeshOptions::from_graphics(before), before.meshing_threads());
        if remesh && let Err(error) = self.remesh() {
            tracing::error!(%error, "could not restart meshing");
        }
        let fov = g.fov as f32;
        match &mut self.stage {
            Stage::Playing(world) => world.game.camera.fov_y_degrees = fov,
            Stage::Title(title) => title.camera.fov_y_degrees = fov,
            Stage::Empty => {}
        }
    }

    fn apply_controls(&mut self, old: &Settings) {
        let controls = &self.applied.controls;
        self.input
            .configure(&controls.bindings, controls.dead_zone_fraction());
        self.idle
            .configure(&controls.bindings, controls.dead_zone_fraction());
        if let Stage::Playing(world) = &mut self.stage {
            world.game.controls = controls.clone();
            world.game.camera_effects = !self.applied.interface.reduce_camera_effects;
            if (controls.auto_jump, controls.step_up)
                != (old.controls.auto_jump, old.controls.step_up)
            {
                world.connection.send(ClientMessage::MovementPreferences {
                    auto_jump: controls.auto_jump,
                    step_up: controls.step_up,
                });
            }
        }
    }
}
