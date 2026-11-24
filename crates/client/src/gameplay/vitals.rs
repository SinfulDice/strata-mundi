//! The survival gauges as the server tells them, and what the player feels: the red
//! flash and the shake of a hit, the notice after fainting, eating with the use button
//! held (progress and sounds).

use super::Gameplay;
use crate::audio::{Audio, Cue};
use crate::input::{Action, InputState};
use crate::interaction::aim_source;
use strata_core::content::{Content, ItemId, SoundId};
use strata_protocol::{HurtView, VitalsView, WorldAction};

/// Seconds the red flash and the shake last.
const HURT_SECONDS: f32 = 0.4;
/// Seconds the notice after fainting stays.
const FAINT_SECONDS: f32 = 6.0;
/// Seconds between two bites or sips heard while using an item.
const USE_SOUND_EVERY: f32 = 0.25;
/// Strongest shake, in radians (at 100 % and a heavy hit).
const MAX_SHAKE: f32 = 0.06;

#[derive(Debug, Default)]
pub struct ClientVitals {
    pub view: Option<VitalsView>,
    /// Seconds left of the last hit's flash and shake, and how strong it was (0 to 1).
    pub hurt: f32,
    hurt_strength: f32,
    hurt_shakes: bool,
    /// Seconds left of the fainting notice.
    pub fainted: f32,
    /// The item held down with the use button.
    pub using: Option<ClientUse>,
    sounds: Vec<SoundId>,
}

#[derive(Debug, Clone, Copy)]
pub struct ClientUse {
    pub item: ItemId,
    pub slot: usize,
    /// Seconds held, and seconds one use takes.
    pub held: f32,
    pub needed: f32,
    pub sound: Option<SoundId>,
    since_sound: f32,
}

impl ClientUse {
    /// How far the current bite is, 0 to 1 (it starts again while held).
    pub fn progress(&self) -> f32 {
        if self.needed <= 0.0 {
            return 0.0;
        }
        (self.held % self.needed) / self.needed
    }
}

impl ClientVitals {
    pub fn hit(&mut self, hurt: &HurtView, content: &Content) {
        self.hurt = HURT_SECONDS;
        let max = self.view.as_ref().map_or(20.0, |v| v.max_health.max(1.0));
        self.hurt_strength = (hurt.amount / max * 4.0).clamp(0.25, 1.0);
        self.hurt_shakes = hurt.shakes;
        if let Some(id) = content.sounds.registry.key(&hurt_sound()) {
            self.sounds.push(id);
        }
    }

    pub fn faint(&mut self) {
        self.fainted = FAINT_SECONDS;
        self.using = None;
    }

    /// Red flash opacity, 0 to 1.
    pub fn flash(&self) -> f32 {
        if self.hurt <= 0.0 {
            return 0.0;
        }
        self.hurt / HURT_SECONDS * self.hurt_strength
    }

    /// Camera tilt of the shake this frame, in radians, for a setting of `percent`.
    pub fn shake(&self, percent: u32) -> f32 {
        if self.hurt <= 0.0 || !self.hurt_shakes {
            return 0.0;
        }
        let fade = self.hurt / HURT_SECONDS;
        let wave = (self.hurt * 60.0).sin();
        wave * fade * self.hurt_strength * MAX_SHAKE * percent.min(100) as f32 / 100.0
    }

    pub(super) fn advance(&mut self, dt: f32) {
        self.hurt = (self.hurt - dt).max(0.0);
        self.fainted = (self.fainted - dt).max(0.0);
        if let Some(using) = &mut self.using {
            using.held += dt;
            using.since_sound += dt;
            if using.since_sound >= USE_SOUND_EVERY {
                using.since_sound = 0.0;
                self.sounds.extend(using.sound);
            }
        }
    }
}

fn hurt_sound() -> strata_core::id::Identifier {
    strata_core::survival::engine("hurt")
}

impl Gameplay {
    pub(super) fn receive_vitals(&mut self, view: VitalsView) {
        if let Some(player) = &mut self.player {
            player.movement.can_sprint = view.can_sprint;
        }
        self.vitals.view = Some(view);
    }

    /// An empty bottle used on water fills; sneaking with an empty hand, one drinks.
    pub(super) fn fill_or_drink(&self, input: &InputState) -> Option<WorldAction> {
        if !input.just_pressed(Action::Use) || self.interaction.entity.is_some() {
            return None;
        }
        let survival = &self.info.content.survival;
        let target = aim_source(&self.camera, &self.view.world, &self.info)?;
        let block = self.view.world.block_at(target.block);
        match self.inventory.selected() {
            Some(item) => {
                let fill = survival.fills.get(&item)?;
                (fill.liquid == block).then_some(WorldAction::Fill {
                    item,
                    pos: target.block,
                })
            }
            None => {
                let drinkable = survival.drink_source == Some(block);
                (drinkable && input.is_held(Action::Sneak))
                    .then_some(WorldAction::Drink { pos: target.block })
            }
        }
    }

    /// Holding the use button with something to eat, drink or apply in hand consumes it
    /// (the server counts the time); letting go, or another item, stops.
    pub(super) fn use_consumable(&mut self, input: &InputState) -> Option<WorldAction> {
        let content = &self.info.content;
        let slot = self.inventory.selected_index();
        let held = self.inventory.selected();
        let consumable =
            held.and_then(|item| Some((item, content.survival.consumables.get(&item)?)));
        let holding = input.is_held(Action::Use) && self.interaction.entity.is_none();
        let same = |using: &ClientUse| Some(using.item) == held && using.slot == slot;
        match (self.vitals.using, consumable) {
            (Some(using), _) if !holding || !same(&using) => {
                self.vitals.using = None;
                Some(WorldAction::StopUse)
            }
            (None, Some((item, consumable))) if input.just_pressed(Action::Use) => {
                let sound =
                    (consumable.sound.as_ref()).and_then(|id| content.sounds.registry.key(id));
                self.vitals.using = Some(ClientUse {
                    item,
                    slot,
                    held: 0.0,
                    needed: f32::from(consumable.ticks) / 20.0,
                    sound,
                    since_sound: 0.0,
                });
                Some(WorldAction::StartUse {
                    item,
                    slot: u8::try_from(slot).unwrap_or(u8::MAX),
                })
            }
            _ => None,
        }
    }

    /// The bites, sips and hits of this frame.
    pub fn play_vitals(&mut self, audio: &mut Audio) {
        let at = self.camera.position;
        for sound in self.vitals.sounds.drain(..) {
            audio.play(Cue {
                sound,
                at,
                subtitled: false,
            });
        }
    }
}
