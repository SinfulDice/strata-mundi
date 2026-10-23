use super::rect::Rect;
use strata_core::content::{Content, Face, ItemKind};
use strata_core::font::Font;
use strata_core::texture::{Rgba, RgbaImage, block_icon, upscale};

/// Twice `MAX_FONT_ATLAS`: the font (top) always fits, icons fill the rest.
const ATLAS_SIZE: u32 = 1024;
/// Icons are drawn twice the texture resolution (crisp at the 16-pixel slot size scaled up).
const MAX_ICON: u32 = 64;

/// One texture holding everything the interface draws: font, item icons, a white pixel.
pub struct UiAtlas {
    pub image: RgbaImage,
    pub font: Font,
    pub white: Rect,
    /// Icon of each item, by `ItemId` index.
    icons: Vec<Option<Rect>>,
}

impl UiAtlas {
    pub fn build(content: &Content, font: Font) -> Self {
        let mut image = RgbaImage::new(ATLAS_SIZE, ATLAS_SIZE, Rgba([0; 4]));
        image.draw(&font.atlas, 0, 0);
        let white_y = font.atlas.height();
        image.set(0, white_y, Rgba([255; 4]));
        let white = Rect::new(0, white_y as i32, 1, 1);
        let icon_size = (content.textures.resolution * 2).min(MAX_ICON);
        let mut shelf = (0u32, white_y + 2);
        let icons = content
            .items
            .iter()
            .map(|(_, _, item)| {
                let icon = item_icon(content, item.kind, icon_size)?;
                if shelf.0 + icon_size > ATLAS_SIZE {
                    shelf = (0, shelf.1 + icon_size);
                }
                if shelf.1 + icon_size > ATLAS_SIZE {
                    tracing::warn!("interface atlas full: some item icons are missing");
                    return None;
                }
                image.draw(&icon, shelf.0, shelf.1);
                let rect = Rect::new(shelf.0 as i32, shelf.1 as i32, icon_size, icon_size);
                shelf.0 += icon_size;
                Some(rect)
            })
            .collect();
        Self {
            image,
            font,
            white,
            icons,
        }
    }

    pub fn icon(&self, item: strata_core::content::ItemId) -> Option<Rect> {
        self.icons.get(usize::from(item.0)).copied().flatten()
    }

    /// Glyph rectangle in the atlas.
    pub fn glyph(&self, ch: char) -> (Rect, u32) {
        let glyph = self.font.glyph(ch);
        let rect = Rect::new(
            glyph.x as i32,
            glyph.y as i32,
            glyph.width,
            self.font.line_height,
        );
        (rect, glyph.advance)
    }
}

fn item_icon(content: &Content, kind: ItemKind, size: u32) -> Option<RgbaImage> {
    let image = |layer| content.textures.images.get(layer);
    match kind {
        ItemKind::Block(block) => {
            let block = content.blocks.get(block)?;
            let face = |f: Face| image(block.texture(f));
            Some(block_icon(
                face(Face::Up)?,
                face(Face::South)?,
                face(Face::East)?,
                size,
            ))
        }
        ItemKind::Flat(texture) => Some(upscale(image(texture)?, size)),
    }
}
