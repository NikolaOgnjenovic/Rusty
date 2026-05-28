//! 2D rendering module built on top of `wgpu`.

pub mod app;
pub mod components;
pub mod renderer;
pub mod system;

pub use app::{RuntimeControl, pressed_key_code, run_2d_game};
pub use components::{SpriteComponent, TextureId, Transform2D};
pub use renderer::{Camera2D, RenderError, Renderer2D};
pub use system::RenderSystem;

#[cfg(test)]
mod tests;
