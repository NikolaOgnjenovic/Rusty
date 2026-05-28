use crate::entity::Entity;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use serde::ser::Serialize;
use serde::de::{Deserialize};

/// Marker trait for ECS components.
///
/// Components must be `'static` and serializable to support world persistence.
pub trait Component: Any + Serialize + for<'de> Deserialize<'de> + 'static {}
impl<T: Any + Serialize + for<'de> Deserialize<'de> + 'static> Component for T {}

/// Type-erased component storage used internally by [`ComponentManager`].
pub trait ComponentStorage: Any {
    /// Returns an immutable type-erased reference for downcasting.
    fn as_any(&self) -> &dyn Any;
    /// Returns a mutable type-erased reference for downcasting.
    fn as_any_mut(&mut self) -> &mut dyn Any;
    /// Removes a component for the given entity, if present.
    fn remove(&mut self, entity: Entity);
    /// Serializes the storage into JSON.
    fn serialize_storage(&self) -> serde_json::Value;
    /// Replaces the storage contents from JSON.
    fn deserialize_storage(&mut self, value: serde_json::Value);
}

/// HashMap-backed storage mapping entities to components of type `T`.
pub struct HashMapComponentStorage<T: Component> {
    components: HashMap<Entity, T>,
}

impl<T: Component> HashMapComponentStorage<T> {
    /// Creates an empty storage.
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    /// Inserts or replaces the component for `entity`.
    pub fn insert(&mut self, entity: Entity, component: T) {
        self.components.insert(entity, component);
    }

    /// Returns an immutable component reference for `entity`.
    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.components.get(&entity)
    }

    /// Returns a mutable component reference for `entity`.
    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut(&entity)
    }

    /// Iterates over entities that currently own this component type.
    pub fn entities(&self) -> impl Iterator<Item = &Entity> {
        self.components.keys()
    }
}

impl<T: Component> ComponentStorage for HashMapComponentStorage<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn remove(&mut self, entity: Entity) {
        self.components.remove(&entity);
    }

    fn serialize_storage(&self) -> serde_json::Value {
        let map: HashMap<String, &T> = self.components.iter()
            .map(|(e, c)| (format!("{}:{}", e.id, e.generation), c))
            .collect();
        serde_json::to_value(&map).unwrap_or_else(|e| {
            eprintln!("Failed to serialize storage: {}", e);
            serde_json::Value::Null
        })
    }

    fn deserialize_storage(&mut self, value: serde_json::Value) {
        match serde_json::from_value::<HashMap<String, T>>(value) {
            Ok(map) => {
                self.components.clear();
                for (k, v) in map {
                    let parts: Vec<&str> = k.split(':').collect();
                    if parts.len() == 2 {
                        if let (Ok(id), Ok(generation)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                            self.components.insert(Entity { id, generation }, v);
                        }
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to deserialize storage: {}", e);
            }
        }
    }
}

pub struct ComponentManager {
    storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
    type_names: HashMap<String, TypeId>,
}

impl ComponentManager {
    /// Creates an empty component manager.
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
            type_names: HashMap::new(),
        }
    }

    /// Registers a component type and associates it with a stable type name.
    pub fn register<T: Component>(&mut self, name: &str) {
        let type_id = TypeId::of::<T>();
        self.type_names.insert(name.to_string(), type_id);
        if !self.storages.contains_key(&type_id) {
            self.storages
                .insert(type_id, Box::new(HashMapComponentStorage::<T>::new()));
        }
    }

    /// Returns type-erased storage by [`TypeId`].
    pub fn get_storage(&self, type_id: &TypeId) -> Option<&Box<dyn ComponentStorage>> {
        self.storages.get(type_id)
    }

    /// Returns mutable type-erased storage by [`TypeId`].
    pub fn get_storage_mut(&mut self, type_id: &TypeId) -> Option<&mut Box<dyn ComponentStorage>> {
        self.storages.get_mut(type_id)
    }

    /// Returns typed storage for component `T`.
    pub fn get_storage_by_type<T: Component>(&self) -> Option<&HashMapComponentStorage<T>> {
        self.storages
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<HashMapComponentStorage<T>>()
    }

    /// Returns mutable typed storage for component `T`.
    pub fn get_storage_mut_by_type<T: Component>(&mut self) -> Option<&mut HashMapComponentStorage<T>> {
        let storage = self.storages.get_mut(&TypeId::of::<T>())?;
        storage
            .as_any_mut()
            .downcast_mut::<HashMapComponentStorage<T>>()
    }

    /// Adds a component to an entity, creating storage for `T` if necessary.
    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        let type_id = TypeId::of::<T>();
        if !self.storages.contains_key(&type_id) {
            self.storages
                .insert(type_id, Box::new(HashMapComponentStorage::<T>::new()));
        }
        if let Some(storage) = self.get_storage_mut_by_type::<T>() {
            storage.insert(entity, component);
        }
    }

    /// Removes all component types associated with an entity.
    pub fn remove_all_components(&mut self, entity: Entity) {
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
    }

    /// Serializes all registered component storages by their registered names.
    pub fn serialize_all(&self) -> HashMap<String, serde_json::Value> {
        let mut serialized = HashMap::new();
        for (name, type_id) in &self.type_names {
            if let Some(storage) = self.storages.get(type_id) {
                let val = storage.serialize_storage();
                serialized.insert(name.clone(), val);
            }
        }
        serialized
    }

    /// Deserializes storages from a name-to-JSON map.
    pub fn deserialize_all(&mut self, serialized: HashMap<String, serde_json::Value>) {
        for (name, value) in serialized {
            if let Some(type_id) = self.type_names.get(&name) {
                if let Some(storage) = self.storages.get_mut(type_id) {
                    storage.deserialize_storage(value);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ComponentManager, Entity, HashMapComponentStorage};
    use crate::component::ComponentStorage;
    use serde::{Serialize, Deserialize};

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Velocity {
        dx: f32,
        dy: f32,
    }

    #[test]
    fn test_insert_and_get_component() {
        let mut storage = HashMapComponentStorage::<Position>::new();
        let entity = Entity { id: 1, generation: 0 };

        storage.insert(entity, Position { x: 10.0, y: 20.0 });

        let pos = storage.get(entity);
        assert!(pos.is_some());
        assert_eq!(pos.unwrap(), &Position { x: 10.0, y: 20.0 });
    }

    #[test]
    fn test_get_mut_component() {
        let mut storage = HashMapComponentStorage::<Position>::new();
        let entity = Entity { id: 2, generation: 0 };

        storage.insert(entity, Position { x: 1.0, y: 2.0 });

        if let Some(pos) = storage.get_mut(entity) {
            pos.x = 5.0;
        }

        assert_eq!(
            storage.get(entity),
            Some(&Position { x: 5.0, y: 2.0 })
        );
    }

    #[test]
    fn test_remove_component() {
        let mut storage = HashMapComponentStorage::<Position>::new();
        let entity = Entity { id: 3, generation: 0 };

        storage.insert(entity, Position { x: 0.0, y: 0.0 });
        storage.remove(entity);

        assert!(storage.get(entity).is_none());
    }

    #[test]
    fn test_entities_iterator() {
        let mut storage = HashMapComponentStorage::<Position>::new();

        let e1 = Entity { id: 1, generation: 0 };
        let e2 = Entity { id: 2, generation: 0 };

        storage.insert(e1, Position { x: 0.0, y: 0.0 });
        storage.insert(e2, Position { x: 1.0, y: 1.0 });

        let entities: Vec<_> = storage.entities().cloned().collect();

        assert_eq!(entities.len(), 2);
        assert!(entities.contains(&e1));
        assert!(entities.contains(&e2));
    }

    #[test]
    fn test_register_and_get_storage() {
        let mut manager = ComponentManager::new();

        manager.register::<Position>("Position");

        let storage = manager.get_storage_by_type::<Position>();
        assert!(storage.is_some());
    }

    #[test]
    fn test_add_component_creates_storage_if_missing() {
        let mut manager = ComponentManager::new();
        let entity = Entity { id: 10, generation: 0 };

        manager.add_component(entity, Position { x: 3.0, y: 4.0 });

        let storage = manager.get_storage_by_type::<Position>().unwrap();
        assert_eq!(
            storage.get(entity),
            Some(&Position { x: 3.0, y: 4.0 })
        );
    }

    #[test]
    fn test_multiple_component_types() {
        let mut manager = ComponentManager::new();
        let entity = Entity { id: 11, generation: 0 };

        manager.add_component(entity, Position { x: 1.0, y: 2.0 });
        manager.add_component(entity, Velocity { dx: 0.5, dy: 1.5 });

        let pos_storage = manager.get_storage_by_type::<Position>().unwrap();
        let vel_storage = manager.get_storage_by_type::<Velocity>().unwrap();

        assert_eq!(
            pos_storage.get(entity),
            Some(&Position { x: 1.0, y: 2.0 })
        );

        assert_eq!(
            vel_storage.get(entity),
            Some(&Velocity { dx: 0.5, dy: 1.5 })
        );
    }

    #[test]
    fn test_remove_all_components() {
        let mut manager = ComponentManager::new();
        let entity = Entity { id: 12, generation: 0 };

        manager.add_component(entity, Position { x: 1.0, y: 2.0 });
        manager.add_component(entity, Velocity { dx: 3.0, dy: 4.0 });

        manager.remove_all_components(entity);

        let pos_storage = manager.get_storage_by_type::<Position>().unwrap();
        let vel_storage = manager.get_storage_by_type::<Velocity>().unwrap();

        assert!(pos_storage.get(entity).is_none());
        assert!(vel_storage.get(entity).is_none());
    }

    #[test]
    fn test_get_storage_returns_none_if_not_registered() {
        let manager = ComponentManager::new();
        assert!(manager.get_storage_by_type::<Position>().is_none());
    }

    #[test]
    fn test_serialization() {
        let mut manager = ComponentManager::new();
        manager.register::<Position>("Position");
        
        let e1 = Entity { id: 1, generation: 0 };
        manager.add_component(e1, Position { x: 10.0, y: 20.0 });
        
        let serialized = manager.serialize_all();
        assert!(serialized.contains_key("Position"));
        
        let mut new_manager = ComponentManager::new();
        new_manager.register::<Position>("Position");
        new_manager.deserialize_all(serialized);
        
        let storage = new_manager.get_storage_by_type::<Position>().unwrap();
        assert_eq!(storage.get(e1), Some(&Position { x: 10.0, y: 20.0 }));
    }
}