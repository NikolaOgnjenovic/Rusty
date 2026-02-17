use std::any::{Any, TypeId};
use std::collections::{HashMap, VecDeque};

pub trait Event: Any + 'static {}
impl<T: Any + 'static> Event for T {}

pub trait EventQueueTrait: Any {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn clear(&mut self);
}

pub struct EventQueue<E: Event> {
    events: VecDeque<E>,
}

impl<E: Event> EventQueue<E> {
    pub fn new() -> Self {
        Self {
            events: VecDeque::new(),
        }
    }

    pub fn push(&mut self, event: E) {
        self.events.push_back(event);
    }

    pub fn pop(&mut self) -> Option<E> {
        self.events.pop_front()
    }

    pub fn iter(&self) -> impl Iterator<Item = &E> {
        self.events.iter()
    }
}

impl<E: Event> EventQueueTrait for EventQueue<E> {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn clear(&mut self) {
        self.events.clear();
    }
}

pub struct EventManager {
    queues: HashMap<TypeId, Box<dyn EventQueueTrait>>,
}

impl EventManager {
    pub fn new() -> Self {
        Self {
            queues: HashMap::new(),
        }
    }

    pub fn register<E: Event>(&mut self) {
        let type_id = TypeId::of::<E>();
        if !self.queues.contains_key(&type_id) {
            self.queues.insert(type_id, Box::new(EventQueue::<E>::new()));
        }
    }

    pub fn get_queue<E: Event>(&self) -> Option<&EventQueue<E>> {
        self.queues
            .get(&TypeId::of::<E>())?
            .as_any()
            .downcast_ref::<EventQueue<E>>()
    }

    pub fn get_queue_mut<E: Event>(&mut self) -> Option<&mut EventQueue<E>> {
        let queue = self.queues.get_mut(&TypeId::of::<E>())?;
        queue.as_any_mut().downcast_mut::<EventQueue<E>>()
    }

    pub fn push<E: Event>(&mut self, event: E) {
        self.register::<E>();
        if let Some(queue) = self.get_queue_mut::<E>() {
            queue.push(event);
        }
    }

    pub fn clear(&mut self) {
        for queue in self.queues.values_mut() {
            queue.clear();
        }
    }
}

#[cfg(test)] mod tests {
    use crate::{EventManager, EventQueue};

    #[derive(Debug, PartialEq)]
    struct DamageEvent {
        amount: u32,
    }

    #[derive(Debug, PartialEq)]
    struct SpawnEvent {
        id: u32,
    }

    #[test]
    fn test_event_queue_push_and_pop() {
        let mut queue = EventQueue::<DamageEvent>::new();

        queue.push(DamageEvent { amount: 10 });
        queue.push(DamageEvent { amount: 20 });

        assert_eq!(queue.pop(), Some(DamageEvent { amount: 10 }));
        assert_eq!(queue.pop(), Some(DamageEvent { amount: 20 }));
        assert_eq!(queue.pop(), None);
    }

    #[test]
    fn test_event_queue_fifo_order() {
        let mut queue = EventQueue::<DamageEvent>::new();

        for i in 0..5 {
            queue.push(DamageEvent { amount: i });
        }

        for i in 0..5 {
            assert_eq!(queue.pop(), Some(DamageEvent { amount: i }));
        }
    }

    #[test]
    fn test_event_queue_iter() {
        let mut queue = EventQueue::<DamageEvent>::new();

        queue.push(DamageEvent { amount: 1 });
        queue.push(DamageEvent { amount: 2 });

        let events: Vec<_> = queue.iter().collect();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0], &DamageEvent { amount: 1 });
        assert_eq!(events[1], &DamageEvent { amount: 2 });
    }

    #[test]
    fn test_event_manager_auto_register_on_push() {
        let mut manager = EventManager::new();

        manager.push(DamageEvent { amount: 42 });

        let queue = manager.get_queue::<DamageEvent>();
        assert!(queue.is_some());

        let event = queue.unwrap().iter().next();
        assert_eq!(event, Some(&DamageEvent { amount: 42 }));
    }

    #[test]
    fn test_multiple_event_types() {
        let mut manager = EventManager::new();

        manager.push(DamageEvent { amount: 10 });
        manager.push(SpawnEvent { id: 99 });

        let damage_queue = manager.get_queue::<DamageEvent>().unwrap();
        let spawn_queue = manager.get_queue::<SpawnEvent>().unwrap();

        assert_eq!(
            damage_queue.iter().next(),
            Some(&DamageEvent { amount: 10 })
        );

        assert_eq!(
            spawn_queue.iter().next(),
            Some(&SpawnEvent { id: 99 })
        );
    }

    #[test]
    fn test_get_queue_unregistered() {
        let manager = EventManager::new();

        assert!(manager.get_queue::<DamageEvent>().is_none());
    }

    #[test]
    fn test_event_manager_clear() {
        let mut manager = EventManager::new();

        manager.push(DamageEvent { amount: 1 });
        manager.push(SpawnEvent { id: 2 });

        manager.clear();

        let damage_queue = manager.get_queue::<DamageEvent>().unwrap();
        let spawn_queue = manager.get_queue::<SpawnEvent>().unwrap();

        assert_eq!(damage_queue.iter().count(), 0);
        assert_eq!(spawn_queue.iter().count(), 0);
    }
}