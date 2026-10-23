use strata_core::content::ItemId;

pub const SLOTS: usize = 9;

/// The nine quick slots at the bottom of the screen. In creative they hold any item;
/// survival inventories (v0.3) will live on the server.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Hotbar {
    slots: [Option<ItemId>; SLOTS],
    selected: usize,
}

impl Hotbar {
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    pub fn selected(&self) -> Option<ItemId> {
        self.slots[self.selected]
    }

    pub fn slot(&self, index: usize) -> Option<ItemId> {
        self.slots.get(index).copied().flatten()
    }

    pub fn select(&mut self, index: usize) {
        self.selected = index.min(SLOTS - 1);
    }

    /// Scrolls through the slots, wrapping around (`steps` > 0 = to the right).
    pub fn scroll(&mut self, steps: i32) {
        self.selected = (self.selected as i32 + steps).rem_euclid(SLOTS as i32) as usize;
    }

    pub fn set(&mut self, index: usize, item: Option<ItemId>) {
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = item;
        }
    }

    /// Middle click: select the item if already in the hotbar, otherwise put it in the
    /// selected slot (or the first empty one).
    pub fn pick(&mut self, item: ItemId) {
        if let Some(index) = self.slots.iter().position(|&s| s == Some(item)) {
            self.selected = index;
            return;
        }
        if self.slots[self.selected].is_some()
            && let Some(empty) = self.slots.iter().position(Option::is_none)
        {
            self.selected = empty;
        }
        self.slots[self.selected] = Some(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picking_reuses_existing_slots_then_empty_ones() {
        let mut hotbar = Hotbar::default();
        hotbar.pick(ItemId(4));
        assert_eq!(hotbar.selected(), Some(ItemId(4)));
        hotbar.pick(ItemId(7));
        assert_eq!(hotbar.selected_index(), 1);
        hotbar.pick(ItemId(4));
        assert_eq!(hotbar.selected_index(), 0);
    }

    #[test]
    fn scrolling_wraps() {
        let mut hotbar = Hotbar::default();
        hotbar.scroll(-1);
        assert_eq!(hotbar.selected_index(), 8);
        hotbar.scroll(2);
        assert_eq!(hotbar.selected_index(), 1);
    }
}
