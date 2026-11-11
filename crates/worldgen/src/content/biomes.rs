use super::Resolver;
use crate::defs::{BandsDef, BiomeColors, BiomeDef, Climate};
use std::path::Path;
use strata_core::chunk::BiomeId;
use strata_core::content::{BiomeTints, Block, BlockId, Registry, Tint};
use strata_core::id::Identifier;
use strata_core::pack::{PackError, PackSet, Sourced};

#[derive(Debug, Clone)]
pub struct Biome {
    pub id: Identifier,
    /// Position in `WorldgenContent::biomes`, as sent to clients.
    pub index: BiomeId,
    pub climate: Climate,
    pub surface: BlockId,
    pub subsurface: BlockId,
    pub subsurface_depth: u32,
    pub underwater: BlockId,
    /// Highest ground, above the sea level, still covered with `underwater`.
    pub shore_height: i32,
    /// Sorted by height.
    pub peaks: Vec<(i32, BlockId)>,
    /// Surface of steep slopes; `None` = the dimension's stone.
    pub steep: Option<BlockId>,
    pub frozen: bool,
    pub min_height: Option<i32>,
    pub max_height: Option<i32>,
    pub river: bool,
    /// First and last heat band where the biome may appear; `None` = any.
    pub heat: Option<(u8, u8)>,
    pub bands: Option<Bands>,
    pub map_color: [u8; 3],
    pub spawn: bool,
    /// `(feature index, attempts per column)`.
    pub features: Vec<(usize, f64)>,
    pub tints: BiomeTints,
}

impl Biome {
    pub fn allows_heat(&self, band: u8) -> bool {
        self.heat
            .is_none_or(|(first, last)| (first..=last).contains(&band))
    }

    /// Biomes whose heat bands are more than one apart (desert and tundra) never touch.
    pub fn may_touch(&self, other: &Self) -> bool {
        match (self.heat, other.heat) {
            (Some((a_first, a_last)), Some((b_first, b_last))) => {
                a_first <= b_last + 1 && b_first <= a_last + 1
            }
            _ => true,
        }
    }
}

/// Layers by height: `blocks[y % len]`, shifted by a little noise.
#[derive(Debug, Clone)]
pub struct Bands {
    pub blocks: Vec<BlockId>,
    pub depth: i32,
    pub waviness: f64,
}

pub(super) fn load_biomes(
    packs: &PackSet,
    blocks: &Registry<BlockId, Block>,
    errors: &mut Vec<PackError>,
    index_of: &dyn Fn(&Identifier) -> Option<usize>,
) -> Vec<Biome> {
    let defs = packs.layered::<BiomeDef>("biomes", errors);
    defs.into_iter()
        .enumerate()
        .filter_map(|(index, (id, Sourced { value, source }))| {
            let Ok(index) = u16::try_from(index) else {
                let message = "too many biomes (65 536 at most): this one is ignored";
                errors.push(PackError::invalid(&source, message));
                return None;
            };
            let features = value
                .features
                .iter()
                .filter_map(|placement| match index_of(&placement.feature) {
                    Some(index) => Some((index, placement.per_column.clamp(0.0, 64.0))),
                    None => {
                        let message = format!("unknown feature `{}`", placement.feature);
                        errors.push(PackError::invalid(&source, message));
                        None
                    }
                })
                .collect();
            let mut r = Resolver {
                blocks,
                errors,
                source: &source,
            };
            Some(Biome {
                id,
                index: BiomeId(index),
                climate: value.climate,
                surface: r.block(&value.surface),
                subsurface: r.block(&value.subsurface),
                subsurface_depth: value.subsurface_depth.min(16),
                underwater: r.block(&value.underwater),
                shore_height: value.shore_height.clamp(-8, 8),
                peaks: sorted_peaks(&value.peaks, &mut r),
                steep: value.steep.as_ref().map(|id| r.block(id)),
                frozen: value.frozen,
                min_height: value.min_height,
                max_height: value.max_height,
                river: value.river,
                heat: value.heat.map(|(a, b)| (a.min(b), a.max(b))),
                bands: value.bands.as_ref().and_then(|def| bands(def, &mut r)),
                map_color: {
                    let [r, g, b, _] = value.map_color.0.0;
                    [r, g, b]
                },
                spawn: value.spawn,
                features,
                tints: biome_tints(&value.colors),
            })
        })
        .collect()
}

fn biome_tints(colors: &BiomeColors) -> BiomeTints {
    BiomeTints {
        kinds: [colors.grass, colors.foliage, colors.water].map(Tint::from_factors),
        fog: Tint::from_factors(colors.fog),
    }
}

fn bands(def: &BandsDef, r: &mut Resolver<'_>) -> Option<Bands> {
    let blocks = r.blocks(&def.blocks);
    (!blocks.is_empty()).then(|| Bands {
        blocks,
        depth: def.depth.min(64) as i32,
        waviness: def.waviness.clamp(0.0, 16.0),
    })
}

fn sorted_peaks(peaks: &[(i32, Identifier)], r: &mut Resolver<'_>) -> Vec<(i32, BlockId)> {
    let mut resolved: Vec<_> = peaks.iter().map(|(y, id)| (*y, r.block(id))).collect();
    resolved.sort_by_key(|(y, _)| *y);
    resolved
}

/// Every heat band must hold a land biome (rivers aside), and no biome may name a band
/// that does not exist: a colder or hotter region would otherwise fall back to any biome.
pub(super) fn check_heat(
    bands: &[f64],
    biomes: &[Biome],
    source: &Path,
    errors: &mut Vec<PackError>,
) {
    let last = u8::try_from(bands.len()).unwrap_or(u8::MAX);
    for biome in biomes {
        if let Some((_, highest)) = biome.heat.filter(|&(_, highest)| highest > last) {
            let message = format!(
                "biome `{}` names heat band {highest}, but there are only {} bands",
                biome.id,
                bands.len() + 1
            );
            errors.push(PackError::invalid(source, message));
        }
    }
    if bands.is_empty() {
        return;
    }
    for band in 0..=last {
        let held = biomes
            .iter()
            .any(|b| !b.river && b.heat.is_some() && b.allows_heat(band));
        if !held {
            let message = format!("no land biome may appear in heat band {band}");
            errors.push(PackError::invalid(source, message));
        }
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::expect_used)]

    use super::*;
    use crate::WorldgenContent;
    use strata_core::content::Content;

    fn pack_biomes() -> (Vec<f64>, Vec<Biome>) {
        let mut errors = Vec::new();
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let packs = PackSet::discover(&dir, &mut errors);
        let (content, _) = Content::load(&packs, &PackSet::default());
        let terra = "strata:terra".parse().expect("valid id");
        let worldgen = WorldgenContent::load(&packs, &content.blocks, &terra, &mut errors)
            .expect("terra exists");
        (
            worldgen.terrain.heat_bands.clone(),
            worldgen.biomes().to_vec(),
        )
    }

    fn heat_errors(bands: &[f64], biomes: &[Biome]) -> usize {
        let mut errors = Vec::new();
        check_heat(bands, biomes, Path::new("terra.ron"), &mut errors);
        errors.len()
    }

    #[test]
    fn the_pack_covers_every_heat_band() {
        let (bands, biomes) = pack_biomes();
        assert_eq!(heat_errors(&bands, &biomes), 0);
    }

    #[test]
    fn missing_or_unknown_heat_bands_are_errors() {
        let (bands, mut biomes) = pack_biomes();
        let hottest = u8::try_from(bands.len()).expect("few bands");
        let mut hot_gone = biomes.clone();
        hot_gone.retain(|b| b.heat.is_none_or(|(_, last)| last < hottest));
        assert!(heat_errors(&bands, &hot_gone) > 0);
        biomes[0].heat = Some((0, hottest + 1));
        assert!(heat_errors(&bands, &biomes) > 0);
    }
}
