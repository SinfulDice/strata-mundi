use crate::id::Identifier;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifier of a dimension, e.g. `strata:terra`, defined in `dimensions/` of a pack.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DimensionId(pub Identifier);

impl fmt::Display for DimensionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
