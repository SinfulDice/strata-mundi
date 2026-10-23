use super::entity::Entity;

/// Components of one type, packed densely (fast to iterate) with a sparse index from
/// entity slot to position (fast to look up). Removing swaps the last element in.
#[derive(Debug)]
pub struct Storage<T> {
    dense: Vec<T>,
    owners: Vec<Entity>,
    sparse: Vec<Option<u32>>,
}

impl<T> Default for Storage<T> {
    fn default() -> Self {
        Self {
            dense: Vec::new(),
            owners: Vec::new(),
            sparse: Vec::new(),
        }
    }
}

impl<T> Storage<T> {
    pub fn insert(&mut self, entity: Entity, component: T) {
        if let Some(existing) = self.get_mut(entity) {
            *existing = component;
            return;
        }
        let slot = entity.index();
        if let Some(Some(stale)) = self.sparse.get(slot).copied() {
            let owner = self.owners[stale as usize];
            self.remove(owner);
        }
        if self.sparse.len() <= slot {
            self.sparse.resize(slot + 1, None);
        }
        self.sparse[slot] = Some(self.dense.len() as u32);
        self.dense.push(component);
        self.owners.push(entity);
    }

    pub fn remove(&mut self, entity: Entity) -> Option<T> {
        let position = self.position(entity)?;
        self.sparse[entity.index()] = None;
        let last = self.dense.len() - 1;
        if position != last {
            let moved = self.owners[last];
            self.sparse[moved.index()] = Some(position as u32);
        }
        self.owners.swap_remove(position);
        Some(self.dense.swap_remove(position))
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.position(entity).map(|p| &self.dense[p])
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.position(entity).map(|p| &mut self.dense[p])
    }

    pub fn iter(&self) -> impl Iterator<Item = (Entity, &T)> {
        self.owners.iter().copied().zip(&self.dense)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Entity, &mut T)> {
        self.owners.iter().copied().zip(&mut self.dense)
    }

    fn position(&self, entity: Entity) -> Option<usize> {
        let position = (*self.sparse.get(entity.index())?)? as usize;
        (self.owners[position] == entity).then_some(position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ecs::Entities;

    #[test]
    fn insert_remove_and_iterate_stay_consistent() {
        let mut entities = Entities::default();
        let [a, b, c] = [entities.spawn(), entities.spawn(), entities.spawn()];
        let mut storage = Storage::default();
        storage.insert(a, "a");
        storage.insert(b, "b");
        storage.insert(c, "c");
        assert_eq!(storage.remove(a), Some("a"));
        assert_eq!(storage.get(c), Some(&"c"));
        assert_eq!(storage.get(a), None);
        let mut seen: Vec<_> = storage.iter().map(|(_, v)| *v).collect();
        seen.sort_unstable();
        assert_eq!(seen, ["b", "c"]);
    }

    #[test]
    fn stale_handles_find_nothing() {
        let mut entities = Entities::default();
        let old = entities.spawn();
        let mut storage = Storage::default();
        storage.insert(old, 1);
        storage.remove(old);
        entities.despawn(old);
        let new = entities.spawn();
        storage.insert(new, 2);
        assert_eq!(storage.get(old), None);
        assert_eq!(storage.get(new), Some(&2));
    }

    #[test]
    fn a_new_generation_replaces_a_component_left_behind() {
        let mut entities = Entities::default();
        let old = entities.spawn();
        let mut storage = Storage::default();
        storage.insert(old, 1);
        entities.despawn(old);
        let new = entities.spawn();
        storage.insert(new, 2);
        assert_eq!(storage.iter().map(|(_, v)| *v).collect::<Vec<_>>(), [2]);
    }
}
