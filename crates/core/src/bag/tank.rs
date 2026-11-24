//! A bag's tank: a full bucket carried onto it pours in, an empty one fills from it.

use super::contents::{BagContents, Tank};
use crate::content::{Item, ItemAction, ItemId, Registry};
use crate::inventory::{ItemRules, ItemStack};

/// What a click on the tank did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TankUse {
    /// The carried bucket poured into the tank.
    Poured,
    /// One carried empty bucket was filled: the full one is on the cursor, or (`Some`)
    /// left to put in the inventory when more empty buckets are carried.
    Filled(Option<ItemStack>),
    Nothing,
}

/// The tank of `capacity` buckets and the carried stack meet.
pub fn use_tank(
    contents: &mut BagContents,
    capacity: u16,
    cursor: &mut Option<ItemStack>,
    items: &Registry<ItemId, Item>,
) -> TankUse {
    let Some(carried) = cursor.as_ref() else {
        return TankUse::Nothing;
    };
    match items.get(carried.item).and_then(|item| item.action) {
        Some(ItemAction::PlaceFluid(_)) => pour(contents, capacity, cursor, items),
        Some(ItemAction::TakeFluid) => fill(contents, cursor, items),
        _ => TankUse::Nothing,
    }
}

fn pour(
    contents: &mut BagContents,
    capacity: u16,
    cursor: &mut Option<ItemStack>,
    items: &Registry<ItemId, Item>,
) -> TankUse {
    let Some(carried) = cursor.as_ref() else {
        return TankUse::Nothing;
    };
    let (Some(bucket), Some(empty)) = (
        items.identifier(carried.item),
        items.remainder(carried.item),
    ) else {
        return TankUse::Nothing;
    };
    let room = contents
        .tank
        .as_ref()
        .is_none_or(|tank| tank.bucket == *bucket && tank.buckets < capacity);
    if carried.count != 1 || !room {
        return TankUse::Nothing;
    }
    let tank = contents.tank.get_or_insert_with(|| Tank {
        bucket: bucket.clone(),
        buckets: 0,
    });
    tank.buckets += 1;
    *cursor = Some(ItemStack::new(empty, 1, items));
    TankUse::Poured
}

fn fill(
    contents: &mut BagContents,
    cursor: &mut Option<ItemStack>,
    items: &Registry<ItemId, Item>,
) -> TankUse {
    let Some(tank) = contents.tank.as_mut() else {
        return TankUse::Nothing;
    };
    let full = items.key(&tank.bucket);
    let (Some(full), Some(carried)) = (full, cursor.as_mut()) else {
        return TankUse::Nothing;
    };
    if items.remainder(full) != Some(carried.item) {
        return TankUse::Nothing;
    }
    tank.buckets -= 1;
    if tank.buckets == 0 {
        contents.tank = None;
    }
    let filled = ItemStack::new(full, 1, items);
    if carried.count == 1 {
        *cursor = Some(filled);
        return TankUse::Filled(None);
    }
    carried.count -= 1;
    TankUse::Filled(Some(filled))
}
