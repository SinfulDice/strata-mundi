use crate::hotbar::{Hotbar, SLOTS};
use crate::ui::ScreenModel;
use strata_core::content::ItemId;

/// Data shown by the in-game overlay.
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

    fn tabs(&self, _: &str) -> Vec<String> {
        Vec::new()
    }

    fn tooltip(&self, _: &str, _: usize) -> Option<String> {
        None
    }

    fn activate(&mut self, _: &str, _: usize) {}
}
