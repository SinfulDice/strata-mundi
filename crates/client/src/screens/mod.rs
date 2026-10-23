//! The screens drawn over the world: the HUD always, the creative inventory on demand.

mod hud;
mod inventory;

use crate::gameplay::Gameplay;
use crate::input::{Action, InputState};
use crate::render::{Gpu, UiPipeline};
use crate::ui::ScreenModel;
use crate::ui::{
    Direction, Hit, Painter, Rect, UiAssets, UiAtlas, UiContext, auto_scale, draw_screen, navigate,
};
use hud::HudModel;
use inventory::{InventoryModel, InventoryState};
use std::sync::Arc;
use strata_core::content::{Content, ItemKind};

/// Largest interface scale the automatic setting may pick.
const MAX_AUTO_SCALE: u32 = 4;

pub struct Screens {
    assets: UiAssets,
    atlas: UiAtlas,
    pipeline: UiPipeline,
    language: String,
    inventory: Option<InventoryState>,
    focus: Option<Hit>,
    hits: Vec<Hit>,
    scale: u32,
}

impl Screens {
    pub fn new(
        gpu: &Gpu,
        color_format: wgpu::TextureFormat,
        assets: UiAssets,
        content: &Content,
        language: String,
    ) -> Self {
        let atlas = UiAtlas::build(content, assets.font.clone());
        let pipeline = UiPipeline::new(gpu, color_format, &atlas.image);
        Self {
            assets,
            atlas,
            pipeline,
            language,
            inventory: None,
            focus: None,
            hits: Vec::new(),
            scale: 1,
        }
    }

    /// A menu is open: the cursor is free and the player stands still.
    pub fn is_menu_open(&self) -> bool {
        self.inventory.is_some()
    }

    /// Opens or closes menus; returns whether the input was used by a menu.
    pub fn handle(&mut self, input: &InputState, game: &mut Gameplay) -> bool {
        let Some(state) = self.inventory.as_ref() else {
            if input.just_pressed(Action::Inventory) {
                self.inventory = Some(InventoryState::default());
                self.focus = None;
            }
            return self.inventory.is_some();
        };
        if state.search_focused {
            self.edit_search(input, game);
            return true;
        }
        if input.just_pressed(Action::Inventory) || input.just_pressed(Action::MenuBack) {
            self.inventory = None;
            self.focus = None;
            self.hits.clear();
            return true;
        }
        self.follow_mouse(input);
        self.navigate(input);
        let focus = self.focus.clone();
        let content = Arc::clone(&game.info.content);
        let Some(state) = self.inventory.as_mut() else {
            return true;
        };
        let mut model = InventoryModel::new(state, &mut game.hotbar, &content, &self.language);
        model.scroll(-input.scroll_steps());
        let tabs = i32::from(input.just_pressed(Action::HotbarNext))
            - i32::from(input.just_pressed(Action::HotbarPrevious));
        if tabs != 0 {
            let tab = (model.state.tab as i32 + tabs).rem_euclid(model.tab_count() as i32) as usize;
            model.activate("tabs", tab);
        }
        if let Some(focus) = focus {
            if input.just_pressed(Action::MenuConfirm) || input.just_pressed(Action::Attack) {
                model.activate(&focus.id, focus.index);
            }
            for (slot, action) in Action::HOTBAR.into_iter().enumerate() {
                if input.just_pressed(action) && focus.id == "items" {
                    let item = model.grid_item(focus.index);
                    model.hotbar.set(slot, item);
                }
            }
        }
        true
    }

    fn edit_search(&mut self, input: &InputState, game: &mut Gameplay) {
        let content = Arc::clone(&game.info.content);
        let Some(state) = self.inventory.as_mut() else {
            return;
        };
        let mut model = InventoryModel::new(state, &mut game.hotbar, &content, &self.language);
        model.type_text(input.typed());
        if input.just_pressed(Action::Erase) {
            model.type_text("\u{8}");
        }
        if input.just_pressed(Action::MenuBack) || input.just_pressed(Action::MenuConfirm) {
            model.state.search_focused = false;
        }
    }

    fn follow_mouse(&mut self, input: &InputState) {
        let Some((x, y)) = input.cursor().filter(|_| input.cursor_moved()) else {
            return;
        };
        let (ux, uy) = (
            (x / f64::from(self.scale)) as i32,
            (y / f64::from(self.scale)) as i32,
        );
        self.focus = self
            .hits
            .iter()
            .find(|hit| hit.rect.contains(ux, uy))
            .cloned();
    }

    fn navigate(&mut self, input: &InputState) {
        let moves = [
            (Action::MenuUp, Direction::Up),
            (Action::MenuDown, Direction::Down),
            (Action::MenuLeft, Direction::Left),
            (Action::MenuRight, Direction::Right),
        ];
        for (action, direction) in moves {
            if input.just_pressed(action) {
                self.focus =
                    navigate(&self.hits, self.focus.as_ref(), direction).or(self.focus.take());
            }
        }
    }

    /// Draws the HUD, and the open menu over it, on top of the frame in `target`.
    pub fn draw(
        &mut self,
        gpu: &Gpu,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        size: (u32, u32),
        game: &mut Gameplay,
    ) {
        self.scale = auto_scale(size.0, size.1, MAX_AUTO_SCALE);
        let screen = Rect::new(0, 0, size.0 / self.scale, size.1 / self.scale);
        let content = Arc::clone(&game.info.content);
        let ctx = UiContext {
            theme: &self.assets.theme,
            translations: &content.translations,
            language: &self.language,
        };
        let mut painter = Painter::new(&self.atlas, self.scale);
        let target_name = game
            .controls
            .show_block_name
            .then_some(game.interaction.target)
            .flatten()
            .and_then(|hit| {
                block_name(
                    &content,
                    game.view.world.block_at(hit.block),
                    &self.language,
                )
            });
        let hud = HudModel {
            hotbar: &game.hotbar,
            target: target_name,
        };
        if self.inventory.is_none() {
            draw_screen(&self.assets.hud, &mut painter, &hud, &ctx, screen, None);
        }
        if let Some(state) = self.inventory.as_mut() {
            let model = InventoryModel::new(state, &mut game.hotbar, &content, &self.language);
            self.hits = draw_screen(
                &self.assets.inventory,
                &mut painter,
                &model,
                &ctx,
                screen,
                self.focus.as_ref(),
            );
        }
        self.pipeline.draw(
            gpu,
            encoder,
            target,
            size,
            &painter.vertices,
            &painter.indices,
        );
    }
}

fn block_name(
    content: &Content,
    block: strata_core::content::BlockId,
    language: &str,
) -> Option<String> {
    let id = content.block_name(block)?;
    content
        .items
        .key(id)
        .and_then(|item| content.items.get(item))
        .filter(|i| matches!(i.kind, ItemKind::Block(_)))?;
    Some(
        content
            .translations
            .text(language, &id.translation_key("block")),
    )
}

#[cfg(test)]
mod tests {
    use crate::ui::UiAssets;
    use std::path::Path;
    use strata_core::content::Content;
    use strata_core::pack::PackSet;

    #[test]
    fn every_interface_text_is_translated() {
        let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets");
        let mut errors = Vec::new();
        let packs = PackSet::discover(&assets.join("packs"), &mut errors);
        let ui = UiAssets::load(&assets.join("ui"), &packs, &mut errors).unwrap();
        assert!(errors.is_empty(), "{errors:#?}");
        let (content, _) = Content::load(&packs, &PackSet::default());
        let mut keys = ui.translation_keys();
        keys.push(super::inventory::TAB_ALL_KEY.to_owned());
        let missing = content.translations.missing(&keys);
        assert!(missing.is_empty(), "missing translations: {missing:#?}");
    }
}
