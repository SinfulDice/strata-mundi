//! What tooltips tell about bags: their slots and how to use them, what a pouch takes,
//! and what one holds.

use super::BagContents;
use crate::content::{Content, ItemId};
use crate::inventory::ItemStack;

impl Content {
    /// Lines of a bag item's tooltip: its slots, then how to open and put it down (or
    /// what a pouch takes).
    pub(crate) fn bag_item_lines(&self, item: ItemId, language: &str) -> Vec<String> {
        let Some(bag) = self.bags.get(item) else {
            return Vec::new();
        };
        let tr = &self.translations;
        let slots = ("slots", bag.layout.storage.to_string());
        let mut lines = vec![match bag.layout.upgrades {
            0 => tr.text_with(language, "bag-pouch-slots-line", &[slots]),
            upgrades => {
                let args = [slots, ("upgrades", upgrades.to_string())];
                tr.text_with(language, "bag-slots-line", &args)
            }
        }];
        let accepts = self
            .items
            .get(item)
            .and_then(|i| i.bag.as_ref()?.accepts.as_ref());
        if let Some(tag) = accepts {
            let what = tr.text(language, &tag.translation_key("bag-accepts"));
            lines.push(tr.text_with(language, "bag-pouch-line", &[("what", what)]));
        }
        let key = match bag.placed {
            Some(_) => "bag-backpack-line",
            None => "bag-open-line",
        };
        lines.push(tr.text(language, key));
        let worn = self.items.get(item).and_then(|i| i.equip).is_some();
        if worn {
            lines.push(tr.text(language, "bag-back-line"));
        }
        lines
    }

    /// « Holds 12 items » for a bag that holds something (from the summary the client
    /// receives with the inventory).
    pub fn bag_stack_line(&self, stack: &ItemStack, language: &str) -> Option<String> {
        let bag = self.bags.get(stack.item)?;
        let (count, upgrades) = BagContents::summary(&stack.data, &bag.layout);
        if count == 0 && upgrades == 0 {
            return None;
        }
        let tr = &self.translations;
        let count = ("count", count.to_string());
        Some(match upgrades {
            0 => tr.text_with(language, "bag-holds-line", &[count]),
            _ => {
                let args = [count, ("upgrades", upgrades.to_string())];
                tr.text_with(language, "bag-holds-upgrades-line", &args)
            }
        })
    }
}
