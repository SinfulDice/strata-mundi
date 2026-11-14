//! `block_entities/<namespace>/<dimension>.ron`: what container blocks hold, by position.
//! Kept apart from the region files, which store only block ids (see D063).

use super::stacks::SavedStack;
use super::{SaveError, WorldFolder, write_atomic};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strata_core::dimension::DimensionId;
use strata_core::inventory::ItemData;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename = "BlockEntities")]
pub struct BlockEntitiesData {
    #[serde(default)]
    pub entities: Vec<SavedBlockEntity>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename = "BlockEntity")]
pub struct SavedBlockEntity {
    pub pos: [i32; 3],
    pub slots: Vec<Option<SavedStack>>,
    #[serde(default)]
    pub data: ItemData,
}

impl WorldFolder {
    fn block_entities_path(&self, dimension: &DimensionId) -> PathBuf {
        let id = &dimension.0;
        self.dir
            .join("block_entities")
            .join(id.namespace())
            .join(format!("{}.ron", id.name()))
    }

    /// Nothing saved yet (a world from before v0.3) reads as none.
    pub fn read_block_entities(
        &self,
        dimension: &DimensionId,
    ) -> Result<BlockEntitiesData, SaveError> {
        let path = self.block_entities_path(dimension);
        if !path.is_file() {
            return Ok(BlockEntitiesData::default());
        }
        let text = std::fs::read_to_string(&path).map_err(|e| SaveError::io(&path, e))?;
        ron::from_str(&text).map_err(|e| SaveError::format(&path, e))
    }

    pub fn write_block_entities(
        &self,
        dimension: &DimensionId,
        data: &BlockEntitiesData,
    ) -> Result<(), SaveError> {
        let path = self.block_entities_path(dimension);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| SaveError::io(parent, e))?;
        }
        let text = ron::ser::to_string_pretty(data, ron::ser::PrettyConfig::default())
            .map_err(|e| SaveError::format(&path, e))?;
        write_atomic(&path, text.as_bytes()).map_err(|e| SaveError::io(&path, e))
    }
}
