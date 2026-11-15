//! The held stack as the server sees it (upgrades counted) and the miner's hammer on the
//! client: the area chosen, shown in the HUD and outlined in the world, changed with its
//! key. The server works the area out again and decides.

use crate::inventory::ClientInventory;
use crate::world::ClientWorld;
use strata_core::content::{BlockId, Content, ToolDef};
use strata_core::coords::BlockPos;
use strata_core::inventory::ItemStack;
use strata_core::raycast::RayHit;
use strata_core::upgrade::{AreaSize, area_cells, chosen_area, fits_area, stack_stats};
use strata_protocol::InventoryRequest;

/// What the stack in hand mines with, and the hammer areas it unlocked.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HeldGear {
    pub tool: Option<ToolDef>,
    /// Smallest first; empty for anything but an upgraded hammer.
    pub areas: Vec<AreaSize>,
    pub area: AreaSize,
}

fn held_stack(inventory: &ClientInventory) -> Option<&ItemStack> {
    let stack = inventory.stack(inventory.selected_index())?;
    (!stack.is_broken()).then_some(stack)
}

pub fn held_gear(inventory: &ClientInventory, content: &Content) -> HeldGear {
    let Some(stack) = held_stack(inventory) else {
        return HeldGear::default();
    };
    let stats = stack_stats(stack, &content.items, &content.upgrades);
    HeldGear {
        area: chosen_area(stack, &stats.areas),
        tool: stats.tool,
        areas: stats.areas,
    }
}

/// The next area unlocked (back to 1×1 after the largest), to ask the server for.
pub fn next_area(inventory: &ClientInventory, content: &Content) -> Option<InventoryRequest> {
    let gear = held_gear(inventory, content);
    let now = gear.areas.iter().position(|&area| area == gear.area)?;
    let next = (now + 1) % gear.areas.len();
    Some(InventoryRequest::ChooseArea(u8::try_from(next).ok()?))
}

/// « Zone : 3×3 », while a hammer with more than one area is held.
pub fn area_line(inventory: &ClientInventory, content: &Content, language: &str) -> Option<String> {
    let gear = held_gear(inventory, content);
    if gear.areas.len() < 2 {
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
    let Some(tool) = gear.tool.as_ref().filter(|_| gear.area != AreaSize::SINGLE) else {
        return single;
    };
    let aimed = content.blocks.get(world.block_at(hit.block));
    if !aimed.is_some_and(|block| fits_area(block, tool)) {
        return single;
    }
    let cells = area_cells(hit.block, hit.face, gear.area, yaw);
    cells.iter().fold(single, |(low, high), cell| {
        (
            BlockPos::new(low.x.min(cell.x), low.y.min(cell.y), low.z.min(cell.z)),
            BlockPos::new(high.x.max(cell.x), high.y.max(cell.y), high.z.max(cell.z)),
        )
    })
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
    use strata_core::inventory::PlayerInventory;
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
            block: BlockPos::new(1, 0, 1),
            face: strata_core::content::Face::Up,
        };
        let empty = ClientWorld::default();
        assert_eq!(
            outline_box(hit, &gear, 0.0, &empty, &content),
            (hit.block, hit.block)
        );
    }
}
