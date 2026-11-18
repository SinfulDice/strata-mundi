use super::*;
use crate::content::{EquipSlot, Item, ItemId, ItemKind, Registry, TextureId};
use crate::gear::{Armor, Burden, ItemGear};
use crate::id::Identifier;
use crate::inventory::{ACCESSORIES, ARMOR, ItemStack, PlayerInventory};
use crate::upgrade::{Effect, StatOp, Upgrades};

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

fn brick(stat: &str, value: f32) -> Effect {
    Effect::Stat {
        stat: id(stat),
        op: StatOp::Add,
        value,
    }
}

struct Kit {
    items: Registry<ItemId, Item>,
    helmet: ItemId,
    chest: ItemId,
    legs: ItemId,
    boots: ItemId,
    ring: ItemId,
    big_ring: ItemId,
}

fn piece(equip: EquipSlot, worn: Vec<Effect>, protection: u16) -> Item {
    let armor = (protection > 0).then(|| Armor {
        protection,
        weight: id("strata:light"),
        burden: Burden::default(),
    });
    Item {
        max_stack: 1,
        category: None,
        kind: ItemKind::Flat(TextureId::MISSING),
        action: None,
        durability: armor.as_ref().map(|_| 10),
        equip: Some(equip),
        tool: None,
        remainder: None,
        kit: None,
        module: None,
        gear: ItemGear {
            armor,
            worn,
            ..ItemGear::default()
        },
    }
}

fn kit() -> Kit {
    let mut items = Registry::default();
    let mut add = |name: &str, item| items.insert(id(name), item).unwrap();
    let helmet = add("strata:helmet", piece(EquipSlot::Head, vec![], 1));
    let chest = add("strata:chest", piece(EquipSlot::Chest, vec![], 3));
    let legs = add("strata:legs", piece(EquipSlot::Legs, vec![], 2));
    let boots = add(
        "strata:boots",
        piece(EquipSlot::Feet, vec![brick("strata:air_jumps", 1.0)], 1),
    );
    let ring = add(
        "strata:ring",
        piece(EquipSlot::Ring, vec![brick("strata:max_health", 4.0)], 0),
    );
    let big_ring = add(
        "strata:big_ring",
        piece(EquipSlot::Ring, vec![brick("strata:max_health", 6.0)], 0),
    );
    Kit {
        items,
        helmet,
        chest,
        legs,
        boots,
        ring,
        big_ring,
    }
}

fn rules(kit: &Kit) -> Worn {
    Worn {
        sets: vec![GearSet {
            id: id("strata:test"),
            pieces: vec![kit.helmet, kit.chest, kit.legs, kit.boots],
            bonuses: vec![
                SetBonus {
                    pieces: 2,
                    effects: vec![brick("strata:protection", 1.0)],
                },
                SetBonus {
                    pieces: 4,
                    effects: vec![brick("strata:protection", 3.0), brick("strata:dash", 1.0)],
                },
            ],
        }],
        mobility: MobilityRules::default(),
    }
}

fn wearing(kit: &Kit, armour: &[ItemId], accessories: &[ItemId]) -> PlayerInventory {
    let mut inventory = PlayerInventory::default();
    for (slot, item) in ARMOR.zip(armour) {
        inventory.slots.slots[slot] = Some(ItemStack::new(*item, 1, &kit.items));
    }
    for (slot, item) in ACCESSORIES.zip(accessories) {
        inventory.slots.slots[slot] = Some(ItemStack::new(*item, 1, &kit.items));
    }
    inventory
}

fn stats(kit: &Kit, inventory: &PlayerInventory) -> WornStats {
    inventory_worn_stats(inventory, &kit.items, &Upgrades::default(), &rules(kit))
}

#[test]
fn two_like_rings_count_once_and_the_strongest_brick_wins() {
    let kit = kit();
    let one = stats(&kit, &wearing(&kit, &[], &[kit.ring]));
    assert_eq!(one.get(WornStat::MaxHealth), 4.0);
    let two = stats(&kit, &wearing(&kit, &[], &[kit.ring, kit.ring]));
    assert_eq!(
        two.get(WornStat::MaxHealth),
        4.0,
        "same effect, not cumulative"
    );
    let mixed = stats(&kit, &wearing(&kit, &[], &[kit.ring, kit.big_ring]));
    assert_eq!(mixed.get(WornStat::MaxHealth), 6.0, "the strongest counts");
}

#[test]
fn set_bonuses_grow_with_the_pieces_worn() {
    let kit = kit();
    let pieces = [kit.helmet, kit.chest, kit.legs, kit.boots];
    let expected = [
        (1, 1, None),
        (2, 5, Some(0)),
        (3, 7, Some(0)),
        (4, 10, Some(1)),
    ];
    for (count, protection, bonus) in expected {
        let worn = stats(&kit, &wearing(&kit, &pieces[..count], &[]));
        let progress = worn.set(0).unwrap();
        assert_eq!((progress.worn, progress.total), (count as u8, 4));
        assert_eq!(progress.bonus, bonus, "{count} pieces");
        assert_eq!(worn.protection, protection, "{count} pieces");
    }
    let full = stats(&kit, &wearing(&kit, &pieces, &[]));
    assert_eq!(full.get(WornStat::Dash), 1.0);
    assert!(stats(&kit, &wearing(&kit, &[], &[])).sets.is_empty());
}

#[test]
fn a_broken_piece_gives_nothing_to_its_set() {
    let kit = kit();
    let mut inventory = wearing(&kit, &[kit.helmet, kit.chest], &[]);
    inventory.slots.slots[ARMOR.start]
        .as_mut()
        .unwrap()
        .wear(10);
    let worn = stats(&kit, &inventory);
    assert_eq!(worn.set(0).unwrap().worn, 1);
    assert_eq!(worn.protection, 3);
}

#[test]
fn worn_stats_give_the_mobility_abilities() {
    let kit = kit();
    let pieces = [kit.helmet, kit.chest, kit.legs, kit.boots];
    let rules = MobilityRules::default();
    let boots = stats(&kit, &wearing(&kit, &[kit.boots], &[]));
    let mobility = rules.mobility(&boots);
    assert_eq!(mobility.air_jumps, 1);
    assert!(mobility.dash.is_none() && mobility.climb.is_none());
    let full = rules.mobility(&stats(&kit, &wearing(&kit, &pieces, &[])));
    assert_eq!(full.dash.map(|d| d.cooldown), Some(rules.dash_cooldown));
}

#[test]
fn the_bonus_reached_is_the_largest_within_the_pieces() {
    let kit = kit();
    let set = &rules(&kit).sets[0];
    assert_eq!(set.bonus(1), None);
    assert_eq!(set.bonus(3), Some(0));
    assert_eq!(set.bonus(9), Some(1));
}

#[test]
fn mobility_rules_are_bounded() {
    let wild = MobilityRules {
        air_jump_speed: f64::NAN,
        max_air_jumps: 200,
        dash_speed: 1e9,
        dash_ticks: 0,
        dash_cooldown: 0,
        climb_speed: -3.0,
    }
    .clamped();
    assert_eq!(wild.air_jump_speed, 0.5);
    assert_eq!(wild.max_air_jumps, 5);
    assert_eq!(wild.dash_speed, 20.0);
    assert_eq!((wild.dash_ticks, wild.dash_cooldown), (1, 5));
    assert_eq!(wild.climb_speed, 0.5);
    let kit = kit();
    let boots = stats(&kit, &wearing(&kit, &[kit.boots], &[]));
    let none = MobilityRules {
        max_air_jumps: 0,
        ..MobilityRules::default()
    };
    assert_eq!(none.mobility(&boots).air_jumps, 0, "capped by the rules");
}

#[test]
fn the_last_pack_with_mobility_rules_wins() {
    let dir = std::env::temp_dir().join(format!("strata-mobility-{}", std::process::id()));
    for (name, priority, speed) in [("first", 0, 3.0), ("second", 10, 9.0)] {
        let root = dir.join(name);
        std::fs::create_dir_all(&root).unwrap();
        let manifest = format!("(namespace: \"{name}\", priority: {priority})");
        std::fs::write(root.join("pack.ron"), manifest).unwrap();
        let rules = format!("Mobility(dash_speed: {speed})");
        std::fs::write(root.join("mobility.ron"), rules).unwrap();
    }
    let mut errors = Vec::new();
    let packs = crate::pack::PackSet::discover(&dir, &mut errors);
    let rules = super::load::mobility_rules(&packs, &mut errors);
    assert!(errors.is_empty(), "{errors:?}");
    assert_eq!(rules.dash_speed, 9.0);
    assert_eq!(rules.climb_speed, MobilityRules::default().climb_speed);
    std::fs::remove_dir_all(dir).unwrap();
}
