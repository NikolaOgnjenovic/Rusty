use crate::world::World;

pub trait System {
    fn run(&mut self, world: &mut World);
}

pub struct SystemExecutor {
    systems: Vec<Box<dyn System>>,
}

impl SystemExecutor {
    pub fn new() -> Self {
        Self {
            systems: Vec::new(),
        }
    }

    pub fn add_system<S: System + 'static>(&mut self, system: S) {
        self.systems.push(Box::new(system));
    }

    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::world::World;

    struct CounterComponent(i32);
    struct FlagComponent(bool);

    struct CounterIncrementorSystem;

    impl System for CounterIncrementorSystem {
        fn run(&mut self, world: &mut World) {
            let entities = world.query_entities::<CounterComponent>();
            for entity in entities {
                if let Some(c) = world.get_component_mut::<CounterComponent>(entity) {
                    c.0 += 1;
                }
            }
        }
    }

    struct CounterDoublerSystem;

    impl System for CounterDoublerSystem {
        fn run(&mut self, world: &mut World) {
            let entities = world.query_entities::<CounterComponent>();
            for entity in entities {
                if let Some(c) = world.get_component_mut::<CounterComponent>(entity) {
                    c.0 *= 2;
                }
            }
        }
    }

    struct FlagToggleSystem;

    impl System for FlagToggleSystem {
        fn run(&mut self, world: &mut World) {
            let entities = world.query_entities::<FlagComponent>();
            for entity in entities {
                if let Some(f) = world.get_component_mut::<FlagComponent>(entity) {
                    f.0 = !f.0;
                }
            }
        }
    }

    #[test]
    fn test_multiple_entities() {
        let mut world = World::new();

        let e1 = world.create_entity();
        let e2 = world.create_entity();

        world.add_component(e1, CounterComponent(5));
        world.add_component(e2, CounterComponent(10));

        let mut executor = SystemExecutor::new();
        executor.add_system(CounterIncrementorSystem);
        executor.run(&mut world);

        assert_eq!(world.get_component::<CounterComponent>(e1).unwrap().0, 6);
        assert_eq!(world.get_component::<CounterComponent>(e2).unwrap().0, 11);
    }

    #[test]
    fn test_multiple_systems_execution_order() {
        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, CounterComponent(3));

        let mut executor = SystemExecutor::new();
        executor.add_system(CounterIncrementorSystem);
        executor.add_system(CounterDoublerSystem);

        executor.run(&mut world);

        assert_eq!(world.get_component::<CounterComponent>(e).unwrap().0, 8);
    }

    #[test]
    fn test_system_runs_multiple_times() {
        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, CounterComponent(0));

        let mut executor = SystemExecutor::new();
        executor.add_system(CounterIncrementorSystem);

        executor.run(&mut world);
        executor.run(&mut world);
        executor.run(&mut world);

        assert_eq!(world.get_component::<CounterComponent>(e).unwrap().0, 3);
    }

    #[test]
    fn test_system_with_no_matching_entities() {
        let mut world = World::new();
        let _e = world.create_entity();

        let mut executor = SystemExecutor::new();
        executor.add_system(CounterIncrementorSystem);

        // Should not panic
        executor.run(&mut world);
    }

    #[test]
    fn test_multiple_component_types() {
        let mut world = World::new();

        let e1 = world.create_entity();
        let e2 = world.create_entity();

        world.add_component(e1, CounterComponent(1));
        world.add_component(e2, FlagComponent(true));

        let mut executor = SystemExecutor::new();
        executor.add_system(CounterIncrementorSystem);
        executor.add_system(FlagToggleSystem);

        executor.run(&mut world);

        assert_eq!(world.get_component::<CounterComponent>(e1).unwrap().0, 2);
        assert_eq!(world.get_component::<FlagComponent>(e2).unwrap().0, false);
    }

    #[test]
    fn test_execution_order_matters() {
        let mut world = World::new();
        let e = world.create_entity();
        world.add_component(e, CounterComponent(2));

        let mut executor = SystemExecutor::new();
        executor.add_system(CounterDoublerSystem);
        executor.add_system(CounterIncrementorSystem);

        executor.run(&mut world);

        assert_eq!(world.get_component::<CounterComponent>(e).unwrap().0, 5);
    }
}
