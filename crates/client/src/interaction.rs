//! Aiming at blocks and entities, breaking, placing and picking blocks, getting into boats. The client only sends intents;
//! the world changes when the server answers with `BlockChanged`.

use crate::camera::Camera;
use crate::hotbar::Hotbar;
use crate::input::{Action, InputState};
use crate::world::ClientWorld;
use glam::DVec3;
use std::sync::Arc;
use strata_core::content::{BlockId, Content, HitBox, ItemAction, ItemId, ItemKind};
use strata_core::physics::{Aabb, LiquidCell};
use strata_core::raycast::{RayHit, ray_box, raycast};
use strata_protocol::{ClientMessage, NetEntity};

/// Creative reach, in blocks from the eyes.
const REACH: f64 = 5.0;
/// Holding a button repeats the action this often (seconds), like Minecraft.
const REPEAT_DELAY: f32 = 0.25;

/// Block lookups the interaction code needs, built once from the content.
pub struct BlockInfo {
    pub content: Arc<Content>,
    solid: Vec<bool>,
    hit_boxes: Vec<Option<HitBox>>,
    items: Vec<Option<ItemId>>,
    liquids: Vec<Option<LiquidCell>>,
    sources: Vec<bool>,
}

impl BlockInfo {
    pub fn new(content: Arc<Content>) -> Self {
        let solid = content.solidity();
        let hit_boxes = content
            .blocks
            .iter()
            .map(|(_, _, block)| block.hit_box())
            .collect();
        let items = content
            .blocks
            .iter()
            .map(|(key, _, _)| content.block_name(key).and_then(|id| content.items.key(id)))
            .collect();
        let liquids = content.liquids();
        let sources = content
            .blocks
            .iter()
            .map(|(key, _, block)| block.fluid.is_some_and(|state| state.source == key))
            .collect();
        Self {
            content,
            solid,
            hit_boxes,
            items,
            liquids,
            sources,
        }
    }

    pub fn is_solid(&self, block: BlockId) -> bool {
        self.solid
            .get(usize::from(block.0))
            .copied()
            .unwrap_or(false)
    }

    pub fn hit_box(&self, block: BlockId) -> Option<HitBox> {
        self.hit_boxes.get(usize::from(block.0)).copied().flatten()
    }

    pub fn liquid(&self, block: BlockId) -> Option<LiquidCell> {
        self.liquids.get(usize::from(block.0)).copied().flatten()
    }

    /// What an empty bucket can aim at: a liquid source counts as a full block.
    fn bucket_hit_box(&self, block: BlockId) -> Option<HitBox> {
        let source = self.sources.get(usize::from(block.0)) == Some(&true);
        if source {
            Some(HitBox::FULL)
        } else {
            self.hit_box(block)
        }
    }

    /// Boats go on liquids too: any liquid counts as a full block.
    fn boat_hit_box(&self, block: BlockId) -> Option<HitBox> {
        match self.liquid(block) {
            Some(_) => Some(HitBox::FULL),
            None => self.hit_box(block),
        }
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
    /// Entity aimed at, closer than any block.
    pub entity: Option<NetEntity>,
    /// The boat the player sits in: never aimed at.
    pub mount: Option<NetEntity>,
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
        self.target = aim(camera, world, |block| info.hit_box(block));
        self.entity = aim_entity(camera, world, info, self.target, self.mount);
        if self.entity.is_some() {
            self.target = None;
        }
        self.cooldown = (self.cooldown - dt).max(0.0);
        if let Some(message) = self.entity_action(input) {
            return Some(message);
        }
        let triggered =
            |action| input.just_pressed(action) || (input.is_held(action) && self.cooldown == 0.0);
        let held = hotbar.selected();
        let action = held.and_then(|item| info.content.items.get(item)?.action);
        if let (Some(item), Some(action), true) = (held, action, triggered(Action::Use)) {
            let message = use_item(camera, world, info, item, action);
            if message.is_some() {
                self.cooldown = REPEAT_DELAY;
            }
            return message;
        }
        let target = self.target?;
        if input.just_pressed(Action::PickBlock) {
            if let Some(item) = info.item_of(world.block_at(target.block)) {
                hotbar.pick(item);
            }
            return None;
        }
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

impl Interaction {
    /// Right click gets into the aimed boat, left click takes it away (creative).
    fn entity_action(&mut self, input: &InputState) -> Option<ClientMessage> {
        let entity = self.entity?;
        let message = if input.just_pressed(Action::Use) {
            ClientMessage::InteractEntity(entity)
        } else if input.just_pressed(Action::Attack) {
            ClientMessage::AttackEntity(entity)
        } else {
            return None;
        };
        self.cooldown = REPEAT_DELAY;
        Some(message)
    }
}

/// The closest entity the view ray crosses within reach, if no block is closer.
fn aim_entity(
    camera: &Camera,
    world: &ClientWorld,
    info: &BlockInfo,
    block: Option<RayHit>,
    mount: Option<NetEntity>,
) -> Option<NetEntity> {
    let direction = camera.forward().as_dvec3().try_normalize()?;
    let origin = camera.position;
    let block_distance = block
        .and_then(|hit| {
            let shape = info.hit_box(world.block_at(hit.block))?;
            let corner = DVec3::new(
                f64::from(hit.block.x),
                f64::from(hit.block.y),
                f64::from(hit.block.z),
            );
            let (min, max) = (
                corner + DVec3::from(shape.min),
                corner + DVec3::from(shape.max),
            );
            ray_box(origin, direction, min, max, REACH)
        })
        .unwrap_or(REACH);
    world
        .entities
        .iter()
        .filter(|(id, _, _)| Some(*id) != mount)
        .filter_map(|(id, kind, pose)| {
            let size = info.content.entities.get(kind)?.size;
            let aabb = Aabb::standing(pose.position, size.width, size.height);
            let distance = ray_box(origin, direction, aabb.min, aabb.max, REACH)?;
            (distance < block_distance).then_some((distance, id))
        })
        .min_by(|a, b| a.0.total_cmp(&b.0))
        .map(|(_, id)| id)
}

/// Buckets: a full one pours against what is aimed at, an empty one takes the source;
/// boats go on the aimed liquid or block.
fn use_item(
    camera: &Camera,
    world: &ClientWorld,
    info: &BlockInfo,
    item: ItemId,
    action: ItemAction,
) -> Option<ClientMessage> {
    let target = match action {
        ItemAction::PlaceFluid(_) => aim(camera, world, |block| info.hit_box(block)),
        ItemAction::TakeFluid => aim(camera, world, |block| info.bucket_hit_box(block)),
        ItemAction::PlaceEntity(_) => aim(camera, world, |block| info.boat_hit_box(block)),
    }?;
    Some(ClientMessage::UseItem {
        item,
        against: target.block,
        face: target.face,
    })
}

fn aim(
    camera: &Camera,
    world: &ClientWorld,
    hit_box: impl Fn(BlockId) -> Option<HitBox>,
) -> Option<RayHit> {
    let direction = camera.forward().as_dvec3();
    raycast(camera.position, direction, REACH, |pos| {
        hit_box(world.block_at(pos))
    })
}
