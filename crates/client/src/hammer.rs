//! The held stack as the server sees it (upgrades counted) and the miner's hammer on the
//! client: the area chosen, shown in the HUD and outlined in the world, changed with its
//! key. The server works the area out again and decides.

use crate::inventory::ClientInventory;
use crate::world::ClientWorld;
use std::sync::Arc;
use strata_core::content::{BlockId, Content, ToolDef};
use strata_core::coords::BlockPos;
use strata_core::raycast::RayHit;
use strata_core::upgrade::{AreaSize, ItemStats, area_bounds, chosen_area, fits_area};
use strata_protocol::InventoryRequest;

/// What the stack in hand mines with, and the hammer area chosen.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeldGear {
    stats: Option<Arc<ItemStats>>,
    pub area: AreaSize,
}

impl HeldGear {
    pub fn tool(&self) -> Option<&ToolDef> {
        self.stats.as_ref()?.tool.as_ref()
    }

    /// Smallest first; empty for anything but an upgraded hammer.
    pub fn areas(&self) -> &[AreaSize] {
        self.stats
            .as_ref()
            .map_or(&[], |stats| stats.areas.as_slice())
    }
}

pub fn held_gear(inventory: &ClientInventory, content: &Content) -> HeldGear {
    let slot = inventory.selected_index();
    let stack = inventory.stack(slot).filter(|stack| !stack.is_broken());
    let Some((stack, stats)) = stack.zip(inventory.slot_stats(slot, content)) else {
        return HeldGear::default();
    };
    HeldGear {
        area: chosen_area(stack, &stats.areas),
        stats: Some(stats),
    }
}

/// The next area unlocked (back to 1×1 after the largest), to ask the server for.
pub fn next_area(inventory: &ClientInventory, content: &Content) -> Option<InventoryRequest> {
    let gear = held_gear(inventory, content);
    let areas = gear.areas();
    let now = areas.iter().position(|&area| area == gear.area)?;
    let next = (now + 1) % areas.len();
    Some(InventoryRequest::ChooseArea(u8::try_from(next).ok()?))
}

/// « Zone : 3×3 », while a hammer with more than one area is held.
pub fn area_line(inventory: &ClientInventory, content: &Content, language: &str) -> Option<String> {
    let gear = held_gear(inventory, content);
    if gear.areas().len() < 2 {
        return None;
    }
    let args = [
        ("width", gear.area.width.to_string()),
        ("height", gear.area.height.to_string()),
    ];
    Some(
        content
            .translations
            .text_with(language, "hud-hammer-area", &args),
    )
}

/// The corners (lowest, highest) of the box outlined around the aimed block: the whole
/// area when the hammer would break more than this block.
pub fn outline_box(
    hit: RayHit,
    gear: &HeldGear,
    yaw: f32,
    world: &ClientWorld,
    content: &Content,
) -> (BlockPos, BlockPos) {
    let single = (hit.block, hit.block);
    let Some(tool) = gear.tool().filter(|_| gear.area != AreaSize::SINGLE) else {
        return single;
    };
    let aimed = content.blocks.get(world.block_at(hit.block));
    if !aimed.is_some_and(|block| fits_area(block, tool)) {
        return single;
    }
    area_bounds(hit.block, hit.face, gear.area, yaw)
}

/// The blocks of the area other than the one hit, for the break time.
pub fn blocks_at<'a>(
    world: &'a ClientWorld,
    content: &'a Content,
) -> impl Fn(BlockPos) -> Option<&'a strata_core::content::Block> {
    move |pos| {
        let block = world.block_at(pos);
        (block != BlockId::AIR)
            .then(|| content.blocks.get(block))
            .flatten()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::path::Path;
    use strata_core::chunk::{BiomeId, BiomeMap, Column};
    use strata_core::coords::ColumnPos;
    use strata_core::inventory::{ItemStack, PlayerInventory};
    use strata_core::pack::PackSet;
    use strata_core::upgrade::{with_area, with_upgrade};

    fn holding(level: u8, area: Option<usize>) -> (Content, ClientInventory) {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut Vec::new());
        let content = Content::load(&packs, &PackSet::default()).0;
        let key = content.items.key(&"strata:iron_hammer".parse().unwrap());
        let hammer = ItemStack::new(key.unwrap(), 1, &content.items);
        let head = "strata:hammer_area".parse().unwrap();
        let mut hammer = with_upgrade(&hammer, &head, level);
        if let Some(index) = area {
            hammer = with_area(&hammer, index);
        }
        let mut inventory = PlayerInventory::default();
        inventory.slots.slots[0] = Some(hammer);
        let mut client = ClientInventory::default();
        client.restore(inventory);
        (content, client)
    }

    #[test]
    fn the_area_key_goes_round_the_areas_unlocked() {
        let (content, inventory) = holding(2, None);
        let line = area_line(&inventory, &content, "en").unwrap();
        assert!(line.contains("5×1"), "{line}");
        let next = next_area(&inventory, &content);
        assert_eq!(next, Some(InventoryRequest::ChooseArea(0)), "back to 1×1");
        let (content, inventory) = holding(2, Some(0));
        let next = next_area(&inventory, &content);
        assert_eq!(next, Some(InventoryRequest::ChooseArea(1)));
    }

    #[test]
    fn the_outline_covers_the_area_on_a_block_it_mines() {
        let (content, inventory) = holding(3, None);
        let gear = held_gear(&inventory, &content);
        let hit = RayHit {
            block: BlockPos::new(2, 0, 2),
            face: strata_core::content::Face::Up,
        };
        let empty = ClientWorld::default();
        let alone = outline_box(hit, &gear, 0.0, &empty, &content);
        assert_eq!(alone, (hit.block, hit.block), "nothing there to mine");
        let stone = content
            .blocks
            .key(&"strata:stone".parse().unwrap())
            .unwrap();
        let mut column = Column::empty(ColumnPos { x: 0, z: 0 });
        for x in 0..5 {
            for z in 0..5 {
                column.set(x, 0, z, stone);
            }
        }
        let mut world = ClientWorld::default();
        world.insert(column, BiomeMap::uniform(BiomeId(0)));
        let area = outline_box(hit, &gear, 0.0, &world, &content);
        assert_eq!(
            area,
            (BlockPos::new(1, 0, 1), BlockPos::new(3, 0, 3)),
            "3×3"
        );
    }
}
