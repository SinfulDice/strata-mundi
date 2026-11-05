//! Entity types (`entities/<name>.ron`) and the box models that draw them
//! (`models/<name>.ron`).

use super::registry::{EntityTypeId, Registry, TextureId};
use super::textures::TextureLibrary;
use crate::id::Identifier;
use crate::pack::{PackError, PackSet, Sourced};
use crate::physics::{BoatHandling, BoxSize};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// Largest model coordinate, in sixteenths of a block from the entity's origin.
const MAX_MODEL_EXTENT: i32 = 64;
const MAX_BOXES: usize = 64;

/// An entity type as written in the pack.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Entity", deny_unknown_fields)]
pub struct EntityDef {
    pub model: Identifier,
    /// Texture painted on every face of the model (repeated every block).
    pub texture: Identifier,
    /// Collision box: square footprint width and height, in blocks.
    pub size: (f64, f64),
    /// Makes it a boat with this handling.
    #[serde(default)]
    pub boat: Option<BoatHandling>,
}

/// `models/<name>.ron`: boxes in sixteenths of a block, around the entity's origin
/// (bottom centre); the model's front is towards -Z (north at yaw 0).
#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "Model", deny_unknown_fields)]
struct ModelDef {
    boxes: Vec<ModelBox>,
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct ModelBox {
    pub from: (i32, i32, i32),
    pub to: (i32, i32, i32),
}

impl ModelBox {
    fn is_valid(&self) -> bool {
        let (from, to) = (
            [self.from.0, self.from.1, self.from.2],
            [self.to.0, self.to.1, self.to.2],
        );
        from.iter()
            .zip(&to)
            .all(|(&a, &b)| a < b && a.abs() <= MAX_MODEL_EXTENT && b.abs() <= MAX_MODEL_EXTENT)
    }
}

/// A loaded entity type.
#[derive(Debug, Clone, PartialEq)]
pub struct EntityType {
    pub boxes: Vec<ModelBox>,
    pub texture: TextureId,
    pub size: BoxSize,
    pub boat: Option<BoatHandling>,
}

pub(super) fn load_entities(
    packs: &PackSet,
    textures: &TextureLibrary,
    errors: &mut Vec<PackError>,
) -> Registry<EntityTypeId, EntityType> {
    let models = load_models(packs, errors);
    let mut entities = Registry::default();
    for (id, Sourced { value, source }) in packs.layered::<EntityDef>("entities", errors) {
        let Some(entity) = entity_type(value, &models, textures, &source, errors) else {
            continue;
        };
        if entities.insert(id, entity).is_none() {
            errors.push(PackError::invalid(&source, "too many entity types"));
        }
    }
    entities
}

fn entity_type(
    def: EntityDef,
    models: &BTreeMap<Identifier, Vec<ModelBox>>,
    textures: &TextureLibrary,
    source: &Path,
    errors: &mut Vec<PackError>,
) -> Option<EntityType> {
    let Some(boxes) = models.get(&def.model) else {
        errors.push(PackError::invalid(
            source,
            format!("unknown model `{}`", def.model),
        ));
        return None;
    };
    let (width, height) = def.size;
    let sized = |v: f64| v.is_finite() && (0.1..=4.0).contains(&v);
    if !sized(width) || !sized(height) {
        errors.push(PackError::invalid(
            source,
            "size must be between 0.1 and 4 blocks",
        ));
        return None;
    }
    let texture = textures.layer(&def.texture).unwrap_or_else(|| {
        errors.push(PackError::invalid(
            source,
            format!("unknown texture `{}`", def.texture),
        ));
        TextureId::MISSING
    });
    Some(EntityType {
        boxes: boxes.clone(),
        texture,
        size: BoxSize { width, height },
        boat: def
            .boat
            .filter(|handling| valid_handling(handling, source, errors)),
    })
}

fn valid_handling(handling: &BoatHandling, source: &Path, errors: &mut Vec<PackError>) -> bool {
    let valid = [
        handling.thrust,
        handling.reverse_thrust,
        handling.turn_speed,
        handling.draft,
    ]
    .iter()
    .all(|v| v.is_finite() && (0.0..=10.0).contains(v))
        && (0.0..1.0).contains(&handling.water_momentum)
        && handling.seat.is_finite()
        && handling.seat.abs() <= 2.0;
    if !valid {
        errors.push(PackError::invalid(
            source,
            "boat: speeds and draft between 0 and 10, water_momentum below 1, seat within 2",
        ));
    }
    valid
}

fn load_models(
    packs: &PackSet,
    errors: &mut Vec<PackError>,
) -> BTreeMap<Identifier, Vec<ModelBox>> {
    let mut models = BTreeMap::new();
    for (id, Sourced { value, source }) in packs.layered::<ModelDef>("models", errors) {
        let valid = !value.boxes.is_empty()
            && value.boxes.len() <= MAX_BOXES
            && value.boxes.iter().all(ModelBox::is_valid);
        if valid {
            models.insert(id, value.boxes);
        } else {
            errors.push(PackError::invalid(
                &source,
                format!(
                    "a model has 1 to {MAX_BOXES} boxes, `from` below `to`, \
                     coordinates within ±{MAX_MODEL_EXTENT}"
                ),
            ));
        }
    }
    models
}
