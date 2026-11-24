//! Which slots each grid of the inventory screen shows (the open container's grids are
//! placed by `storage_panel`).

use super::SlotsModel;
use strata_core::cooking::{FURNACE_FUEL, FURNACE_INPUT, FURNACE_OUTPUT};
use strata_core::inventory::{ACCESSORIES, ARMOR, Area, HOTBAR, ItemStack, MAIN, OFFHAND, SlotRef};

/// Where the slots of a grid are: the area and its slots (the open container's grids
/// are placed by `storage_grid`).
fn grid(id: &str) -> Option<(Area, std::ops::Range<usize>)> {
    Some(match id {
        "hotbar" => (Area::Player, HOTBAR),
        "main" => (Area::Player, MAIN),
        "armor" => (Area::Player, ARMOR),
        "offhand" => (Area::Player, OFFHAND..OFFHAND + 1),
        "accessories" => (Area::Player, ACCESSORIES),
        "furnace_input" => (Area::Open, FURNACE_INPUT..FURNACE_INPUT + 1),
        "furnace_fuel" => (Area::Open, FURNACE_FUEL..FURNACE_FUEL + 1),
        "furnace_output" => (Area::Open, FURNACE_OUTPUT..FURNACE_OUTPUT + 1),
        "craft" | "table" | "cutter" | "anvil" | "bench" => (Area::Craft, 0..usize::from(u16::MAX)),
        "craft_result" | "table_result" | "cutter_result" | "anvil_result" | "bench_result" => {
            (Area::CraftResult, 0..1)
        }
        _ => return None,
    })
}

impl SlotsModel<'_> {
    pub(in crate::screens) fn grid(&self, id: &str) -> Option<(Area, std::ops::Range<usize>)> {
        self.storage_grid(id).or_else(|| grid(id))
    }

    /// The slot behind element `index` of grid `id`.
    pub fn slot(&self, id: &str, index: usize) -> Option<SlotRef> {
        let (area, range) = self.grid(id)?;
        let slot = range.start + index;
        let exists = range.contains(&slot)
            && match area {
                Area::Player | Area::CraftResult => true,
                Area::Open => self.container.is_some_and(|c| slot < c.contents.len()),
                Area::Craft => self.crafting.is_some_and(|c| slot < c.view.grid.len()),
            };
        exists.then_some(SlotRef {
            area,
            index: slot as u16,
        })
    }

    pub(in crate::screens) fn stack(&self, slot: SlotRef) -> Option<&ItemStack> {
        let index = usize::from(slot.index);
        match slot.area {
            Area::Player => self.inventory.stack(index),
            Area::Open => self.container?.contents.get(index),
            Area::Craft => self.crafting?.view.grid.get(index),
            Area::CraftResult => self.crafting?.view.result.as_ref(),
        }
    }

    pub(in crate::screens) fn stacks(&self, id: &str) -> Vec<Option<&ItemStack>> {
        let Some((area, range)) = self.grid(id) else {
            return Vec::new();
        };
        let end = match area {
            Area::Player | Area::CraftResult => range.end,
            Area::Open => self.container.map_or(0, |c| c.contents.len()),
            Area::Craft => self.crafting.map_or(0, |c| c.view.grid.len()),
        }
        .min(range.end);
        (range.start..end)
            .map(|index| {
                self.stack(SlotRef {
                    area,
                    index: index as u16,
                })
            })
            .collect()
    }
}
