//! Reading `blocks/`: every block file becomes one block, or several variants (turned,
//! facing, attached, liquid levels).

use super::worked::WorkedRefs;
use super::*;
use crate::pack::{PackError, PackSet, Sourced};
use std::collections::BTreeMap;
use std::path::PathBuf;

/// What blocks may refer to that resource packs may restyle.
pub(super) struct Presented<'a> {
    pub(super) textures: &'a TextureLibrary,
    pub(super) particles: &'a Particles,
    pub(super) sounds: &'a Sounds,
}

/// What blocks name and only later content resolves: loot tables, worked blocks.
#[derive(Default)]
pub(super) struct BlockRefs {
    pub(super) loot: Vec<(Identifier, PathBuf)>,
    pub(super) worked: WorkedRefs,
}

pub(super) fn load_blocks(
    packs: &PackSet,
    presented: &Presented<'_>,
    refs: &mut BlockRefs,
    errors: &mut Vec<PackError>,
) -> (Registry<BlockId, Block>, BTreeMap<BlockId, Fluid>) {
    let Presented {
        textures,
        particles,
        sounds,
    } = *presented;
    let mut blocks = Registry::default();
    let mut fluids = Vec::new();
    blocks.insert(air_id(), Block::air());
    for (id, Sourced { value, source }) in packs.layered::<BlockDef>("blocks", errors) {
        if id == air_id() {
            errors.push(PackError::invalid(
                &source,
                "`strata:air` is built into the engine",
            ));
            continue;
        }
        if value.light_opacity > 15 || value.light_emission > 15 {
            errors.push(PackError::invalid(
                &source,
                "light_opacity and light_emission must be between 0 and 15",
            ));
        }
        let faces = Face::ALL.map(|face| match value.textures.face(face) {
            None => TextureId::MISSING,
            Some(texture) => textures.layer(texture).unwrap_or_else(|| {
                errors.push(PackError::invalid(
                    &source,
                    format!("unknown texture `{texture}`"),
                ));
                TextureId::MISSING
            }),
        });
        if let Some(set) = value
            .sound
            .as_ref()
            .filter(|set| !sounds.sets.contains_key(*set))
        {
            errors.push(PackError::invalid(
                &source,
                format!("unknown sound set `{set}`"),
            ));
        }
        let placement = value.placement;
        let ceiling_texture = attached::ceiling_layer(&placement, textures, &source, errors);
        if value.rooted && value.hanging {
            errors.push(PackError::invalid(
                &source,
                "a block cannot be both rooted and hanging",
            ));
        }
        let block_particles =
            particle::block_particles(&value.particles, particles, &source, &mut *errors);
        let fluid_def = value.fluid;
        let block = Block {
            solid: value.solid,
            render: value.render,
            light_opacity: value.light_opacity.min(15),
            light_emission: value.light_emission.min(15),
            hardness: value.hardness,
            tool: value.tool,
            loot: value
                .loot
                .inspect(|table| refs.loot.push((table.clone(), source.clone()))),
            faces,
            sound: value.sound,
            category: value.category,
            has_item: value.has_item,
            max_stack: value.max_stack.max(1),
            cull_same: value.cull_same,
            orientation: None,
            attachment: None,
            replaceable: value.replaceable,
            rooted: value.rooted,
            hanging: value.hanging,
            fluid: None,
            slipperiness: value.slipperiness.clamp(0.0, 0.99),
            flammability: value.flammability,
            fire: value.fire.map(FireDef::clamped),
            tint: value.tint,
            particles: block_particles,
            container: value.container.filter(|c| {
                let valid =
                    c.upgradable.is_some() || (1..=ContainerDef::MAX_ROWS).contains(&c.rows);
                if !valid {
                    errors.push(PackError::invalid(&source, "container: 1 to 6 rows"));
                }
                valid
            }),
            station: value
                .station
                .filter(|station| container::valid_station(station, &source, errors)),
            facing: None,
            furnace: value.furnace,
            worked: Vec::new(),
        };
        refs.worked.push((id.clone(), value.worked, source.clone()));
        let lit = facing::lit_look(&placement, textures, &source, errors);
        let inserted = match (fluid_def, &placement) {
            (Some(def), Placement::Fixed) => {
                let pending = fluid::insert_fluid(&mut blocks, &id, block, def, &source, errors);
                let inserted = pending.is_some();
                fluids.extend(pending);
                inserted
            }
            (Some(_), _) => false,
            (None, Placement::Fixed) => blocks.insert(id, block).is_some(),
            (None, Placement::Axis) => turned::insert_turned(&mut blocks, &id, block),
            (None, Placement::Facing { .. }) => facing::insert_facing(&mut blocks, &id, block, lit),
            (
                None,
                &Placement::Attached {
                    floor,
                    wall,
                    ceiling,
                    ..
                },
            ) => {
                let sides = AttachedSides {
                    floor,
                    wall,
                    ceiling,
                };
                attached::insert_attached(&mut blocks, &id, block, sides, ceiling_texture)
            }
        };
        if !inserted {
            errors.push(PackError::invalid(
                &source,
                "too many blocks (65 536 at most), a variant name (`__x`, `__north`, `__1`…) \
                 is taken, a fluid is turned or attached, or an attached block allows no side",
            ));
        }
    }
    let fluids = fluid::resolve(&blocks, fluids, errors);
    (blocks, fluids)
}
