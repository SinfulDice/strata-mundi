//! What a bag holds, kept in its item's data (or in the block entity of a bag put down):
//! its slots, its upgrade slots and the liquid of its tank.

use crate::id::Identifier;
use crate::inventory::{Container, DataValue, ItemData, ItemStack};
use crate::storage::SlotLayout;

fn key(name: &'static str) -> Identifier {
    Identifier::builtin("strata", name)
}

/// The liquid in a bag's tank: the full bucket it comes in (`strata:water_bucket`), and
/// how many of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tank {
    pub bucket: Identifier,
    pub buckets: u16,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BagContents {
    pub storage: Container,
    pub upgrades: Container,
    pub tank: Option<Tank>,
}

impl BagContents {
    /// What `data` holds, laid out as `layout` says. Slots a pack no longer gives are
    /// returned, to be thrown.
    pub fn read(data: &ItemData, layout: &SlotLayout) -> (Self, Vec<ItemStack>) {
        let mut storage = stacks(data, "bag_storage");
        let mut upgrades = stacks(data, "bag_upgrades");
        let mut cut = storage.resize(layout.storage);
        cut.extend(upgrades.resize(layout.upgrades));
        let tank = Self::read_tank(data);
        let contents = Self {
            storage,
            upgrades,
            tank,
        };
        (contents, cut)
    }

    /// Only the upgrade slots kept in `data` (what a bag's upgrades do, without copying
    /// its slots).
    pub fn read_upgrades(data: &ItemData, layout: &SlotLayout) -> Container {
        let mut upgrades = stacks(data, "bag_upgrades");
        // What a pack no longer gives a slot for does nothing.
        let _cut = upgrades.resize(layout.upgrades);
        upgrades
    }

    /// Keeps the contents in `data`; what is empty leaves no trace.
    pub fn write(&self, data: &mut ItemData) {
        put_stacks(data, "bag_storage", &self.storage);
        put_stacks(data, "bag_upgrades", &self.upgrades);
        Self::write_tank(data, self.tank.as_ref());
    }

    /// The tank kept in `data` (a bag's, or a bag put down's block entity).
    pub fn read_tank(data: &ItemData) -> Option<Tank> {
        let Some(DataValue::List(list)) = data.get(&key("tank")) else {
            return None;
        };
        match list.as_slice() {
            [DataValue::Id(bucket), DataValue::Int(buckets)] => Some(Tank {
                bucket: bucket.clone(),
                buckets: u16::try_from(*buckets).ok().filter(|&b| b > 0)?,
            }),
            _ => None,
        }
    }

    pub fn write_tank(data: &mut ItemData, tank: Option<&Tank>) {
        match tank {
            Some(tank) if tank.buckets > 0 => data.set(
                key("tank"),
                DataValue::List(vec![
                    DataValue::Id(tank.bucket.clone()),
                    DataValue::Int(i64::from(tank.buckets)),
                ]),
            ),
            _ => {
                data.0.remove(&key("tank"));
            }
        }
    }

    /// In the block entity of a bag put down: the bag item it gives back.
    pub fn put_down_as(data: &ItemData) -> Option<&Identifier> {
        match data.get(&key("bag")) {
            Some(DataValue::Id(id)) => Some(id),
            _ => None,
        }
    }

    /// Marks a block entity's data as a bag put down that gives back `bag` (`None`: no
    /// longer a bag).
    pub fn set_put_down_as(data: &mut ItemData, bag: Option<&Identifier>) {
        match bag {
            Some(id) => data.set(key("bag"), DataValue::Id(id.clone())),
            None => {
                data.0.remove(&key("bag"));
            }
        }
    }

    /// What a client is told of a bag outside its screen: its contents give way to how
    /// many items and upgrades it holds (its screen gets the slots themselves).
    pub fn summarise(data: &mut ItemData, layout: &SlotLayout) {
        let (contents, _) = Self::read(data, layout);
        let count: u32 = contents
            .storage
            .slots
            .iter()
            .flatten()
            .map(|s| u32::from(s.count))
            .sum();
        let upgrades = contents.upgrades.slots.iter().flatten().count();
        Self::strip(data);
        if count > 0 || upgrades > 0 {
            let summary = [i64::from(count), upgrades as i64].map(DataValue::Int);
            data.set(key("bag_summary"), DataValue::List(summary.to_vec()));
        }
    }

    /// Items and upgrades a bag holds: from its summary, or counted.
    pub fn summary(data: &ItemData, layout: &SlotLayout) -> (u32, u32) {
        if let Some(DataValue::List(list)) = data.get(&key("bag_summary"))
            && let [DataValue::Int(count), DataValue::Int(upgrades)] = list.as_slice()
        {
            return (
                u32::try_from(*count).unwrap_or(0),
                u32::try_from(*upgrades).unwrap_or(0),
            );
        }
        let (contents, _) = Self::read(data, layout);
        let count = contents
            .storage
            .slots
            .iter()
            .flatten()
            .map(|s| u32::from(s.count))
            .sum();
        let upgrades = contents.upgrades.slots.iter().flatten().count();
        (count, u32::try_from(upgrades).unwrap_or(u32::MAX))
    }

    /// The bag's data without its contents (what stays on a bag put down).
    pub fn strip(data: &mut ItemData) {
        for name in ["bag_storage", "bag_upgrades", "tank"] {
            data.0.remove(&key(name));
        }
    }

    /// The window of the bag: its slots, then its upgrade slots.
    pub fn window(&self) -> Container {
        let mut slots = self.storage.slots.clone();
        slots.extend(self.upgrades.slots.iter().cloned());
        Container { slots }
    }

    /// Takes back the window's slots after a click.
    pub fn set_window(&mut self, window: Container) {
        let mut slots = window.slots;
        let upgrades = slots.split_off(self.storage.len().min(slots.len()));
        self.storage.slots = slots;
        self.upgrades.slots = upgrades;
    }

    /// Every stack inside, upgrades included.
    pub fn stacks(&self) -> impl Iterator<Item = &ItemStack> {
        self.storage
            .slots
            .iter()
            .chain(&self.upgrades.slots)
            .flatten()
    }
}

fn stacks(data: &ItemData, name: &'static str) -> Container {
    match data.get(&key(name)) {
        Some(DataValue::Stacks(slots)) => Container {
            slots: slots.clone(),
        },
        _ => Container::default(),
    }
}

fn put_stacks(data: &mut ItemData, name: &'static str, container: &Container) {
    if container.is_empty() {
        data.0.remove(&key(name));
    } else {
        data.set(key(name), DataValue::Stacks(container.slots.clone()));
    }
}
