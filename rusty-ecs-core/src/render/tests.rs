use crate::render::components::{SpriteComponent, TextureId, Transform2D};
use crate::render::renderer::{collect_visible_sprites, Renderer2D};
use crate::World;
use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

fn gpu_tests_enabled() -> bool {
    std::env::var("RUST_ECS_RENDER_TESTS").ok().as_deref() == Some("1")
}

fn has_adapter() -> bool {
    pollster::block_on(async {
        let instance = wgpu::Instance::default();
        instance
            .request_adapter(&wgpu::RequestAdapterOptions::default())
            .await
            .is_some()
    })
}

fn setup_world_for_batch(z_values: &[i32], visible: &[bool]) -> World {
    let mut world = World::new();
    for (idx, z) in z_values.iter().enumerate() {
        let entity = world.create_entity();
        world.add_component(
            entity,
            Transform2D {
                position: (idx as f32, idx as f32),
                rotation: 0.0,
                scale: (1.0, 1.0),
            },
        );
        world.add_component(
            entity,
            SpriteComponent {
                texture_id: TextureId(1),
                src_rect: None,
                draw_size: (1.0, 1.0),
                z: *z,
                visible: visible[idx],
                tint: [1.0, 1.0, 1.0, 1.0],
            },
        );
    }
    world
}

#[test]
fn sprite_default() {
    let sprite = SpriteComponent::default();
    assert!(sprite.visible);
    assert_eq!(sprite.tint, [1.0, 1.0, 1.0, 1.0]);
    assert_eq!(sprite.z, 0);
}

#[test]
fn transform_default() {
    let t = Transform2D::default();
    assert_eq!(t.position, (0.0, 0.0));
    assert_eq!(t.rotation, 0.0);
    assert_eq!(t.scale, (1.0, 1.0));
}

#[test]
fn camera_move_by() {
    let mut camera = crate::render::renderer::Camera2D {
        position: (0.0, 0.0),
        zoom: 1.0,
        viewport: (1, 1),
    };
    camera.move_by(10.0, 5.0);
    camera.move_by(-3.0, 2.0);
    assert_eq!(camera.position, (7.0, 7.0));
}

#[test]
fn camera_zoom_clamp() {
    let mut camera = crate::render::renderer::Camera2D {
        position: (0.0, 0.0),
        zoom: 1.0,
        viewport: (1, 1),
    };
    camera.set_zoom(2.0);
    assert_eq!(camera.zoom, 2.0);
    camera.set_zoom(0.0);
    assert_eq!(camera.zoom, 0.01);
}

#[test]
fn z_order_sort() {
    let world = setup_world_for_batch(&[-1, 5, 0], &[true, true, true]);
    let batch = collect_visible_sprites(&world);
    let order: Vec<i32> = batch.iter().map(|i| i.z).collect();
    assert_eq!(order, vec![-1, 0, 5]);
}

#[test]
fn visibility_filter() {
    let world = setup_world_for_batch(&[0, 1, 2], &[true, false, true]);
    let batch = collect_visible_sprites(&world);
    assert_eq!(batch.len(), 2);
}

#[test]
fn world_query_filter() {
    let mut world = World::new();
    for _ in 0..3 {
        let e = world.create_entity();
        world.add_component(e, Transform2D::default());
        world.add_component(
            e,
            SpriteComponent {
                texture_id: TextureId(1),
                ..Default::default()
            },
        );
    }
    let e_only_transform = world.create_entity();
    world.add_component(e_only_transform, Transform2D::default());
    let e_only_sprite = world.create_entity();
    world.add_component(
        e_only_sprite,
        SpriteComponent {
            texture_id: TextureId(1),
            ..Default::default()
        },
    );

    let batch = collect_visible_sprites(&world);
    assert_eq!(batch.len(), 3);
}

#[test]
fn renderer_init() {
    if !gpu_tests_enabled() || !has_adapter() {
        return;
    }
    let event_loop = match EventLoop::new() {
        Ok(v) => v,
        Err(_) => return,
    };
    let window = match event_loop.create_window(WindowAttributes::default()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let renderer = Renderer2D::new(&window);
    assert!(renderer.is_ok());
}

#[test]
fn load_rgba() {
    if !gpu_tests_enabled() || !has_adapter() {
        return;
    }
    let event_loop = match EventLoop::new() {
        Ok(v) => v,
        Err(_) => return,
    };
    let window = match event_loop.create_window(WindowAttributes::default()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut renderer = match Renderer2D::new(&window) {
        Ok(v) => v,
        Err(_) => return,
    };
    let data: [u8; 16] = [
        255, 255, 255, 255, 0, 0, 0, 255, 0, 0, 0, 255, 255, 255, 255, 255,
    ];
    let result = renderer.load_texture_rgba(TextureId(9), 2, 2, &data);
    assert!(result.is_ok());
    assert!(renderer.has_texture(TextureId(9)));
}

#[test]
fn texture_id_overwrite() {
    if !gpu_tests_enabled() || !has_adapter() {
        return;
    }
    let event_loop = match EventLoop::new() {
        Ok(v) => v,
        Err(_) => return,
    };
    let window = match event_loop.create_window(WindowAttributes::default()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut renderer = match Renderer2D::new(&window) {
        Ok(v) => v,
        Err(_) => return,
    };
    let data_a: [u8; 16] = [255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255, 255, 0, 0, 255];
    let data_b: [u8; 16] = [0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255, 0, 255];
    assert!(renderer.load_texture_rgba(TextureId(1), 2, 2, &data_a).is_ok());
    assert!(renderer.load_texture_rgba(TextureId(1), 2, 2, &data_b).is_ok());
    assert_eq!(renderer.texture_count(), 1);
}

#[test]
fn render_empty_world() {
    if !gpu_tests_enabled() || !has_adapter() {
        return;
    }
    let event_loop = match EventLoop::new() {
        Ok(v) => v,
        Err(_) => return,
    };
    let window = match event_loop.create_window(WindowAttributes::default()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut renderer = match Renderer2D::new(&window) {
        Ok(v) => v,
        Err(_) => return,
    };
    let world = World::new();
    assert!(renderer.render_world(&world).is_ok());
}

#[test]
fn render_single_sprite() {
    if !gpu_tests_enabled() || !has_adapter() {
        return;
    }
    let event_loop = match EventLoop::new() {
        Ok(v) => v,
        Err(_) => return,
    };
    let window = match event_loop.create_window(WindowAttributes::default()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut renderer = match Renderer2D::new(&window) {
        Ok(v) => v,
        Err(_) => return,
    };
    let data: [u8; 16] = [255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255];
    if renderer.load_texture_rgba(TextureId(3), 2, 2, &data).is_err() {
        return;
    }

    let mut world = World::new();
    let e = world.create_entity();
    world.add_component(e, Transform2D::default());
    world.add_component(
        e,
        SpriteComponent {
            texture_id: TextureId(3),
            ..Default::default()
        },
    );
    assert!(renderer.render_world(&world).is_ok());
}

#[test]
fn background_color() {
    if !gpu_tests_enabled() || !has_adapter() {
        return;
    }
    let event_loop = match EventLoop::new() {
        Ok(v) => v,
        Err(_) => return,
    };
    let window = match event_loop.create_window(WindowAttributes::default()) {
        Ok(v) => v,
        Err(_) => return,
    };
    let mut renderer = match Renderer2D::new(&window) {
        Ok(v) => v,
        Err(_) => return,
    };
    renderer.set_background([0.2, 0.3, 0.4, 1.0]);
    assert_eq!(renderer.background(), [0.2, 0.3, 0.4, 1.0]);
}
