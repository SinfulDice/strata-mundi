//! Resources read again while the game runs (textures, translations, interface files,
//! shaders): edit a pack, press the key, see the result. Blocks and items cannot change
//! this way, since the server and the open world use them.

use super::Session;
use crate::app::ClientError;
use crate::mesher::BlockTable;
use crate::render::{PostProcess, RenderOptions, Renderer, TextureFiltering};
use std::sync::Arc;
use strata_core::content::Content;
use strata_core::id::Identifier;

impl Session {
    pub(super) fn reload_resources(&mut self) -> Result<(), ClientError> {
        let enabled = &self.applied.game.resource_packs;
        let Some((content, ui)) = self.launcher.reload_resources(enabled) else {
            self.reload_failed(&"unusable packs");
            return Ok(());
        };
        if !same_registries(&self.content, &content) {
            self.screens.show_notice("ui-reload-blocks-changed");
            return Ok(());
        }
        let graphics = &self.applied.graphics;
        let filtering = TextureFiltering {
            mipmaps: graphics.mipmaps,
            anisotropy: graphics.anisotropy.clamp(1, 16) as u16,
        };
        let format = self.surface.format();
        let scope = self
            .gpu
            .device
            .push_error_scope(wgpu::ErrorFilter::Validation);
        let renderer = Renderer::new(&self.gpu, format, &content.textures, filtering);
        let post = PostProcess::new(&self.gpu, format);
        let gpu_error = pollster::block_on(scope.pop());
        let mut renderer = match (renderer, gpu_error) {
            (Ok(renderer), None) => renderer,
            (Err(error), _) => {
                self.reload_failed(&error);
                return Ok(());
            }
            (_, Some(error)) => {
                self.reload_failed(&error);
                return Ok(());
            }
        };
        renderer.ambient_occlusion = f32::from(u8::from(graphics.ambient_occlusion));
        renderer.vertical_distance = graphics.vertical_distance;
        renderer.options = RenderOptions::from_settings(&self.applied);
        self.renderer = renderer;
        self.post = post;
        self.table = Arc::new(BlockTable::new(content.blocks.iter().map(|(_, _, b)| b)));
        self.content = Arc::new(content);
        self.screens
            .replace_assets(&self.gpu, format, ui, &self.content);
        self.screens.available_packs = self.launcher.resource_packs();
        self.remesh()?;
        tracing::info!("resources reloaded");
        Ok(())
    }
}

impl Session {
    fn reload_failed(&mut self, error: &dyn std::fmt::Display) {
        tracing::warn!(%error, "resources not reloaded");
        self.screens.show_notice("ui-reload-failed");
    }
}

fn same_registries(old: &Content, new: &Content) -> bool {
    fn names<'a>(ids: impl Iterator<Item = &'a Identifier>) -> Vec<&'a Identifier> {
        ids.collect()
    }
    names(old.blocks.iter().map(|(_, id, _)| id)) == names(new.blocks.iter().map(|(_, id, _)| id))
        && names(old.items.iter().map(|(_, id, _)| id))
            == names(new.items.iter().map(|(_, id, _)| id))
}
