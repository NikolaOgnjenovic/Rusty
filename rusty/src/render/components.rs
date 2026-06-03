use serde::{Deserialize, Serialize};

/// Identifier used for renderer-managed textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct TextureId(pub u32);

/// Sprite data used by the 2D renderer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpriteComponent {
    /// Texture handle inside the renderer registry.
    pub texture_id: TextureId,
    /// Optional source rectangle in pixels as `[x, y, width, height]`.
    pub src_rect: Option<[f32; 4]>,
    /// Output sprite size in world units.
    pub draw_size: (f32, f32),
    /// Draw order value. Lower z values are rendered first.
    pub z: i32,
    /// Visibility toggle for batching.
    pub visible: bool,
    /// Multiplicative RGBA tint.
    pub tint: [f32; 4],
}

impl Default for SpriteComponent {
    fn default() -> Self {
        Self {
            texture_id: TextureId(0),
            src_rect: None,
            draw_size: (1.0, 1.0),
            z: 0,
            visible: true,
            tint: [1.0, 1.0, 1.0, 1.0],
        }
    }
}

/// 2D transform component consumed by the renderer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform2D {
    /// World-space position.
    pub position: (f32, f32),
    /// Rotation in radians.
    pub rotation: f32,
    /// World-space scale.
    pub scale: (f32, f32),
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            position: (0.0, 0.0),
            rotation: 0.0,
            scale: (1.0, 1.0),
        }
    }
}
