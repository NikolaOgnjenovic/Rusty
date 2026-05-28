//! Core ECS primitives and optional 2D rendering support.

/// Entity identifiers and lifecycle management.
pub mod entity;
/// Component abstractions, storages, and registration.
pub mod component;
/// Event queues and type-erased event dispatching.
pub mod event;
/// High-level world API combining entities, components, and events.
pub mod world;
/// System traits and executor for frame updates.
pub mod system;
/// 2D rendering components, renderer, and render system integration.
pub mod render;

pub use entity::{Entity, EntityManager};
pub use component::{Component, ComponentManager, HashMapComponentStorage};
pub use event::{Event, EventManager, EventQueue};
pub use world::World;
pub use system::{System, SystemExecutor};
