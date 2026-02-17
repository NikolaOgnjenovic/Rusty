use crate::entity::{Entity, EntityManager};
use crate::component::{Component, ComponentManager};
use crate::event::{Event, EventManager};

pub struct World {
    entities: EntityManager,
    components: ComponentManager,
    events: EventManager,
}

impl World {
    pub fn new() -> Self {
        Self {
            entities: EntityManager::new(),
            components: ComponentManager::new(),
            events: EventManager::new(),
        }
    }

    pub fn create_entity(&mut self) -> Entity {
        self.entities.create()
    }

    pub fn destroy_entity(&mut self, entity: Entity) {
        self.components.remove_all_components(entity);
        self.entities.destroy(entity);
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.components.add_component(entity, component);
    }

    pub fn get_component<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.components.get_storage::<T>()?.get(entity)
    }

    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_storage_mut::<T>()?.get_mut(entity)
    }

    pub fn push_event<E: Event>(&mut self, event: E) {
        self.events.push(event);
    }

    pub fn take_events<E: Event>(&mut self) -> Vec<E> {
        let mut events = Vec::new();
        if let Some(queue) = self.events.get_queue_mut::<E>() {
            while let Some(event) = queue.pop() {
                events.push(event);
            }
        }
        events
    }

    pub fn query_entities<T: Component>(&self) -> Vec<Entity> {
        if let Some(storage) = self.components.get_storage::<T>() {
            storage.entities().cloned().collect()
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Health(u32);
    struct Tag();
    struct DamageEvent(u32);

    #[test]
    fn test_world_basics() {
        let mut world = World::new();
        let e1 = world.create_entity();
        let e2 = world.create_entity();

        world.add_component(e1, Health(100));
        world.add_component(e1, Tag());
        world.add_component(e2, Health(50));

        // Test retrieval
        assert_eq!(world.get_component::<Health>(e1).unwrap().0, 100);
        assert_eq!(world.get_component::<Health>(e2).unwrap().0, 50);
        assert!(world.get_component::<Tag>(e1).is_some());
        assert!(world.get_component::<Tag>(e2).is_none());

        // Test mutation
        if let Some(h) = world.get_component_mut::<Health>(e1) {
            h.0 -= 20;
        }
        assert_eq!(world.get_component::<Health>(e1).unwrap().0, 80);

        // Test query
        let health_entities = world.query_entities::<Health>();
        assert_eq!(health_entities.len(), 2);
        assert!(health_entities.contains(&e1));
        assert!(health_entities.contains(&e2));

        let pos_entities = world.query_entities::<Tag>();
        assert_eq!(pos_entities.len(), 1);
        assert!(pos_entities.contains(&e1));
    }

    #[test]
    fn test_world_events() {
        let mut world = World::new();
        world.push_event(DamageEvent(10));
        world.push_event(DamageEvent(20));

        let events = world.take_events::<DamageEvent>();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].0, 10);
        assert_eq!(events[1].0, 20);

        let empty_events = world.take_events::<DamageEvent>();
        assert_eq!(empty_events.len(), 0);
    }

    #[test]
    fn test_entity_destruction() {
        let mut world = World::new();
        let e1 = world.create_entity();
        world.add_component(e1, Health(100));
        
        world.destroy_entity(e1);
        assert!(world.get_component::<Health>(e1).is_none());
        
        let e2 = world.create_entity();
        // e2 should reuse e1's ID but have a different generation
        assert_eq!(e1.id, e2.id);
        assert_ne!(e1.generation, e2.generation);
        assert!(world.get_component::<Health>(e2).is_none());
    }
}
