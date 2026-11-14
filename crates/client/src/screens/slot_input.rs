//! Mouse and gamepad on inventory slots: take and put (also by dragging from one slot to
//! another), halves, transfers, gathering, throwing and locks.

use super::Screens;
use super::menu::Menu;
use super::slots::SlotsState;
use crate::gameplay::Gameplay;
use crate::input::{Action, InputState};
use crate::ui::{Hit, HitKind, SlotPress};
use strata_core::content::Content;
use strata_core::inventory::Click;

/// Two clicks on the same slot closer than this (seconds) gather.
const DOUBLE_CLICK: f32 = 0.3;

/// What the pointer did lately on slots.
#[derive(Debug, Default)]
pub struct SlotInput {
    /// Slot where the button went down: releasing it over another slot puts there.
    drag: Option<Hit>,
    /// Last click: slot and time, for double clicks.
    last: Option<(String, usize, f32)>,
    clock: f32,
    /// An inventory screen was open last frame (closing it gives the carried stack back).
    was_open: bool,
}

impl Screens {
    /// The inventory key: every item in creative, the player's slots otherwise.
    pub(super) fn inventory_menu(game: Option<&Gameplay>) -> Menu {
        match game {
            Some(game) if !game.is_creative() => Menu::Slots(SlotsState::default()),
            _ => Menu::Inventory(Default::default()),
        }
    }

    /// Keeps inventory screens in step with the server: a newly opened container shows,
    /// a container gone closes its screen, a closed screen gives the carried stack back.
    pub(super) fn follow_inventory(&mut self, game: Option<&mut Gameplay>) {
        let Some(game) = game else {
            self.slot_input = SlotInput::default();
            return;
        };
        if std::mem::take(&mut game.container_opened) {
            let shown =
                matches!(self.stack.last(), Some(Menu::Slots(state)) if state.for_container);
            if !shown {
                self.stack.push(Menu::Slots(SlotsState {
                    for_container: true,
                    ..SlotsState::default()
                }));
                self.reset_focus();
            }
        }
        let gone = matches!(self.stack.last(), Some(Menu::Slots(state)) if state.for_container)
            && game.container.is_none();
        if gone {
            self.stack.pop();
            self.reset_focus();
        }
        let open = self.stack.iter().any(|menu| matches!(menu, Menu::Slots(_)));
        if self.slot_input.was_open && !open {
            game.hotbar
                .request(strata_protocol::InventoryRequest::Close);
            game.container = None;
        }
        self.slot_input.was_open = open;
    }

    /// Slot input of an inventory screen; returns whether the input was used.
    pub(super) fn handle_slots(
        &mut self,
        input: &InputState,
        mut game: Option<&mut Gameplay>,
        content: &Content,
        cursor: Option<(i32, i32)>,
        frame_seconds: f32,
    ) -> bool {
        if !matches!(self.stack.last(), Some(Menu::Slots(_))) {
            return false;
        }
        self.slot_input.clock += frame_seconds;
        let hovered = cursor
            .and_then(|(x, y)| self.hit_at(x, y))
            .filter(|hit| hit.kind == HitKind::Slot);
        if let Some(from) = self
            .slot_input
            .drag
            .clone()
            .filter(|_| !input.is_held(Action::Attack))
        {
            self.slot_input.drag = None;
            if let Some(to) = hovered
                .clone()
                .filter(|to| (&to.id, to.index) != (&from.id, from.index))
            {
                return self.press_slot(game, content, &to, SlotPress::Click(Click::Primary));
            }
        }
        let outside = cursor.is_some_and(|(x, y)| self.window.is_some_and(|w| !w.contains(x, y)));
        for (action, whole) in [(Action::Attack, true), (Action::Use, false)] {
            if !input.just_pressed(action) {
                continue;
            }
            if let Some(hit) = &hovered {
                let click = self.mouse_click(input, hit, action);
                return self.press_slot(game.as_deref_mut(), content, hit, SlotPress::Click(click));
            }
            if outside {
                let drop = SlotPress::Click(Click::DropCursor { whole });
                return self.press_slot(game, content, &first_slot(), drop);
            }
        }
        let target = hovered.or_else(|| self.focus.clone().filter(|f| f.kind == HitKind::Slot));
        let Some(target) = target else {
            return false;
        };
        let press = if input.just_pressed(Action::MenuSecondary) {
            SlotPress::Click(Click::Secondary)
        } else if input.just_pressed(Action::MenuTransfer) {
            SlotPress::Click(Click::Transfer)
        } else if input.just_pressed(Action::MenuGather) {
            SlotPress::Click(Click::Gather)
        } else if input.just_pressed(Action::MenuDrop) {
            SlotPress::Click(Click::Drop { whole: false })
        } else if input.just_pressed(Action::Drop) {
            SlotPress::Click(Click::Drop {
                whole: input.is_held(Action::Sprint),
            })
        } else if input.just_pressed(Action::MenuLock) {
            SlotPress::ToggleLock
        } else {
            return false;
        };
        self.press_slot(game, content, &target, press)
    }

    fn mouse_click(&mut self, input: &InputState, hit: &Hit, action: Action) -> Click {
        if action == Action::Use {
            return Click::Secondary;
        }
        if input.is_held(Action::Sneak) {
            return Click::Transfer;
        }
        let now = self.slot_input.clock;
        let double = self
            .slot_input
            .last
            .as_ref()
            .is_some_and(|(id, index, at)| {
                *id == hit.id && *index == hit.index && now - at <= DOUBLE_CLICK
            });
        if double {
            self.slot_input.last = None;
            return Click::Gather;
        }
        self.slot_input.last = Some((hit.id.clone(), hit.index, now));
        self.slot_input.drag = Some(hit.clone());
        Click::Primary
    }

    fn press_slot(
        &mut self,
        game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
        press: SlotPress,
    ) -> bool {
        self.focus = Some(hit.clone());
        let used = self.with_model(game, content, |m| m.slot_press(&hit.id, hit.index, press));
        used.unwrap_or(false)
    }

    /// Whether the inventory key closes the screen on top (on a gamepad, the same button
    /// sends stacks across in an inventory screen).
    pub(super) fn closes_inventory(&self, input: &InputState) -> bool {
        let inventory = matches!(self.stack.last(), Some(Menu::Inventory(_) | Menu::Slots(_)));
        inventory
            && input.just_pressed(Action::Inventory)
            && !input.just_pressed(Action::MenuTransfer)
    }
}

/// Any slot of the player: clicks outside the window only need one to name.
fn first_slot() -> Hit {
    Hit {
        id: "hotbar".to_owned(),
        index: 0,
        rect: crate::ui::Rect::new(0, 0, 0, 0),
        kind: HitKind::Slot,
    }
}
