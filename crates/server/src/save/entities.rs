//! `entities/<namespace>/<dimension>.ron`: the entities of one dimension (boats, items
//! on the ground).
//! A world saved before entities existed simply has no such file.

use super::stacks::SavedStack;
use super::{SaveError, WorldFolder, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strata_core::dimension::DimensionId;
use strata_core::id::Identifier;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Entities")]
pub struct EntitiesData {
    #[serde(default)]
    pub boats: Vec<SavedBoat>,
    #[serde(default)]
    pub items: Vec<SavedDrop>,
}

/// Items lying on the ground, with their age (they keep ageing after a reload).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Item")]
pub struct SavedDrop {
    pub stack: SavedStack,
    pub position: [f64; 3],
    #[serde(default)]
    pub age: u32,
}

/// A boat by type identifier, so a world survives packs being added or reordered.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "Boat")]
pub struct SavedBoat {
    pub kind: Identifier,
    /// Bottom centre.
    pub position: [f64; 3],
    #[serde(default)]
    pub yaw: f32,
}

impl WorldFolder {
    fn entities_path(&self, dimension: &DimensionId) -> PathBuf {
        let id = &dimension.0;
        self.dir
            .join("entities")
            .join(id.namespace())
            .join(format!("{}.ron", id.name()))
    }

    /// Nothing saved yet reads as no entity.
    pub fn read_entities(&self, dimension: &DimensionId) -> Result<EntitiesData, SaveError> {
        let path = self.entities_path(dimension);
        if !path.is_file() {
            return Ok(EntitiesData::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| SaveError::io(&path, e))?;
        ron::from_str(&text).map_err(|e| SaveError::format(&path, e))
    }

    pub fn write_entities(
        &self,
        dimension: &DimensionId,
        data: &EntitiesData,
    ) -> Result<(), SaveError> {
        let path = self.entities_path(dimension);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SaveError::io(parent, e))?;
        }
        let text = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
            .map_err(|e| SaveError::format(&path, e))?;
        write_atomic(&path, text.as_bytes()).map_err(|e| SaveError::io(&path, e))
    }
}
