//! 2D rendering module built on top of `wgpu`.

pub mod components;
pub mod renderer;
pub mod system;

pub use components::{SpriteComponent, TextureId, Transform2D};
pub use renderer::{Camera2D, RenderError, Renderer2D};
pub use system::RenderSystem;

#[cfg(test)]
mod tests;
