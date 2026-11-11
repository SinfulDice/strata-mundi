//! What the menus take from the system: the clipboard, and pictures copied into the
//! interface atlas (world captures, the map of the generation editor).

use super::Screens;
use super::menu::Menu;
use crate::render::Gpu;
use crate::ui::{ICON_SIZE, Rect, UiAtlas};
use strata_core::texture::RgbaImage;

impl Screens {
    pub(super) fn upload_icon(&self, gpu: &Gpu, slot: usize, png: Option<&[u8]>) -> Option<Rect> {
        let place = UiAtlas::world_icon_slot(slot)?;
        let image = decode_icon(png?)?;
        if (image.width(), image.height()) != (place.w, place.h) {
            return None;
        }
        self.pipeline
            .write_atlas(gpu, place.x as u32, place.y as u32, &image);
        Some(place)
    }

    /// Copies the newest map of the generation editor into the atlas.
    pub(super) fn upload_preview(&mut self, gpu: &Gpu) {
        let Some(Menu::CreateWorld(form)) = self.stack.last_mut() else {
            return;
        };
        let quality = self.settings.graphics.map_preview;
        let Some(image) = form.refresh_preview(quality, self.generation.maps.as_ref()) else {
            return;
        };
        if self.preview_uploaded == Some(image.request) {
            return;
        }
        let Some(place) = UiAtlas::preview_slot(image.size) else {
            return;
        };
        let pixels = image.pixels.iter().flat_map(|&[r, g, b]| [r, g, b, 255]);
        if let Some(rgba) = RgbaImage::from_bytes(place.w, place.h, pixels.collect()) {
            self.pipeline
                .write_atlas(gpu, place.x as u32, place.y as u32, &rgba);
            self.preview_uploaded = Some(image.request);
        }
    }

    /// Whether the text reached the clipboard.
    pub(super) fn copy_text(&mut self, text: String) -> bool {
        match self.clipboard().map(|c| c.set_text(text)) {
            Some(Ok(())) => true,
            Some(Err(error)) => {
                tracing::warn!(%error, "nothing copied");
                false
            }
            None => false,
        }
    }

    pub(super) fn paste_text(&mut self) -> Option<String> {
        match self.clipboard()?.get_text() {
            Ok(text) => Some(text),
            Err(error) => {
                tracing::warn!(%error, "nothing to paste");
                None
            }
        }
    }

    fn clipboard(&mut self) -> Option<&mut arboard::Clipboard> {
        if self.clipboard.is_none() {
            match arboard::Clipboard::new() {
                Ok(clipboard) => self.clipboard = Some(clipboard),
                Err(error) => tracing::warn!(%error, "no clipboard"),
            }
        }
        self.clipboard.as_mut()
    }
}

fn decode_icon(png_bytes: &[u8]) -> Option<RgbaImage> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().ok()?;
    let info = reader.info();
    if (info.width, info.height) != ICON_SIZE {
        return None;
    }
    let mut buffer = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buffer).ok()?;
    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return None;
    }
    buffer.truncate(info.buffer_size());
    RgbaImage::from_bytes(info.width, info.height, buffer)
}
