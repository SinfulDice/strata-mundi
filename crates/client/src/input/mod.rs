//! Unified input: keyboard/mouse, gamepad and (later) touch all drive abstract actions.

mod action;
mod bindings;
mod gamepad;
mod state;

pub use action::Action;
pub use bindings::{Binding, Bindings, PadButton};
pub use gamepad::Gamepads;
pub use state::{InputDevice, InputState};
