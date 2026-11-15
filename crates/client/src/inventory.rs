use crate::ui::SlotMark;
use strata_core::content::{Content, Item, ItemId, Registry};
use strata_core::gear::{worn_burden, worn_protection};
use strata_core::inventory::{
    ARMOR, Area, Click, HOTBAR, ItemStack, PlayerInventory, SlotRef, SortOrder,
};
use strata_core::upgrade::max_durability;
use strata_protocol::InventoryRequest;

pub const SLOTS: usize = HOTBAR.end;

/// The client's copy of the player's inventory, as the server last sent it. The hand's
/// slot is chosen here (and told to the server); every other change is a request the
/// server answers with the new inventory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ClientInventory {
    inventory: PlayerInventory,
    /// Creative: any item may be put in a slot.
    pub creative: bool,
    outbox: Vec<InventoryRequest>,
}

impl ClientInventory {
    pub fn selected_index(&self) -> usize {
        self.inventory.selected_index()
    }

    /// The item in hand; a broken one is no better than a hand.
    pub fn selected(&self) -> Option<ItemId> {
        self.inventory
            .selected_stack()
            .filter(|stack| !stack.is_broken())
            .map(|stack| stack.item)
    }

    pub fn slot(&self, index: usize) -> Option<ItemId> {
        self.stack(index).map(|stack| stack.item)
    }

    pub fn stack(&self, index: usize) -> Option<&ItemStack> {
        self.inventory.slots.get(index)
    }

    /// Counts and locks of the first `count` slots (the hotbar), for drawing.
    pub fn marks(&self, count: usize, content: &Content) -> Vec<SlotMark> {
        (0..count)
            .map(|index| {
                let stack = self.stack(index);
                SlotMark {
                    count: stack.map_or(0, |stack| stack.count),
                    wear: stack.and_then(|stack| wear_share(stack, content)),
                    locked: self.inventory.is_locked(index),
                    dim: stack.is_some_and(ItemStack::is_broken),
                }
            })
            .collect()
    }

    fn worn(&self) -> impl Iterator<Item = &ItemStack> {
        ARMOR.filter_map(|slot| self.stack(slot))
    }

    /// Protection points of the armour worn, shown in the inventory.
    pub fn protection(&self, content: &Content) -> u16 {
        worn_protection(self.worn(), &content.items, &content.upgrades)
    }

    /// Walking speed allowed by the armour worn, as the server computes it.
    pub fn armour_speed(&self, content: &Content) -> f64 {
        let burden = worn_burden(self.worn(), &content.items, &content.upgrades);
        f64::from(burden.speed_factor())
    }

    pub fn inventory(&self) -> &PlayerInventory {
        &self.inventory
    }

    pub fn select(&mut self, index: usize) {
        let index = index.min(SLOTS - 1);
        if index != self.selected_index() {
            self.inventory.selected = index as u8;
            self.outbox.push(InventoryRequest::Select(index as u8));
        }
    }

    /// Scrolls through the slots, wrapping around (`steps` > 0 = to the right).
    pub fn scroll(&mut self, steps: i32) {
        if steps != 0 {
            self.select((self.selected_index() as i32 + steps).rem_euclid(SLOTS as i32) as usize);
        }
    }

    /// Creative: puts one `item` in slot `index` (or empties it). Shown at once; the
    /// server's next inventory overwrites this guess with what it accepted.
    pub fn set(&mut self, index: usize, item: Option<ItemId>, items: &Registry<ItemId, Item>) {
        if !self.creative {
            return;
        }
        let stack = item.map(|item| ItemStack::new(item, 1, items));
        if let Some(slot) = self.inventory.slots.slots.get_mut(index) {
            slot.clone_from(&stack);
            self.outbox.push(InventoryRequest::CreativeSet {
                index: index as u16,
                stack,
            });
        }
    }

    /// The server's inventory; the hand stays where the player put it meanwhile.
    pub fn receive(&mut self, inventory: PlayerInventory) {
        let selected = self.inventory.selected;
        self.inventory = inventory;
        self.inventory.selected = selected;
    }

    /// On arrival: the hand as saved.
    pub fn restore(&mut self, inventory: PlayerInventory) {
        self.inventory = inventory;
        self.inventory.normalize();
    }

    /// Middle click: select the item if already in the hotbar; in creative, otherwise put
    /// it in the selected slot (or the first empty one).
    pub fn pick(&mut self, item: ItemId, items: &Registry<ItemId, Item>) {
        if let Some(index) = HOTBAR.clone().find(|&i| self.slot(i) == Some(item)) {
            self.select(index);
            return;
        }
        if !self.creative {
            return;
        }
        if self.selected().is_some()
            && let Some(empty) = HOTBAR.clone().find(|&i| self.slot(i).is_none())
        {
            self.select(empty);
        }
        self.set(self.selected_index(), Some(item), items);
    }

    pub fn click(&mut self, slot: SlotRef, click: Click) {
        self.outbox.push(InventoryRequest::Click { slot, click });
    }

    pub fn sort(&mut self, area: Area, order: SortOrder, language: &str) {
        self.outbox.push(InventoryRequest::Sort {
            area,
            order,
            language: language.to_owned(),
        });
    }

    pub fn request(&mut self, request: InventoryRequest) {
        self.outbox.push(request);
    }

    /// Requests to send to the server, oldest first.
    pub fn take_requests(&mut self) -> Vec<InventoryRequest> {
        std::mem::take(&mut self.outbox)
    }
}

/// Uses left as a share of a new one's, for the wear bar; `None` if it does not wear.
pub fn wear_share(stack: &ItemStack, content: &Content) -> Option<f32> {
    let full = max_durability(stack, &content.items, &content.upgrades)?;
    let left = stack.durability?;
    Some(f32::from(left) / f32::from(full.max(1)))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use strata_core::content::{ItemKind, TextureId};

    fn items() -> Registry<ItemId, Item> {
        let mut items = Registry::default();
        for name in ["a", "b"] {
            let item = Item {
                max_stack: 64,
                category: None,
                kind: ItemKind::Flat(TextureId::MISSING),
                action: None,
                durability: None,
                equip: None,
                tool: None,
                remainder: None,
                kit: None,
                module: None,
                gear: Default::default(),
            };
            items.insert(format!("strata:{name}").parse().unwrap(), item);
        }
        items
    }

    #[test]
    fn picking_reuses_existing_slots_then_empty_ones() {
        let items = items();
        let mut hotbar = ClientInventory {
            creative: true,
            ..ClientInventory::default()
        };
        hotbar.pick(ItemId(0), &items);
        assert_eq!(hotbar.selected(), Some(ItemId(0)));
        hotbar.pick(ItemId(1), &items);
        assert_eq!(hotbar.selected_index(), 1);
        hotbar.pick(ItemId(0), &items);
        assert_eq!(hotbar.selected_index(), 0);
        assert_eq!(hotbar.take_requests().len(), 4, "two sets, two selections");
    }

    #[test]
    fn survival_only_selects_what_it_has() {
        let items = items();
        let mut hotbar = ClientInventory::default();
        hotbar.pick(ItemId(0), &items);
        assert_eq!(hotbar.selected(), None);
        assert!(hotbar.take_requests().is_empty());
    }

    #[test]
    fn scrolling_wraps_and_the_hand_survives_server_updates() {
        let mut hotbar = ClientInventory::default();
        hotbar.scroll(-1);
        assert_eq!(hotbar.selected_index(), 8);
        hotbar.scroll(2);
        assert_eq!(hotbar.selected_index(), 1);
        hotbar.receive(PlayerInventory::default());
        assert_eq!(hotbar.selected_index(), 1);
    }
}
