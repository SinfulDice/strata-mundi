//! How each kind of place rings (the reverberation the client puts on the sounds around
//! the listener), as written in `spaces/<name>.ron`: `outdoors`, `interior`,
//! `small_cave` and `large_cavern`.

use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use serde::Deserialize;
use std::collections::BTreeMap;

/// Highest comb feedback: above, the tail would hardly die.
pub const MAX_FEEDBACK: f32 = 0.97;

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(rename = "Space", deny_unknown_fields)]
pub struct SpaceDef {
    /// Length of the tail, 0 to `MAX_FEEDBACK`.
    pub feedback: f32,
    /// High frequencies lost at each echo, 0 to 1.
    pub damping: f32,
    /// Loudness of the reverberation, 0 (none) to 1.
    pub wet: f32,
}

impl SpaceDef {
    /// No reverberation (a place the packs leave out).
    pub const DRY: Self = Self {
        feedback: 0.7,
        damping: 0.5,
        wet: 0.0,
    };

    fn bound(&mut self, problems: &mut Vec<String>) {
        let mut clamp = |name: &str, value: &mut f32, high: f32| {
            let bounded = value.clamp(0.0, high);
            if bounded != *value || !value.is_finite() {
                problems.push(format!("`{name}` must be between 0 and {high}"));
                *value = if value.is_finite() { bounded } else { 0.0 };
            }
        };
        clamp("feedback", &mut self.feedback, MAX_FEEDBACK);
        clamp("damping", &mut self.damping, 1.0);
        clamp("wet", &mut self.wet, 1.0);
    }
}

pub(super) fn load_spaces(
    packs: &PackSet,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, SpaceDef> {
    packs
        .layered::<SpaceDef>("spaces", errors)
        .into_iter()
        .map(|(id, Sourced { mut value, source })| {
            let mut problems = Vec::new();
            value.bound(&mut problems);
            for problem in problems {
                errors.push(PackError::invalid(&source, problem));
            }
            (id, value)
        })
        .collect()
}
