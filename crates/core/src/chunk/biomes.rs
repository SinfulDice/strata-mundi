use crate::coords::SECTION_SIZE;

/// Index of a biome in the server's biome list (sorted by identifier), as sent to clients.
#[derive(
    Debug, Clone, Copy, Default, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct BiomeId(pub u16);

const AREA: usize = (SECTION_SIZE * SECTION_SIZE) as usize;

/// Biome of each of the 16×16 block columns of a column, indexed `z * 16 + x`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "Vec<BiomeId>", into = "Vec<BiomeId>")]
pub struct BiomeMap {
    ids: Box<[BiomeId; AREA]>,
}

impl BiomeMap {
    pub fn uniform(biome: BiomeId) -> Self {
        Self {
            ids: Box::new([biome; AREA]),
        }
    }

    /// `biome(x, z)` with local coordinates in `0..16`.
    pub fn from_fn(mut biome: impl FnMut(u32, u32) -> BiomeId) -> Self {
        let mut map = Self::uniform(BiomeId::default());
        for (index, slot) in map.ids.iter_mut().enumerate() {
            *slot = biome(index as u32 % 16, index as u32 / 16);
        }
        map
    }

    /// Local x/z, wrapped into `0..16`.
    pub fn get(&self, x: u32, z: u32) -> BiomeId {
        self.ids[((z % 16) * 16 + x % 16) as usize]
    }
}

impl TryFrom<Vec<BiomeId>> for BiomeMap {
    type Error = String;

    fn try_from(ids: Vec<BiomeId>) -> Result<Self, Self::Error> {
        let length = ids.len();
        let ids: Box<[BiomeId; AREA]> = ids
            .into_boxed_slice()
            .try_into()
            .map_err(|_| format!("a biome map holds {AREA} biomes, not {length}"))?;
        Ok(Self { ids })
    }
}

impl From<BiomeMap> for Vec<BiomeId> {
    fn from(map: BiomeMap) -> Self {
        map.ids.to_vec()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_map_to_their_biome() {
        let map = BiomeMap::from_fn(|x, z| BiomeId((x + 16 * z) as u16));
        assert_eq!(map.get(0, 0), BiomeId(0));
        assert_eq!(map.get(15, 0), BiomeId(15));
        assert_eq!(map.get(3, 2), BiomeId(35));
        assert_eq!(map.get(16 + 3, 2), BiomeId(35));
    }

    #[test]
    fn a_short_list_is_refused() {
        assert!(BiomeMap::try_from(vec![BiomeId(1); 255]).is_err());
        assert!(BiomeMap::try_from(vec![BiomeId(1); 256]).is_ok());
    }
}
