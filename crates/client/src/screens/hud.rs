use crate::gameplay::Gameplay;
use crate::hotbar::{Hotbar, SLOTS};
use crate::ui::ScreenModel;
use strata_core::content::{Content, ItemId, ItemKind};

/// Data shown by the in-game overlay (sample values in the HUD editor).
pub struct HudModel<'a> {
    pub hotbar: &'a Hotbar,
    /// Translated name of the aimed block, if shown.
    pub target: Option<String>,
}

impl ScreenModel for HudModel<'_> {
    fn text(&self, id: &str) -> Option<String> {
        (id == "target").then(|| self.target.clone()).flatten()
    }

    fn items(&self, id: &str) -> Vec<Option<ItemId>> {
        match id {
            "hotbar" => (0..SLOTS).map(|i| self.hotbar.slot(i)).collect(),
            _ => Vec::new(),
        }
    }

    fn selected(&self, id: &str) -> Option<usize> {
        (id == "hotbar").then(|| self.hotbar.selected_index())
    }
}

/// Translated name of the block the player aims at, unless hidden in the options.
pub fn aimed_block_name(game: &Gameplay, content: &Content, language: &str) -> Option<String> {
    if !game.controls.show_block_name {
        return None;
    }
    let block = game.view.world.block_at(game.interaction.target?.block);
    let id = content.block_name(block)?;
    let item = content
        .items
        .key(id)
        .and_then(|item| content.items.get(item))?;
    matches!(item.kind, ItemKind::Block(_)).then(|| {
        content
            .translations
            .text(language, &id.translation_key("block"))
    })
}
