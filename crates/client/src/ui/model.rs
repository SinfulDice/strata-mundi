use super::Rect;
use strata_core::content::ItemId;

/// What a screen shows and how it reacts. The layout comes from the screen's RON file;
/// the model answers by widget id. Every method has a neutral default, so a model only
/// writes what its screen uses.
pub trait ScreenModel {
    /// Text bound with `@name` (labels, buttons), or the content of a text field.
    fn text(&self, _id: &str) -> Option<String> {
        None
    }
    /// Items of a grid (current page) or a row of slots.
    fn items(&self, _id: &str) -> Vec<Option<ItemId>> {
        Vec::new()
    }
    /// Selected tab, slot or list row.
    fn selected(&self, _id: &str) -> Option<usize> {
        None
    }
    /// Translated tab titles.
    fn tabs(&self, _id: &str) -> Vec<String> {
        Vec::new()
    }
    /// Visible rows of a list (already scrolled).
    fn rows(&self, _id: &str) -> Vec<ListRow> {
        Vec::new()
    }
    /// Slider position (0 to 1) and caption.
    fn slider(&self, _id: &str) -> Option<(f32, String)> {
        None
    }
    /// Tooltip for an element (item names).
    fn tooltip(&self, _id: &str, _index: usize) -> Option<String> {
        None
    }
    /// Clicked, or confirmed with Enter / the A button.
    fn activate(&mut self, _id: &str, _index: usize) {}
    /// Left/right on a focused slider (or a click at `fraction` of its width).
    fn adjust(&mut self, _id: &str, _step: i32, _fraction: Option<f32>) {}
    /// Wheel or page keys over a list or grid.
    fn scroll(&mut self, _id: &str, _rows: i32) {}
    /// Number key `slot` pressed while pointing at an element (item → hotbar slot).
    fn quick_slot(&mut self, _id: &str, _index: usize, _slot: usize) {}
    /// Characters typed into a text field (`\u{8}` erases one).
    fn type_text(&mut self, _id: &str, _text: &str) {}
}

/// One line of a `List` widget.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ListRow {
    pub title: String,
    pub detail: String,
    /// Image in the interface atlas (world capture).
    pub icon: Option<Rect>,
    /// Shown greyed (a world that cannot be opened).
    pub dim: bool,
}

/// One interactive element on screen, for clicks and focus navigation.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: String,
    pub index: usize,
    pub rect: super::Rect,
    pub kind: HitKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Button,
    TextField,
    Slider,
    Slot,
    /// Not interactive in play; selectable in the HUD editor.
    Element,
}
