/// Handle to an entity. The generation tells apart a live entity from an older one that
/// was destroyed and whose slot was reused: a stale handle simply finds nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    index: u32,
    generation: u32,
}

impl Entity {
    pub fn index(self) -> usize {
        self.index as usize
    }
}

/// Hands out entity handles and recycles freed slots.
#[derive(Debug, Default)]
pub struct Entities {
    generations: Vec<u32>,
    alive: Vec<bool>,
    free: Vec<u32>,
}

impl Entities {
    pub fn spawn(&mut self) -> Entity {
        if let Some(index) = self.free.pop() {
            let slot = index as usize;
            self.alive[slot] = true;
            return Entity {
                index,
                generation: self.generations[slot],
            };
        }
        let index = self.generations.len() as u32;
        self.generations.push(0);
        self.alive.push(true);
        Entity {
            index,
            generation: 0,
        }
    }

    /// Returns `false` if the entity was already gone.
    pub fn despawn(&mut self, entity: Entity) -> bool {
        if !self.is_alive(entity) {
            return false;
        }
        let slot = entity.index();
        self.alive[slot] = false;
        self.generations[slot] = self.generations[slot].wrapping_add(1);
        self.free.push(entity.index);
        true
    }

    pub fn is_alive(&self, entity: Entity) -> bool {
        let slot = entity.index();
        self.alive.get(slot).copied().unwrap_or(false)
            && self.generations[slot] == entity.generation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reused_slots_get_a_new_generation() {
        let mut entities = Entities::default();
        let first = entities.spawn();
        assert!(entities.despawn(first));
        let second = entities.spawn();
        assert_eq!(first.index(), second.index());
        assert!(!entities.is_alive(first));
        assert!(entities.is_alive(second));
        assert!(!entities.despawn(first));
    }
}
