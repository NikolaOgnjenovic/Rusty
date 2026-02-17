#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Entity {
    pub id: u32,
    pub generation: u32,
}

pub struct EntityManager {
    next_id: u32,
    free_ids: Vec<u32>,
    generations: Vec<u32>,
}

impl EntityManager {
    pub fn new() -> Self {
        Self {
            next_id: 0,
            free_ids: Vec::new(),
            generations: Vec::new(),
        }
    }

    pub fn create(&mut self) -> Entity {
        if let Some(id) = self.free_ids.pop() {
            Entity {
                id,
                generation: self.generations[id as usize],
            }
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.generations.push(0);
            Entity { id, generation: 0 }
        }
    }

    pub fn destroy(&mut self, entity: Entity) {
        if (entity.id as usize) < self.generations.len() {
            if self.generations[entity.id as usize] == entity.generation {
                self.generations[entity.id as usize] += 1;
                self.free_ids.push(entity.id);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entity_creation() {
        let mut manager = EntityManager::new();

        let e1 = manager.create();
        let e2 = manager.create();

        assert_ne!(e1, e2);
        assert_eq!(e1, Entity { id: 0, generation: 0 });
        assert_eq!(e2, Entity { id: 1, generation: 0 });
    }

    #[test]
    fn test_entity_reuse() {
        let mut manager = EntityManager::new();

        let e1 = manager.create();
        manager.destroy(e1);
        let e2 = manager.create();

        assert_ne!(e1, e2);
        assert_eq!(e2.id, e1.id);
        assert_eq!(e2.generation, e1.generation + 1);
    }

    #[test]
    fn test_double_destroy_does_not_duplicate_free() {
        let mut manager = EntityManager::new();

        let original = manager.create();
        manager.destroy(original);
        manager.destroy(original);

        let e1 = manager.create();
        let e2 = manager.create();

        assert_eq!(e1.id, original.id);
        assert_eq!(e1.generation, original.generation + 1);

        // Second entity should not reuse same id again
        assert_ne!(e2.id, original.id);
    }

    #[test]
    fn test_destroy_stale_entity_is_ignored() {
        let mut manager = EntityManager::new();

        let e1 = manager.create();
        manager.destroy(e1);

        let e2 = manager.create();

        manager.destroy(e1);
        manager.destroy(e2);

        let e3 = manager.create();

        assert_eq!(e3.id, e1.id);
        assert_eq!(e3.generation, e2.generation + 1);
    }

    #[test]
    fn test_multiple_reuse_cycles() {
        let mut manager = EntityManager::new();

        let mut e = manager.create();

        for expected_gen in 1..5 {
            manager.destroy(e);
            e = manager.create();

            assert_eq!(e.id, 0);
            assert_eq!(e.generation, expected_gen);
        }
    }

    #[test]
    fn test_destroy_invalid_id_does_nothing() {
        let mut manager = EntityManager::new();

        let fake = Entity { id: 999, generation: 0 };
        manager.destroy(fake);

        let e = manager.create();
        assert_eq!(e.id, 0);
    }

    #[test]
    fn test_sequential_ids_without_reuse() {
        let mut manager = EntityManager::new();

        for expected_id in 0..100 {
            let e = manager.create();
            assert_eq!(e.id, expected_id);
            assert_eq!(e.generation, 0);
        }
    }
}
