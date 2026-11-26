use super::*;

fn round_trip<T: Serialize + serde::de::DeserializeOwned>(value: &T) -> T {
    let bytes = postcard::to_stdvec(value).unwrap();
    postcard::from_bytes(&bytes).unwrap()
}

#[test]
fn the_day_cycle_survives_serialization() {
    let day = DayState {
        time: 123_456,
        flows: true,
        cycle: DayCycleDef {
            length: 30_000,
            day_share: 0.62,
            twilight: 0.04,
            night_light: 0.15,
        },
    };
    assert_eq!(round_trip(&day), day);
    let time = ServerMessage::DayTime {
        time: 9,
        cycle: false,
    };
    assert_eq!(round_trip(&time), time);
}

#[test]
fn columns_keep_their_biomes() {
    use strata_core::chunk::BiomeId;
    use strata_core::content::Tint;
    let pos = strata_core::coords::ColumnPos { x: -3, z: 7 };
    let mut column = Column::empty(pos);
    column.set(4, 130, 9, BlockId(5));
    let biomes = BiomeMap::from_fn(|x, z| BiomeId(if x + z < 12 { 3 } else { 300 }));
    let message = ServerMessage::Column {
        column: Box::new(column),
        biomes,
    };
    assert_eq!(round_trip(&message), message);
    let welcome_biomes = vec![BiomeInfo {
        id: "strata:plains".parse().unwrap(),
        tints: BiomeTints {
            kinds: [Tint([1, 2, 3]), Tint([4, 5, 6]), Tint([7, 8, 9])],
            fog: Tint([200, 100, 50]),
        },
    }];
    assert_eq!(round_trip(&welcome_biomes), welcome_biomes);
}

#[test]
fn entity_messages_survive_serialization() {
    let id = NetEntity(7 | (3 << 32));
    let state = EntityState {
        id,
        kind: EntityTypeId(2),
        position: [12.5, 63.8125, -40.25],
        yaw: 1.5,
        item: Some(ItemId(3)),
    };
    let server = [
        ServerMessage::EntitySpawned(state),
        ServerMessage::EntityMoved {
            id,
            position: [1.0, 2.0, 3.0],
            yaw: -0.5,
        },
        ServerMessage::EntityRemoved(id),
        ServerMessage::Mounted(Some(id)),
        ServerMessage::Mounted(None),
        ServerMessage::ActionDone(ActionId(u32::MAX)),
        ServerMessage::Cracking {
            breaker: id,
            pos: BlockPos::new(-4, 300, 9),
            progress: Some(200),
        },
    ];
    for message in &server {
        assert_eq!(&round_trip(message), message);
    }
    for message in [
        ClientMessage::Action {
            id: ActionId(3),
            action: WorldAction::Interact(id),
        },
        ClientMessage::Action {
            id: ActionId(4),
            action: WorldAction::Attack(id),
        },
        ClientMessage::Action {
            id: ActionId(5),
            action: WorldAction::StartBreak {
                pos: BlockPos::new(1, 2, 3),
                face: Face::East,
            },
        },
        ClientMessage::Action {
            id: ActionId(6),
            action: WorldAction::StopBreak,
        },
        ClientMessage::Action {
            id: ActionId(9),
            action: WorldAction::Place {
                against: BlockPos::new(1, 2, 3),
                face: Face::East,
                block: BlockId(4),
            },
        },
    ] {
        assert_eq!(round_trip(&message), message);
    }
}

#[test]
fn inventory_messages_survive_serialization() {
    use crate::inventory::{CraftingView, InventoryRequest, OpenedContainer};
    use strata_core::inventory::{Area, Click, DataValue, ItemStack, SlotRef, SortOrder};
    let mut stack = ItemStack {
        item: ItemId(4),
        count: 12,
        durability: Some(7),
        data: Default::default(),
    };
    let parts = vec![
        DataValue::Id("strata:oak".parse().unwrap()),
        DataValue::Int(-3),
    ];
    stack
        .data
        .set("strata:parts".parse().unwrap(), DataValue::List(parts));
    let carried = DataValue::Stacks(vec![None, Some(stack.clone())]);
    stack
        .data
        .set("strata:bag_storage".parse().unwrap(), carried);
    let mut inventory = PlayerInventory::default();
    inventory.slots.slots[40] = Some(stack.clone());
    inventory.cursor = Some(stack.clone());
    let mut contents = Container::new(54);
    contents.slots[53] = Some(stack.clone());
    for message in [
        ServerMessage::Inventory(Box::new(inventory)),
        ServerMessage::ContainerOpened(Box::new(OpenedContainer {
            block: BlockId(9),
            pos: BlockPos::new(1, 2, 3),
            contents: contents.clone(),
            upgrades_at: 54,
            furnace: Some(FurnaceView::default()),
            bag: Some(ItemId(5)),
            tank: Some(crate::TankView {
                bucket: Some(ItemId(6)),
                buckets: 3,
                capacity: 8,
            }),
        })),
        ServerMessage::ContainerUpgraded {
            block: BlockId(10),
            upgrades_at: 108,
        },
        ServerMessage::ContainerContents(Box::new(contents)),
        ServerMessage::ContainerClosed,
        ServerMessage::Crafting(Box::new(CraftingView {
            side: 3,
            grid: Container::new(9),
            result: None,
            hint: true,
            station: Some((BlockId(4), BlockPos::new(1, 2, 3))),
            cuts: vec![(3, stack.clone())],
            chosen: Some(3),
            cost: Some(2),
        })),
        ServerMessage::Furnace(FurnaceView {
            burn_left: 4,
            burn_total: 1600,
            progress: 7,
            cook_time: 200,
            seconds: 8.0,
            per_coal: Some(10.0),
        }),
        ServerMessage::Recipes {
            known: vec![RecipeId(2)],
            all: false,
        },
        ServerMessage::RecipeLearned(RecipeId(3)),
    ] {
        assert_eq!(round_trip(&message), message);
    }
    for request in [
        InventoryRequest::Click {
            slot: SlotRef::open(53),
            click: Click::HotbarSwap(8),
        },
        InventoryRequest::Sort {
            area: Area::Open,
            order: SortOrder::Name,
            language: "lol".into(),
        },
        InventoryRequest::CreativeSet {
            index: 3,
            stack: Some(stack),
        },
        InventoryRequest::DropSelected { whole: true },
        InventoryRequest::OpenBag(46),
        InventoryRequest::Tank,
    ] {
        let message = ClientMessage::Inventory(request);
        assert_eq!(round_trip(&message), message);
    }
}

#[test]
fn survival_messages_survive_serialization() {
    let vitals = ServerMessage::Vitals(Box::new(crate::VitalsView {
        health: 13.5,
        max_health: 24.0,
        food: Some(17.0),
        max_food: 20.0,
        saturation: 3.0,
        thirst: None,
        max_thirst: 20.0,
        stamina: Some(40.0),
        max_stamina: 80.0,
        breath: 120,
        max_breath: 400,
        can_sprint: false,
        effects: vec![crate::EffectView {
            effect: "strata:bleeding".parse().unwrap(),
            seconds_left: 12,
            harmful: true,
        }],
    }));
    let hurt = ServerMessage::Hurt(crate::HurtView {
        kind: "strata:fall".parse().unwrap(),
        amount: 4.0,
        shakes: true,
    });
    for message in [vitals, hurt, ServerMessage::Fainted] {
        assert_eq!(round_trip(&message), message);
    }
    let item = ItemId(7);
    let pos = BlockPos::new(1, 62, -3);
    for action in [
        WorldAction::StartUse { item, slot: 3 },
        WorldAction::StopUse,
        WorldAction::Fill { item, pos },
        WorldAction::Drink { pos },
    ] {
        let message = ClientMessage::Action {
            id: ActionId(9),
            action,
        };
        assert_eq!(round_trip(&message), message);
    }
}
