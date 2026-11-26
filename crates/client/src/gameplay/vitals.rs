//! The survival gauges as the server tells them, and what the player feels: the red
//! flash and the shake of a hit, the notice after fainting, eating with the use button
//! held (progress and sounds).

use super::Gameplay;
use crate::audio::{Audio, Cue};
use crate::input::{Action, InputState};
use crate::interaction::aim_source;
use std::cell::RefCell;
use strata_core::content::{Content, ItemId, SoundId};
use strata_core::survival::temperature::TemperatureZone;
use strata_protocol::{HurtView, VitalsView, WorldAction};

/// Seconds the red flash and the shake last.
const HURT_SECONDS: f32 = 0.4;
/// Seconds the notice after fainting stays.
const FAINT_SECONDS: f32 = 6.0;
/// Seconds between two bites or sips heard while using an item.
const USE_SOUND_EVERY: f32 = 0.25;
/// Strongest shake, in radians (at 100 % and a heavy hit).
const MAX_SHAKE: f32 = 0.06;
/// Seconds the temperature hint stays once the zone gets worse.
const HINT_SECONDS: f32 = 8.0;
/// Seconds for the frost or heat at the edges to come or go.
const EDGE_SECONDS: f32 = 1.5;

#[derive(Debug, Default)]
pub struct ClientVitals {
    pub view: Option<VitalsView>,
    /// Seconds left of the last hit's flash and shake, and how strong it was (0 to 1).
    pub hurt: f32,
    hurt_strength: f32,
    hurt_shakes: bool,
    /// Seconds left of the fainting notice.
    pub fainted: f32,
    /// The item and hotbar slot the use button was pressed on (to say when it stops).
    pressing: Option<(ItemId, usize)>,
    /// Seconds since the last bite or sip heard.
    since_sound: f32,
    sounds: Vec<SoundId>,
    /// The effects line of the HUD in a language, made again when the gauges change.
    effects_line: RefCell<Option<(String, Option<String>)>>,
    /// The zone the temperature got worse into, and the seconds its hint has left.
    pub hint: Option<(TemperatureZone, f32)>,
    /// Frost (below 0, to -1) or heat (above 0, to 1) at the edges of the screen.
    pub edges: f32,
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
    }

    /// How far the bite or sip the server counts is, 0 to 1.
    pub fn use_progress(&self, content: &Content) -> Option<f32> {
        let (item, left) = self.view.as_ref()?.using?;
        let needed = content.survival.consumables.get(&item)?.ticks.max(1);
        Some(1.0 - f32::from(left.min(needed)) / f32::from(needed))
    }

    /// The effects line in `language`, made by `make` only when the gauges changed.
    pub fn effects_line(
        &self,
        language: &str,
        make: impl FnOnce() -> Option<String>,
    ) -> Option<String> {
        let mut cached = self.effects_line.borrow_mut();
        match cached.as_ref() {
            Some((known, line)) if known == language => line.clone(),
            _ => {
                let line = make();
                *cached = Some((language.to_owned(), line.clone()));
                line
            }
        }
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

    /// A new temperature zone: a hint when it gets further from comfort.
    fn temperature_changed(&mut self, before: TemperatureZone, now: TemperatureZone) {
        let worse = now.severity() > before.severity()
            || (now.severity() > 0
                && (now > TemperatureZone::Comfortable) != (before > TemperatureZone::Comfortable));
        if worse {
            self.hint = Some((now, HINT_SECONDS));
        } else if now == TemperatureZone::Comfortable {
            self.hint = None;
        }
    }

    fn advance_temperature(&mut self, dt: f32) {
        if let Some((_, left)) = &mut self.hint {
            *left -= dt;
        }
        self.hint = self.hint.filter(|(_, left)| *left > 0.0);
        let zone = self
            .view
            .as_ref()
            .and_then(|v| v.temperature)
            .map(|t| t.zone);
        let target = match zone {
            Some(TemperatureZone::Freezing) => -1.0,
            Some(TemperatureZone::Cold) => -0.45,
            Some(TemperatureZone::Hot) => 0.4,
            Some(TemperatureZone::Scorching) => 1.0,
            _ => 0.0,
        };
        let step = dt / EDGE_SECONDS;
        self.edges += (target - self.edges).clamp(-step, step);
    }

    pub(super) fn advance(&mut self, dt: f32, content: &Content) {
        self.advance_temperature(dt);
        self.hurt = (self.hurt - dt).max(0.0);
        self.fainted = (self.fainted - dt).max(0.0);
        let using = self.view.as_ref().and_then(|view| view.using);
        let Some((item, _)) = using else {
            self.since_sound = USE_SOUND_EVERY;
            return;
        };
        self.since_sound += dt;
        if self.since_sound >= USE_SOUND_EVERY {
            self.since_sound = 0.0;
            let sound = (content.survival.consumables.get(&item))
                .and_then(|c| c.sound.as_ref())
                .and_then(|id| content.sounds.registry.key(id));
            self.sounds.extend(sound);
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
        let zone = |view: Option<&VitalsView>| {
            (view.and_then(|v| v.temperature)).map_or(TemperatureZone::Comfortable, |t| t.zone)
        };
        let before = zone(self.vitals.view.as_ref());
        self.vitals.temperature_changed(before, zone(Some(&view)));
        self.vitals.view = Some(view);
        self.vitals.effects_line.replace(None);
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
    /// (the server counts the time and says how far it is); letting go, or another item,
    /// stops.
    pub(super) fn use_consumable(&mut self, input: &InputState) -> Option<WorldAction> {
        let content = &self.info.content;
        let slot = self.inventory.selected_index();
        let held = self.inventory.selected();
        let consumable = held.filter(|item| content.survival.consumables.contains_key(item));
        let holding = input.is_held(Action::Use) && self.interaction.entity.is_none();
        match (self.vitals.pressing, consumable) {
            (Some(pressed), _) if !holding || Some(pressed) != held.map(|item| (item, slot)) => {
                self.vitals.pressing = None;
                Some(WorldAction::StopUse)
            }
            (None, Some(item)) if input.just_pressed(Action::Use) => {
                self.vitals.pressing = Some((item, slot));
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
