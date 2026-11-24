//! The open bag's panel beside the inventory screen: the 3×3 grid of its crafting
//! upgrade and the tank of its tank upgrade; and the back slot of the inventory.

use super::slots::SlotsModel;
use strata_core::content::ItemAction;
use strata_core::inventory::BACK;
use strata_protocol::TankView;

impl SlotsModel<'_> {
    fn bag_open(&self) -> bool {
        self.container.is_some_and(|open| open.bag.is_some())
    }

    fn tank(&self) -> Option<TankView> {
        self.container?.tank
    }

    /// The bag's crafting upgrade lent the screen a 3×3 grid.
    fn bag_crafting(&self) -> bool {
        let side = self.crafting.map(|c| (c.view.side, c.view.station));
        self.bag_open() && side.is_some_and(|(side, station)| side == 3 && station.is_none())
    }

    pub(super) fn bag_shown(&self, id: &str) -> Option<bool> {
        Some(match id {
            "bag_side" => self.bag_crafting() || self.tank().is_some(),
            "bag_crafting" => self.bag_crafting(),
            "bag_tank" => self.tank().is_some(),
            _ => return None,
        })
    }

    pub(super) fn tank_level(&self) -> Option<f32> {
        let tank = self.tank()?;
        Some(f32::from(tank.buckets) / f32::from(tank.capacity.max(1)))
    }

    /// « Water: 3 / 8 buckets », or « Empty (0 / 8) ».
    pub(super) fn tank_text(&self) -> Option<String> {
        let tank = self.tank()?;
        let tr = &self.content.translations;
        let liquid = tank.bucket.and_then(|bucket| {
            let ItemAction::PlaceFluid(block) = self.content.items.get(bucket)?.action? else {
                return None;
            };
            let id = self.content.block_name(block)?;
            Some(tr.text(self.language, &id.translation_key("block")))
        });
        let args = [
            ("liquid", liquid.unwrap_or_default()),
            ("buckets", tank.buckets.to_string()),
            ("capacity", tank.capacity.to_string()),
        ];
        let key = match tank.bucket {
            Some(_) => "ui-bag-tank-holds",
            None => "ui-bag-tank-empty",
        };
        Some(tr.text_with(self.language, key, &args))
    }

    /// What goes in the empty back slot.
    pub(super) fn back_slot_tip(&self) -> Option<String> {
        if self.inventory.stack(BACK).is_some() {
            return None;
        }
        let tr = &self.content.translations;
        Some(tr.text(self.language, "ui-inventory-back"))
    }
}
