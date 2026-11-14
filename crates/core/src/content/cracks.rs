//! `cracks.ron` at the root of a pack: the textures drawn, in order, over a block being
//! broken. The last pack that has the file wins (a resource pack may restyle it).

use super::registry::TextureId;
use super::textures::TextureLibrary;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, read_ron};
use serde::Deserialize;

/// More stages than this would not be told apart.
pub const MAX_CRACK_STAGES: usize = 32;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Cracks", deny_unknown_fields)]
pub struct CracksDef {
    pub stages: Vec<Identifier>,
}

pub(super) fn load_cracks(
    packs: &PackSet,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> Vec<TextureId> {
    let mut stages = Vec::new();
    for pack in packs.packs() {
        let path = pack.root.join("cracks.ron");
        if !path.is_file() {
            continue;
        }
        let def = match read_ron::<CracksDef>(&path) {
            Ok(def) => def,
            Err(error) => {
                errors.push(error);
                continue;
            }
        };
        if def.stages.len() > MAX_CRACK_STAGES {
            let message = format!("{MAX_CRACK_STAGES} stages at most");
            errors.push(PackError::invalid(&path, message));
        }
        stages = def
            .stages
            .iter()
            .take(MAX_CRACK_STAGES)
            .map(|id| {
                textures.layer(id).unwrap_or_else(|| {
                    errors.push(PackError::invalid(&path, format!("unknown texture `{id}`")));
                    TextureId::MISSING
                })
            })
            .collect();
    }
    stages
}
