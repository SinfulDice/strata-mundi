use crate::id::Identifier;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Compact numeric id used at runtime (chunks, meshes, network) instead of the text identifier.
pub trait RegistryId: Copy {
    fn from_index(index: u16) -> Self;
    fn index(self) -> usize;
}

/// Entries addressable both by text identifier and by dense numeric id.
/// Numeric ids follow the order of insertion and are only stable within one run:
/// saves store text identifiers.
#[derive(Debug, Clone)]
pub struct Registry<K, T> {
    identifiers: Vec<Identifier>,
    values: Vec<T>,
    lookup: HashMap<Identifier, u16>,
    _key: PhantomData<K>,
}

impl<K: RegistryId, T> Default for Registry<K, T> {
    fn default() -> Self {
        Self {
            identifiers: Vec::new(),
            values: Vec::new(),
            lookup: HashMap::new(),
            _key: PhantomData,
        }
    }
}

impl<K: RegistryId, T> Registry<K, T> {
    /// Returns `None` if the identifier is taken or the registry is full.
    pub fn insert(&mut self, id: Identifier, value: T) -> Option<K> {
        if self.lookup.contains_key(&id) {
            return None;
        }
        let index = u16::try_from(self.values.len()).ok()?;
        self.lookup.insert(id.clone(), index);
        self.identifiers.push(id);
        self.values.push(value);
        Some(K::from_index(index))
    }

    pub fn get(&self, key: K) -> Option<&T> {
        self.values.get(key.index())
    }

    pub fn key(&self, id: &Identifier) -> Option<K> {
        self.lookup.get(id).map(|&index| K::from_index(index))
    }

    pub fn by_identifier(&self, id: &Identifier) -> Option<&T> {
        self.key(id).and_then(|key| self.get(key))
    }

    pub fn identifier(&self, key: K) -> Option<&Identifier> {
        self.identifiers.get(key.index())
    }

    pub fn len(&self) -> usize {
        self.values.len()
    }

    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &Identifier, &T)> {
        self.identifiers
            .iter()
            .zip(&self.values)
            .enumerate()
            .map(|(index, (id, value))| (K::from_index(index as u16), id, value))
    }
}

macro_rules! registry_id {
    ($(#[$doc:meta])* $name:ident) => {
        $(#[$doc])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord,
            serde::Serialize, serde::Deserialize,
        )]
        pub struct $name(pub u16);

        impl RegistryId for $name {
            fn from_index(index: u16) -> Self {
                Self(index)
            }

            fn index(self) -> usize {
                usize::from(self.0)
            }
        }
    };
}

registry_id!(
    /// Numeric block id; `BlockId::AIR` is always 0.
    BlockId
);
registry_id!(ItemId);
registry_id!(
    /// Layer in the block texture array; 0 is the "missing texture" pattern.
    TextureId
);

impl BlockId {
    pub const AIR: Self = Self(0);
}

impl TextureId {
    pub const MISSING: Self = Self(0);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_dense_and_unique() {
        let mut registry: Registry<ItemId, &str> = Registry::default();
        let a = registry.insert("x:a".parse().unwrap(), "a").unwrap();
        let b = registry.insert("x:b".parse().unwrap(), "b").unwrap();
        assert_eq!((a, b), (ItemId(0), ItemId(1)));
        assert!(registry.insert("x:a".parse().unwrap(), "again").is_none());
        assert_eq!(registry.by_identifier(&"x:b".parse().unwrap()), Some(&"b"));
        assert_eq!(
            registry.identifier(a).map(ToString::to_string),
            Some("x:a".into())
        );
    }
}
