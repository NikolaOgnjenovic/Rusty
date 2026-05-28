use crate::render::renderer::Renderer2D;
use crate::system::System;
use crate::world::World;
use std::sync::{Arc, Mutex};

/// ECS system that renders the world using a shared `Renderer2D`.
pub struct RenderSystem {
    renderer: Arc<Mutex<Renderer2D>>,
}

impl RenderSystem {
    /// Creates a render system from a shared renderer.
    pub fn new(renderer: Arc<Mutex<Renderer2D>>) -> Self {
        Self { renderer }
    }
}

impl System for RenderSystem {
    fn run(&mut self, world: &mut World) {
        if let Ok(mut renderer) = self.renderer.lock() {
            let _ = renderer.render_world(world);
        }
    }
}
