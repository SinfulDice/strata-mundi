//! The filter set on a filter or destruction upgrade, kept in the upgrade item's data: up
//! to nine items, listed as allowed (only they pass) or refused (all but them pass). An
//! empty filter does nothing: everything passes a filter, nothing is destroyed.

use crate::content::{Item, ItemId, Registry};
use crate::id::Identifier;
use crate::inventory::{DataValue, ItemStack};

/// Items a filter lists at most.
pub const FILTER_CELLS: usize = 9;

fn key(name: &'static str) -> Identifier {
    Identifier::builtin("strata", name)
}

/// A filter as the engine reads it.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ItemFilter {
    /// By cell; an empty cell is `None`.
    pub cells: Vec<Option<ItemId>>,
    /// The listed items are refused, rather than the only ones allowed.
    pub deny: bool,
}

impl ItemFilter {
    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(Option::is_none)
    }

    /// Whether `item` is selected: listed (allow mode) or not listed (deny mode). An
    /// empty filter selects nothing.
    pub fn selects(&self, item: ItemId) -> bool {
        if self.is_empty() {
            return false;
        }
        self.cells.contains(&Some(item)) != self.deny
    }

    /// Whether `item` passes as a filter on what enters: an empty filter lets all in.
    pub fn lets_in(&self, item: ItemId) -> bool {
        self.is_empty() || self.selects(item)
    }
}

/// The filter set on an upgrade stack; items the packs no longer have are left out.
pub fn filter_of(stack: &ItemStack, items: &Registry<ItemId, Item>) -> ItemFilter {
    let cells = match stack.data.get(&key("filter")) {
        Some(DataValue::List(list)) => list
            .iter()
            .take(FILTER_CELLS)
            .map(|cell| match cell {
                DataValue::Id(id) => items.key(id),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let deny = matches!(stack.data.get(&key("filter_deny")), Some(DataValue::Int(1)));
    let mut cells: Vec<Option<ItemId>> = cells;
    cells.resize(FILTER_CELLS, None);
    ItemFilter { cells, deny }
}

/// Cell `cell` of the filter now lists `item` (or nothing). Returns whether it changed.
pub fn set_filter_cell(
    stack: &mut ItemStack,
    cell: usize,
    item: Option<ItemId>,
    items: &Registry<ItemId, Item>,
) -> bool {
    if cell >= FILTER_CELLS {
        return false;
    }
    let mut filter = filter_of(stack, items);
    if filter.cells.get(cell) == Some(&item) || (item.is_some() && filter.cells.contains(&item)) {
        return false;
    }
    filter.cells[cell] = item;
    let list = filter
        .cells
        .iter()
        .map(|cell| match cell.and_then(|item| items.identifier(item)) {
            Some(id) => DataValue::Id(id.clone()),
            None => DataValue::Int(0),
        })
        .collect();
    stack.data.set(key("filter"), DataValue::List(list));
    true
}

/// Switches the filter between allowing and refusing what it lists.
pub fn toggle_filter_mode(stack: &mut ItemStack, items: &Registry<ItemId, Item>) {
    let deny = filter_of(stack, items).deny;
    stack
        .data
        .set(key("filter_deny"), DataValue::Int(i64::from(!deny)));
}
