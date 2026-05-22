use crate::entity::{Entity, EntityManager};
use crate::component::{Component, ComponentManager};
use crate::event::{Event, EventManager};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;
use std::collections::HashMap;

pub struct World {
    entities: EntityManager,
    components: ComponentManager,
    events: EventManager,
}

#[derive(Serialize, Deserialize)]
struct WorldSaveData {
    entities: EntityManager,
    components: HashMap<String, serde_json::Value>,
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
        self.components.get_storage_by_type::<T>()?.get(entity)
    }

    pub fn get_component_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_storage_mut_by_type::<T>()?.get_mut(entity)
    }

    pub fn register_component<T: Component>(&mut self, name: &str) {
        self.components.register::<T>(name);
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
        if let Some(storage) = self.components.get_storage_by_type::<T>() {
            storage.entities().cloned().collect()
        } else {
            Vec::new()
        }
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> std::io::Result<()> {
        let save_data = WorldSaveData {
            entities: self.entities.clone(),
            components: self.components.serialize_all(),
        };
        let json = serde_json::to_string(&save_data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        let mut file = File::create(path)?;
        file.write_all(json.as_bytes())?;
        Ok(())
    }

    pub fn load<P: AsRef<Path>>(&mut self, path: P) -> std::io::Result<()> {
        let mut file = File::open(path)?;
        let mut json = String::new();
        file.read_to_string(&mut json)?;
        let save_data: WorldSaveData = serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        
        self.entities = save_data.entities;
        self.components.deserialize_all(save_data.components);
        self.events.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct Health(u32);
    #[derive(Serialize, Deserialize, Debug, PartialEq)]
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
    fn test_world_save_load() {
        let mut world = World::new();
        world.register_component::<Health>("Health");
        world.register_component::<Tag>("Tag");

        let e1 = world.create_entity();
        let e2 = world.create_entity();

        world.add_component(e1, Health(100));
        world.add_component(e1, Tag());
        world.add_component(e2, Health(50));

        let path = "test_world.save";
        world.save(path).unwrap();

        let mut new_world = World::new();
        new_world.register_component::<Health>("Health");
        new_world.register_component::<Tag>("Tag");
        new_world.load(path).unwrap();

        assert_eq!(new_world.get_component::<Health>(e1).unwrap().0, 100);
        assert!(new_world.get_component::<Tag>(e1).is_some());
        assert_eq!(new_world.get_component::<Health>(e2).unwrap().0, 50);

        std::fs::remove_file(path).unwrap();
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
