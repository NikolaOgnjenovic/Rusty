use crate::entity::Entity;
use std::any::{Any, TypeId};
use std::collections::HashMap;

pub trait Component: Any + 'static {}
impl<T: Any + 'static> Component for T {}

pub trait ComponentStorage: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn remove(&mut self, entity: Entity);
}

pub struct HashMapComponentStorage<T: Component> {
    components: HashMap<Entity, T>,
}

impl<T: Component> HashMapComponentStorage<T> {
    pub fn new() -> Self {
        Self {
            components: HashMap::new(),
        }
    }

    pub fn insert(&mut self, entity: Entity, component: T) {
        self.components.insert(entity, component);
    }

    pub fn get(&self, entity: Entity) -> Option<&T> {
        self.components.get(&entity)
    }

    pub fn get_mut(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut(&entity)
    }

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
}

pub struct ComponentManager {
    storages: HashMap<TypeId, Box<dyn ComponentStorage>>,
}

impl ComponentManager {
    pub fn new() -> Self {
        Self {
            storages: HashMap::new(),
        }
    }

    pub fn register<T: Component>(&mut self) {
        let type_id = TypeId::of::<T>();
        if !self.storages.contains_key(&type_id) {
            self.storages
                .insert(type_id, Box::new(HashMapComponentStorage::<T>::new()));
        }
    }

    pub fn get_storage<T: Component>(&self) -> Option<&HashMapComponentStorage<T>> {
        self.storages
            .get(&TypeId::of::<T>())?
            .as_any()
            .downcast_ref::<HashMapComponentStorage<T>>()
    }

    pub fn get_storage_mut<T: Component>(&mut self) -> Option<&mut HashMapComponentStorage<T>> {
        let storage = self.storages.get_mut(&TypeId::of::<T>())?;
        storage
            .as_any_mut()
            .downcast_mut::<HashMapComponentStorage<T>>()
    }

    pub fn add_component<T: Component>(&mut self, entity: Entity, component: T) {
        self.register::<T>();
        if let Some(storage) = self.get_storage_mut::<T>() {
            storage.insert(entity, component);
        }
    }

    pub fn remove_all_components(&mut self, entity: Entity) {
        for storage in self.storages.values_mut() {
            storage.remove(entity);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ComponentManager, Entity, HashMapComponentStorage};
    use crate::component::ComponentStorage;

    #[derive(Debug, PartialEq)]
    struct Position {
        x: f32,
        y: f32,
    }

    #[derive(Debug, PartialEq)]
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

        manager.register::<Position>();

        let storage = manager.get_storage::<Position>();
        assert!(storage.is_some());
    }

    #[test]
    fn test_add_component_creates_storage_if_missing() {
        let mut manager = ComponentManager::new();
        let entity = Entity { id: 10, generation: 0 };

        manager.add_component(entity, Position { x: 3.0, y: 4.0 });

        let storage = manager.get_storage::<Position>().unwrap();
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

        let pos_storage = manager.get_storage::<Position>().unwrap();
        let vel_storage = manager.get_storage::<Velocity>().unwrap();

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

        let pos_storage = manager.get_storage::<Position>().unwrap();
        let vel_storage = manager.get_storage::<Velocity>().unwrap();

        assert!(pos_storage.get(entity).is_none());
        assert!(vel_storage.get(entity).is_none());
    }

    #[test]
    fn test_get_storage_returns_none_if_not_registered() {
        let manager = ComponentManager::new();
        assert!(manager.get_storage::<Position>().is_none());
    }
}