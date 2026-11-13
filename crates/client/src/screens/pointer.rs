//! Values changed with the pointer (click on a track or an arrow, drag, wheel) and with
//! arrows held down (keys, d-pad or stick), on sliders and options rows.

use super::Screens;
use super::handling::MOVES;
use crate::gameplay::Gameplay;
use crate::input::{Action, InputState};
use crate::ui::{
    Hit, HitKind, HoldRepeat, OptionRow, RowPart, SLIDER_KNOB, ScreenModel, row_part, row_zones,
    track_fraction, wheel_adjusts,
};
use strata_core::content::Content;

/// How far the stick must lean to count as an arrow in menus.
const STICK_PRESS: f32 = 0.5;

/// -1, 0 or 1 on each axis (x right, y down): the arrows (or stick) held this frame.
pub fn held_arrows(input: &InputState) -> (i32, i32) {
    let key = |action| i32::from(input.is_held(action));
    let stick = input.move_stick();
    let lean = |v: f32| i32::from(v >= STICK_PRESS) - i32::from(v <= -STICK_PRESS);
    let x = key(Action::MenuRight) - key(Action::MenuLeft) + lean(stick.x);
    let y = key(Action::MenuDown) - key(Action::MenuUp) - lean(stick.y);
    (x.signum(), y.signum())
}

/// -1, 0 or 1 on each axis: the arrows pressed this frame.
fn tapped_arrows(input: &InputState) -> (i32, i32) {
    let mut tapped = (0, 0);
    for (action, direction) in MOVES {
        if input.just_pressed(action) {
            let (dx, dy) = direction.offset();
            tapped = (tapped.0 + dx, tapped.1 + dy);
        }
    }
    tapped
}

/// Steps of one axis this frame: repeats while held, or a tap shorter than a frame.
fn repeated(repeat: &mut HoldRepeat, held: i32, tapped: i32, frame_seconds: f32) -> i32 {
    let steps = repeat.update(held, frame_seconds);
    if held == 0 { tapped } else { steps }
}

/// Whether holding ← → repeats on this element: sliders and gauges, not toggles or lists
/// of choices (one press, one change).
pub fn repeats(model: &dyn ScreenModel, hit: &Hit) -> bool {
    match hit.kind {
        HitKind::Slider => true,
        HitKind::Option => option_row(model, hit).is_some_and(|row| row.gauge.is_some()),
        _ => false,
    }
}

fn option_row(model: &dyn ScreenModel, hit: &Hit) -> Option<OptionRow> {
    model.option_rows(&hit.id).get(hit.index).cloned()
}

/// ← → on the focused value: the element the repeat belongs to, restarted when the focus
/// moves.
#[derive(Debug, Default)]
pub struct ValueRepeat {
    repeat: HoldRepeat,
    target: Option<(String, usize)>,
}

impl ValueRepeat {
    /// Steps for `focus` this frame; `repeats`: whether holding repeats on it.
    pub fn steps(
        &mut self,
        input: &InputState,
        frame_seconds: f32,
        focus: &Hit,
        repeats: bool,
    ) -> i32 {
        let target = Some((focus.id.clone(), focus.index));
        if self.target != target {
            // A new element waits the whole delay if an arrow is still held; only a
            // fresh press steps at once.
            self.repeat.reset();
            self.repeat.update(held_arrows(input).0, 0.0);
            self.target = target;
            return tapped_arrows(input).0;
        }
        let tapped = tapped_arrows(input).0;
        if !repeats {
            self.repeat.reset();
            return tapped;
        }
        repeated(
            &mut self.repeat,
            held_arrows(input).0,
            tapped,
            frame_seconds,
        )
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Moves of a HUD element this frame, in steps per axis (held arrows repeat).
pub fn editor_moves(
    repeats: &mut [HoldRepeat; 2],
    input: &InputState,
    frame_seconds: f32,
) -> (i32, i32) {
    let (held, tapped) = (held_arrows(input), tapped_arrows(input));
    (
        repeated(&mut repeats[0], held.0, tapped.0, frame_seconds),
        repeated(&mut repeats[1], held.1, tapped.1, frame_seconds),
    )
}

/// What a click did to a value.
#[derive(Debug, Clone, PartialEq)]
pub enum Click {
    /// Not on a value: the element's usual action applies.
    Ignored,
    /// One step through an arrow.
    Stepped,
    /// Set to the value under the pointer; the drag follows this track.
    Drag(Hit),
}

/// A click at `cursor` on a slider or an options row.
pub fn click_value(model: &mut dyn ScreenModel, hit: &Hit, cursor: (i32, i32)) -> Click {
    let (track, fraction) = match hit.kind {
        HitKind::Slider => (hit.rect, track_fraction(hit.rect, SLIDER_KNOB, cursor.0)),
        HitKind::Option => {
            let Some(row) = option_row(model, hit) else {
                return Click::Ignored;
            };
            match row_part(hit.rect, &row, cursor) {
                RowPart::Gauge(fraction) => (row_zones(hit.rect).gauge, fraction),
                RowPart::Step(step) => {
                    model.adjust(&hit.id, hit.index, step, None);
                    return Click::Stepped;
                }
                RowPart::Body => return Click::Ignored,
            }
        }
        _ => return Click::Ignored,
    };
    model.adjust(&hit.id, hit.index, 0, Some(fraction));
    Click::Drag(Hit {
        rect: track,
        ..hit.clone()
    })
}

/// The dragged value follows the pointer at `x`, even outside its track.
pub fn drag_value(model: &mut dyn ScreenModel, drag: &Hit, x: i32) {
    let knob = match drag.kind {
        HitKind::Slider => SLIDER_KNOB,
        _ => 1,
    };
    model.adjust(
        &drag.id,
        drag.index,
        0,
        Some(track_fraction(drag.rect, knob, x)),
    );
}

/// The wheel over a slider or a row's value changes it; returns whether it did.
pub fn wheel_value(model: &mut dyn ScreenModel, hit: &Hit, cursor: (i32, i32), steps: i32) -> bool {
    let adjusts = match hit.kind {
        HitKind::Slider => true,
        HitKind::Option => {
            option_row(model, hit).is_some_and(|row| wheel_adjusts(hit.rect, &row, cursor))
        }
        _ => false,
    };
    if adjusts {
        model.adjust(&hit.id, hit.index, steps, None);
    }
    adjusts
}

impl Screens {
    /// ← → for the focused value this frame (taps, and repeats on sliders and gauges).
    pub(super) fn value_steps(
        &mut self,
        input: &InputState,
        frame_seconds: f32,
        game: Option<&mut Gameplay>,
        content: &Content,
    ) -> i32 {
        let Some(focus) = self
            .focus
            .clone()
            .filter(|f| matches!(f.kind, HitKind::Slider | HitKind::Option))
        else {
            self.repeat.reset();
            return 0;
        };
        let repeat_here = self
            .with_model(game, content, |m| repeats(m, &focus))
            .unwrap_or(false);
        self.repeat.steps(input, frame_seconds, &focus, repeat_here)
    }

    /// A click on a value; `false`: the element's usual action applies instead.
    pub(super) fn click_on_value(
        &mut self,
        game: Option<&mut Gameplay>,
        content: &Content,
        hit: &Hit,
        cursor: (i32, i32),
    ) -> bool {
        let click = self
            .with_model(game, content, |m| click_value(m, hit, cursor))
            .unwrap_or(Click::Ignored);
        match click {
            Click::Ignored => false,
            Click::Stepped => true,
            Click::Drag(track) => {
                self.dragging = Some(track);
                true
            }
        }
    }

    /// While the button is held, the dragged value follows the pointer.
    pub(super) fn follow_drag(
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
            self.with_model(game, content, |m| drag_value(m, &drag, x));
        }
    }

    /// The wheel over a value changes it; returns whether it did (else it scrolls).
    pub(super) fn wheel_on_value(
        &mut self,
        game: Option<&mut Gameplay>,
        content: &Content,
        cursor: Option<(i32, i32)>,
        steps: i32,
    ) -> bool {
        let Some((cursor, hit)) = cursor.and_then(|(x, y)| Some(((x, y), self.hit_at(x, y)?)))
        else {
            return false;
        };
        self.with_model(game, content, |m| wheel_value(m, &hit, cursor, steps))
            .unwrap_or(false)
    }

    /// A list scrolled: a row dragged by the mouse now stands for another setting.
    pub(super) fn drop_row_drag(&mut self) {
        if self
            .dragging
            .as_ref()
            .is_some_and(|d| d.kind == HitKind::Option)
        {
            self.dragging = None;
        }
    }
}

#[cfg(test)]
#[path = "pointer_tests.rs"]
mod tests;
