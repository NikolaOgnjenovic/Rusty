# Rusty

## Advanced Programming Techniques - Class Project

### Implementing a Basic Entity Component System and a Turn-Based Terminal Game

---

## Project Overview

This project explores the design and implementation of a minimal Entity Component System (ECS) written in Rust, accompanied by a small turn-based terminal dungeon crawler used to validate and demonstrate the engine’s architecture.

The primary goal is to build a principled toy engine, focusing on correctness, architecture, performance considerations, and idiomatic Rust usage, while deliberately keeping the scope small enough to remain understandable and extensible. This project serves as a foundation for a future bachelor’s thesis centered on building a 2D game engine in Rust.

The game itself is intentionally simple: a turn-based text dungeon crawler featuring a player, enemies, health, damage, and basic game rules. Its purpose is not to be feature-rich, but to act as a concrete use case for the ECS.

---

## Goals

* Design and implement a custom ECS from scratch
* Explore Rust’s ownership and borrowing model in engine architecture
* Minimize `unsafe` code and justify its use where unavoidable
* Provide a clean, testable, and well-documented codebase
* Build a small yet complete game using the ECS as the only game logic layer

---

## Approach

### 1. ECS Design

The ECS is implemented with the following principles:

* Entities are lightweight identifiers with no behavior
* Components are plain data, stored separately by type
* Systems operate over queried component sets
* Events are used to decouple systems and enable indirect communication
* Resources represent global state shared across systems

The ECS is designed to rely primarily on compile-time borrow checking, avoiding runtime checks (`RefCell`) where possible. Limited `unsafe` Rust may be used internally for performance or ergonomics, with clear documentation and invariants.

---

### 2. Storage Model

The ECS uses a component-centric storage model, allowing efficient iteration over entities that possess a specific set of components. The design prioritizes:

* Cache-friendly access patterns
* Clear ownership of component data
* Safe system execution without aliasing violations

The exact storage strategy is intentionally simple and optimized for clarity rather than maximal performance.

---

### 3. Systems and Scheduling

Systems are executed in a deterministic order, suitable for a turn-based game loop. The scheduler:

* Executes systems sequentially
* Enforces borrowing rules at compile time
* Allows future extensions such as system dependencies or parallel execution

---

### 4. Event System

An internal event queue enables loose coupling between systems. Examples include:

* Damage events
* Entity death events
* Turn completion events

Events are processed in discrete phases of the game loop, ensuring predictable behavior.

---

## Game Demo: Terminal Dungeon Crawler

The ECS is demonstrated through a terminal-based dungeon crawler.

### Game Features

* Turn-based gameplay
* Player and enemy entities
* Health and damage mechanics
* Simple enemy behavior (fixed set of attack / defense options)
* Text-based dungeon representation
* Deterministic simulation loop

The game logic is entirely implemented using the ECS, with no special-case code outside the engine.

---

## Architecture (subject to change)

```
Rusty/
├── ecs-core/
│   ├── entity.rs        # Entity definitions and ID management
│   ├── component.rs     # Component traits and storage
│   ├── system.rs        # System interfaces and execution
│   ├── event.rs         # Event definitions and queues
│   └── world.rs         # ECS world and orchestration
│
├── game-demo/
│   ├── components/      # Game-specific components (Health, Position, etc.)
│   ├── systems/         # Game systems (Combat, AI, Turns)
│   ├── map.rs           # Dungeon representation
│   └── main.rs          # Game loop and input handling
│
└── README.md
```

---

## Key Components

### Core ECS Components

* Entity - Unique identifiers with no embedded data
* Component Storage - Type-based storage for component data
* World - Central registry for entities, components, systems, and events
* System - Stateless or stateful logic operating on queried components
* Event Queue - FIFO event handling mechanism

---

## Safety and Rust Considerations

* Minimal `unsafe` Rust, used only where necessary
* Clear documentation of safety invariants
* No runtime borrow checking (`RefCell`) in the public API
* Emphasis on explicit lifetimes and ownership
* Strong separation between engine and game logic

---

## Testing

* Unit tests for ECS storage and querying
* Deterministic game logic tests
* Validation of event ordering and system execution

---

## Planned Extensions for Bachelor's Thesis

1. Archetype-based storage
2. Change detection for components
3. Save/load functionality
4. Rendering abstraction
5. 2D graphics
6. Expanded game mechanics 
7. Performance benchmarks

---

## Motivation for Future Work

While this project intentionally limits scope, it exposes many of the design trade-offs involved in engine development, particularly when working within Rust’s safety guarantees. These challenges will be explored in depth in the accompanying bachelor’s thesis.
