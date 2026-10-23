//! Aiming at blocks, breaking, placing and picking them. The client only sends intents;
//! the world changes when the server answers with `BlockChanged`.

use crate::camera::Camera;
use crate::hotbar::Hotbar;
use crate::input::{Action, InputState};
use crate::world::ClientWorld;
use std::sync::Arc;
use strata_core::content::{BlockId, Content, ItemId, ItemKind};
use strata_core::raycast::{RayHit, raycast};
use strata_protocol::ClientMessage;

/// Creative reach, in blocks from the eyes.
const REACH: f64 = 5.0;
/// Holding a button repeats the action this often (seconds), like Minecraft.
const REPEAT_DELAY: f32 = 0.25;

/// Block lookups the interaction code needs, built once from the content.
pub struct BlockInfo {
    pub content: Arc<Content>,
    solid: Vec<bool>,
    items: Vec<Option<ItemId>>,
}

impl BlockInfo {
    pub fn new(content: Arc<Content>) -> Self {
        let solid = content.solidity();
        let items = content
            .blocks
            .iter()
            .map(|(key, _, _)| content.block_name(key).and_then(|id| content.items.key(id)))
            .collect();
        Self {
            content,
            solid,
            items,
        }
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        self.solid
            .get(usize::from(block.0))
            .copied()
            .unwrap_or(false)
    }

    pub fn item_of(&self, block: BlockId) -> Option<ItemId> {
        self.items.get(usize::from(block.0)).copied().flatten()
    }

    pub fn block_of(&self, item: ItemId) -> Option<BlockId> {
        match self.content.items.get(item)?.kind {
            ItemKind::Block(block) => Some(block),
            ItemKind::Flat(_) => None,
        }
    }
}

#[derive(Default)]
pub struct Interaction {
    pub target: Option<RayHit>,
    cooldown: f32,
}

impl Interaction {
    pub fn update(
        &mut self,
        dt: f32,
        camera: &Camera,
        world: &ClientWorld,
        info: &BlockInfo,
        input: &InputState,
        hotbar: &mut Hotbar,
    ) -> Option<ClientMessage> {
        self.target = aim(camera, world, info);
        self.cooldown = (self.cooldown - dt).max(0.0);
        let target = self.target?;
        if input.just_pressed(Action::PickBlock) {
            if let Some(item) = info.item_of(world.block_at(target.block)) {
                hotbar.pick(item);
            }
            return None;
        }
        let triggered =
            |action| input.just_pressed(action) || (input.is_held(action) && self.cooldown == 0.0);
        let message = if triggered(Action::Attack) {
            Some(ClientMessage::BreakBlock { pos: target.block })
        } else if triggered(Action::Use) {
            let block = hotbar.selected().and_then(|item| info.block_of(item))?;
            Some(ClientMessage::PlaceBlock {
                against: target.block,
                face: target.face,
                block,
            })
        } else {
            None
        };
        if message.is_some() {
            self.cooldown = REPEAT_DELAY;
        }
        message
    }
}

fn aim(camera: &Camera, world: &ClientWorld, info: &BlockInfo) -> Option<RayHit> {
    let direction = camera.forward().as_dvec3();
    raycast(camera.position, direction, REACH, |pos| {
        let block = world.block_at(pos);
        block != BlockId::AIR && info.is_solid(block)
    })
}
