//! Values changed with the pointer (click on a track or an arrow, drag, wheel) and with
//! ← → held down (keys, d-pad or stick), on sliders and options rows.

use super::Screens;
use crate::gameplay::Gameplay;
use crate::input::{Action, InputState};
use crate::ui::{
    Hit, HitKind, OptionRow, RowPart, SLIDER_KNOB, row_part, row_zones, track_fraction,
    wheel_adjusts,
};
use strata_core::content::Content;

/// How far the stick must lean to count as ← or → in menus.
const STICK_PRESS: f32 = 0.5;

/// -1, 0 or 1: the direction ← → is held this frame.
fn held_direction(input: &InputState) -> i32 {
    let stick = input.move_stick().x;
    let keys =
        i32::from(input.is_held(Action::MenuRight)) - i32::from(input.is_held(Action::MenuLeft));
    let stick = i32::from(stick >= STICK_PRESS) - i32::from(stick <= -STICK_PRESS);
    (keys + stick).signum()
}

/// Width of the knob that follows the pointer on a dragged track.
fn knob(kind: HitKind) -> u32 {
    match kind {
        HitKind::Slider => SLIDER_KNOB,
        _ => 1,
    }
}

impl Screens {
    /// Steps for the focused value this frame: a tap, or the repeats of a held direction.
    pub(super) fn held_steps(&mut self, input: &InputState, frame_seconds: f32) -> i32 {
        let held = held_direction(input);
        let steps = self.repeat.update(held, frame_seconds);
        if held == 0 {
            // Pressed and released within one frame.
            i32::from(input.just_pressed(Action::MenuRight))
                - i32::from(input.just_pressed(Action::MenuLeft))
        } else {
            steps
        }
    }

    /// A click on a slider or an options row: the value under the pointer (and a drag
    /// starts) or an arrow's step. `false`: the element's usual action applies instead.
    pub(super) fn click_value(
        &mut self,
        mut game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
        cursor: (i32, i32),
    ) -> bool {
        let (track, fraction) = match hit.kind {
            HitKind::Slider => (hit.rect, track_fraction(hit.rect, SLIDER_KNOB, cursor.0)),
            HitKind::Option => {
                let Some(row) = self.option_row(game.as_deref_mut(), content, hit) else {
                    return false;
                };
                match row_part(hit.rect, &row, cursor) {
                    RowPart::Gauge(fraction) => (row_zones(hit.rect).gauge, fraction),
                    RowPart::Step(step) => {
                        self.adjust(game, content, hit, step, None);
                        return true;
                    }
                    RowPart::Body => return false,
                }
            }
            _ => return false,
        };
        self.dragging = Some(Hit {
            rect: track,
            ..hit.clone()
        });
        self.adjust(game, content, hit, 0, Some(fraction));
        true
    }

    /// While the button is held, the dragged value follows the pointer, even outside it.
    pub(super) fn drag_value(
        &mut self,
        input: &InputState,
        cursor: Option<(i32, i32)>,
        game: Option<&mut Gameplay>,
        content: &Content,
    ) {
        if !input.is_held(Action::Attack) {
            self.dragging = None;
            return;
        }
        let Some((drag, (x, _))) = self.dragging.clone().zip(cursor) else {
            return;
        };
        if matches!(drag.kind, HitKind::Slider | HitKind::Option) && input.cursor_moved() {
            let fraction = track_fraction(drag.rect, knob(drag.kind), x);
            self.adjust(game, content, &drag, 0, Some(fraction));
        }
    }

    /// The wheel over a slider or a row's value changes it; returns whether it did (else
    /// the wheel scrolls as usual).
    pub(super) fn wheel_value(
        &mut self,
        mut game: Option<&mut Gameplay>,
        content: &Content,
        cursor: Option<(i32, i32)>,
        steps: i32,
    ) -> bool {
        let Some((cursor, hit)) = cursor.and_then(|(x, y)| Some(((x, y), self.hit_at(x, y)?)))
        else {
            return false;
        };
        let adjusts = match hit.kind {
            HitKind::Slider => true,
            HitKind::Option => self
                .option_row(game.as_deref_mut(), content, &hit)
                .is_some_and(|row| wheel_adjusts(hit.rect, &row, cursor)),
            _ => false,
        };
        if adjusts {
            self.adjust(game, content, &hit, steps, None);
        }
        adjusts
    }

    fn option_row(
        &mut self,
        game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
    ) -> Option<OptionRow> {
        self.with_model(game, content, |m| {
            m.option_rows(&hit.id).get(hit.index).cloned()
        })
        .flatten()
    }

    fn adjust(
        &mut self,
        game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
        step: i32,
        fraction: Option<f32>,
    ) {
        self.with_model(game, content, |m| {
            m.adjust(&hit.id, hit.index, step, fraction)
        });
    }
}
