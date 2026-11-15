#![allow(clippy::unwrap_used)]

use super::*;
use crate::content::{Item, ItemId, ItemKind, Registry, TextureId};
use crate::inventory::ItemStack;

fn id(text: &str) -> Identifier {
    text.parse().unwrap()
}

fn iron() -> Tier {
    Tier {
        level: 2,
        speed: 6.0,
        durability: 250,
        damage: 2.0,
        enchantability: 14,
        upgrade_slots: 1,
    }
}

fn gear() -> Gear {
    let mut gear = Gear::default();
    gear.tiers.insert(id("strata:iron"), iron());
    let heavy = ArmorWeight {
        speed: 0.025,
        stamina: 0.1,
        mana: 0.1,
    };
    gear.weights.insert(id("strata:heavy"), heavy);
    gear.weights
        .insert(id("strata:light"), ArmorWeight::default());
    gear
}

fn fields(tier: Option<&str>) -> GearFields {
    GearFields {
        tier: tier.map(id),
        tool: None,
        weapon: None,
        armor: None,
        durability: None,
    }
}

fn resolve(fields: GearFields) -> (Resolved, Vec<PackError>) {
    let mut errors = Vec::new();
    let resolved = gear().resolve(fields, Path::new("test.ron"), &mut errors);
    (resolved, errors)
}

fn spec(kinds: &[&str]) -> ToolSpec {
    ToolSpec {
        kinds: kinds.iter().map(|kind| id(kind)).collect(),
        level: None,
        speed: None,
        sweep: 0,
    }
}

#[test]
fn a_tier_fills_what_the_tool_leaves_out() {
    let (resolved, errors) = resolve(GearFields {
        tool: Some(spec(&["strata:pickaxe"])),
        ..fields(Some("strata:iron"))
    });
    assert!(errors.is_empty(), "{errors:?}");
    let tool = resolved.tool.unwrap();
    assert_eq!((tool.level, tool.speed), (2, 6.0));
    assert_eq!(resolved.durability, Some(250));
    assert_eq!(resolved.gear.upgrade_slots, 1);
    assert_eq!(resolved.gear.enchantability, 14);
}

#[test]
fn a_multi_tool_keeps_its_kinds_and_its_own_durability() {
    let (resolved, _) = resolve(GearFields {
        tool: Some(spec(&["strata:pickaxe", "strata:axe", "strata:shovel"])),
        durability: Some(150),
        ..fields(Some("strata:iron"))
    });
    let tool = resolved.tool.unwrap();
    assert_eq!(tool.kinds.len(), 3);
    assert!(tool.is(&id("strata:axe")) && tool.is(&id("strata:shovel")));
    assert_eq!(resolved.durability, Some(150));
}

#[test]
fn weapon_damage_adds_the_tier() {
    let (resolved, _) = resolve(GearFields {
        weapon: Some(WeaponDef {
            damage: 4.0,
            attack_speed: 1.6,
        }),
        ..fields(Some("strata:iron"))
    });
    let weapon = resolved.gear.weapon.unwrap();
    assert_eq!(weapon.damage, 6.0);
    assert!((weapon.per_second() - 9.6).abs() < 1e-5);
    assert_eq!(resolved.durability, Some(250));
}

#[test]
fn unknown_tiers_and_weights_are_reported() {
    let (resolved, errors) = resolve(GearFields {
        armor: Some(ArmorDef {
            protection: 3,
            weight: id("strata:feather"),
        }),
        ..fields(Some("strata:mithril"))
    });
    assert_eq!(errors.len(), 2);
    assert!(resolved.gear.armor.is_none() && resolved.gear.tier.is_none());
}

fn armor_items() -> (Registry<ItemId, Item>, ItemId, ItemId) {
    let gear = gear();
    let mut items = Registry::default();
    let mut add = |name: &str, protection, weight: &str| {
        let armor = ArmorDef {
            protection,
            weight: id(weight),
        };
        let resolved = gear.resolve(
            GearFields {
                armor: Some(armor),
                durability: Some(100),
                ..fields(None)
            },
            Path::new("test.ron"),
            &mut Vec::new(),
        );
        let item = Item {
            max_stack: 1,
            category: None,
            kind: ItemKind::Flat(TextureId::MISSING),
            action: None,
            durability: resolved.durability,
            equip: None,
            tool: None,
            remainder: None,
            kit: None,
            module: None,
            gear: resolved.gear,
        };
        items.insert(id(name), item).unwrap()
    };
    let plate = add("strata:iron_chestplate", 6, "strata:heavy");
    let cap = add("strata:leather_cap", 1, "strata:light");
    (items, plate, cap)
}

#[test]
fn protection_adds_the_pieces_but_not_broken_ones() {
    let (items, plate, cap) = armor_items();
    let worn = [
        ItemStack::new(plate, 1, &items),
        ItemStack::new(cap, 1, &items),
    ];
    assert_eq!(worn_protection(&worn, &items), 7);
    let mut broken = worn.clone();
    broken[0].wear(100);
    assert!(broken[0].is_broken());
    assert_eq!(worn_protection(&broken, &items), 1);
}

#[test]
fn heavy_armour_slows_light_armour_does_not() {
    let (items, plate, cap) = armor_items();
    let light = [ItemStack::new(cap, 1, &items)];
    assert_eq!(worn_burden(&light, &items).speed_factor(), 1.0);
    let heavy: Vec<ItemStack> = (0..4).map(|_| ItemStack::new(plate, 1, &items)).collect();
    let burden = worn_burden(&heavy, &items);
    assert!((burden.speed_factor() - 0.9).abs() < 1e-5);
    assert!((burden.stamina - 0.4).abs() < 1e-5 && (burden.mana - 0.4).abs() < 1e-5);
    let many: Vec<ItemStack> = (0..40).map(|_| ItemStack::new(plate, 1, &items)).collect();
    assert!(worn_burden(&many, &items).speed_factor() >= 1.0 - MAX_BURDEN);
}
