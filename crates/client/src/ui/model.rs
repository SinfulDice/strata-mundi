use strata_core::content::ItemId;

/// What a screen shows and how it reacts. The layout comes from the screen's RON file;
/// the model answers by widget id.
pub trait ScreenModel {
    /// Text bound with `@name` in a label, or the content of a text field.
    fn text(&self, id: &str) -> Option<String>;
    /// Items of a grid (current page) or a row of slots.
    fn items(&self, id: &str) -> Vec<Option<ItemId>>;
    /// Selected tab or slot.
    fn selected(&self, id: &str) -> Option<usize>;
    /// Translated tab titles.
    fn tabs(&self, id: &str) -> Vec<String>;
    /// Tooltip for an element (item names).
    fn tooltip(&self, id: &str, index: usize) -> Option<String>;
    /// Clicked, or confirmed with Enter / the A button.
    fn activate(&mut self, id: &str, index: usize);
}

/// One interactive element on screen, for clicks and focus navigation.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub id: String,
    pub index: usize,
    pub rect: super::Rect,
}
