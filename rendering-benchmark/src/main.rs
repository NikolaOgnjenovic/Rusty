use rusty::render::{
    RuntimeControl, SpriteComponent, TextureId, Transform2D, run_2d_game,
};
use rusty::{Entity, World};
use serde::{Deserialize, Serialize};
use std::cell::RefCell;
use std::rc::Rc;
use winit::keyboard::KeyCode;

const WHITE_TEXTURE: TextureId = TextureId(1);
const UI_TEXTURE: TextureId = TextureId(2);

const MAX_OBJECTS: usize = 50_000;
const DEFAULT_OBJECTS: usize = 1_500;
const MIN_RADIUS: f32 = 3.0;
const MAX_RADIUS: f32 = 11.0;

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Velocity2D {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct SimObjectTag;

struct PerfScene {
    world: World,
    objects: Vec<Entity>,
    rng_state: u64,
    target_objects: usize,
    input_buffer: String,
    speed_scale: f32,
    size_scale: f32,
    fps_timer: f32,
    fps_frames: u32,
    fps: f32,
    title_timer: f32,
    title_dirty: bool,
    last_viewport: (u32, u32),
}

impl PerfScene {
    fn new(viewport: (u32, u32)) -> Self {
        let mut scene = Self {
            world: World::new(),
            objects: Vec::new(),
            rng_state: 0xD1B5_4A32_86D1_44E3,
            target_objects: DEFAULT_OBJECTS,
            input_buffer: DEFAULT_OBJECTS.to_string(),
            speed_scale: 1.0,
            size_scale: 1.0,
            fps_timer: 0.0,
            fps_frames: 0,
            fps: 0.0,
            title_timer: 0.0,
            title_dirty: true,
            last_viewport: viewport,
        };
        scene.sync_object_count(viewport);
        scene
    }

    fn handle_key(&mut self, key: KeyCode, viewport: (u32, u32)) {
        if viewport != self.last_viewport {
            self.reseed_positions(viewport);
            self.last_viewport = viewport;
            self.title_dirty = true;
        }

        let mut changed = false;
        match key {
            KeyCode::Digit0 => changed = self.push_digit('0'),
            KeyCode::Digit1 => changed = self.push_digit('1'),
            KeyCode::Digit2 => changed = self.push_digit('2'),
            KeyCode::Digit3 => changed = self.push_digit('3'),
            KeyCode::Digit4 => changed = self.push_digit('4'),
            KeyCode::Digit5 => changed = self.push_digit('5'),
            KeyCode::Digit6 => changed = self.push_digit('6'),
            KeyCode::Digit7 => changed = self.push_digit('7'),
            KeyCode::Digit8 => changed = self.push_digit('8'),
            KeyCode::Digit9 => changed = self.push_digit('9'),
            KeyCode::Backspace => {
                self.input_buffer.pop();
                changed = true;
            }
            KeyCode::Enter => {
                self.apply_input(viewport);
                changed = true;
            }
            KeyCode::ArrowUp => {
                self.target_objects = (self.target_objects + 250).min(MAX_OBJECTS);
                self.input_buffer = self.target_objects.to_string();
                self.sync_object_count(viewport);
                changed = true;
            }
            KeyCode::ArrowDown => {
                self.target_objects = self.target_objects.saturating_sub(250);
                self.input_buffer = self.target_objects.to_string();
                self.sync_object_count(viewport);
                changed = true;
            }
            KeyCode::ArrowRight => {
                self.speed_scale = (self.speed_scale + 0.1).min(5.0);
                changed = true;
            }
            KeyCode::ArrowLeft => {
                self.speed_scale = (self.speed_scale - 0.1).max(0.1);
                changed = true;
            }
            KeyCode::KeyW => {
                self.size_scale = (self.size_scale + 0.05).min(2.5);
                self.refresh_sizes();
                changed = true;
            }
            KeyCode::KeyS => {
                self.size_scale = (self.size_scale - 0.05).max(0.4);
                self.refresh_sizes();
                changed = true;
            }
            KeyCode::KeyR => {
                self.randomize_motion();
                changed = true;
            }
            _ => {}
        }

        if changed {
            self.title_dirty = true;
        }
    }

    fn update(&mut self, dt: f32, viewport: (u32, u32)) {
        if viewport != self.last_viewport {
            self.reseed_positions(viewport);
            self.last_viewport = viewport;
            self.title_dirty = true;
        }

        let sim_dt = dt.min(0.05);
        self.simulate(sim_dt, viewport);

        self.fps_timer += dt;
        self.fps_frames += 1;
        if self.fps_timer >= 0.25 {
            self.fps = self.fps_frames as f32 / self.fps_timer;
            self.fps_timer = 0.0;
            self.fps_frames = 0;
            self.title_dirty = true;
        }

        self.title_timer += dt;
    }

    fn needs_title_refresh(&self) -> bool {
        self.title_dirty || self.title_timer >= 0.25
    }

    fn make_title(&mut self) -> String {
        self.title_dirty = false;
        self.title_timer = 0.0;
        format!(
            "2D ECS Performance Test | FPS {:.1} | Objects {}/{} | Input '{}' + Enter | Arrows=Count/Speed, PgUp/PgDn=Size, R=Randomize",
            self.fps,
            self.objects.len(),
            self.target_objects,
            self.input_buffer
        )
    }

    fn world(&self) -> &World {
        &self.world
    }

    fn push_digit(&mut self, digit: char) -> bool {
        if self.input_buffer.len() >= 6 {
            return false;
        }
        self.input_buffer.push(digit);
        true
    }

    fn apply_input(&mut self, viewport: (u32, u32)) {
        if self.input_buffer.is_empty() {
            self.target_objects = 0;
        } else if let Ok(parsed) = self.input_buffer.parse::<isize>() {
            self.target_objects = parsed.clamp(0, MAX_OBJECTS as isize) as usize;
        }
        self.input_buffer = self.target_objects.to_string();
        self.sync_object_count(viewport);
    }

    fn sync_object_count(&mut self, viewport: (u32, u32)) {
        while self.objects.len() < self.target_objects {
            self.spawn_object(viewport);
        }

        while self.objects.len() > self.target_objects {
            if let Some(entity) = self.objects.pop() {
                self.world.destroy_entity(entity);
            }
        }
    }

    fn refresh_sizes(&mut self) {
        for &entity in &self.objects {
            let radius = self.object_radius(entity);
            if let Some(sprite) = self.world.get_component_mut::<SpriteComponent>(entity) {
                sprite.draw_size = (radius * 2.0, radius * 2.0);
            }
        }
    }

    fn randomize_motion(&mut self) {
        let entities = self.objects.clone();
        for entity in entities {
            let speed = self.rand_range(30.0, 420.0);
            let angle = self.rand_range(0.0, std::f32::consts::TAU);
            if let Some(velocity) = self.world.get_component_mut::<Velocity2D>(entity) {
                velocity.x = speed * angle.cos();
                velocity.y = speed * angle.sin();
            }
        }
    }

    fn reseed_positions(&mut self, viewport: (u32, u32)) {
        let entities = self.objects.clone();
        for entity in entities {
            let radius = self.object_radius(entity);
            let max_x = (viewport.0 as f32 - radius * 2.0).max(0.0);
            let max_y = (viewport.1 as f32 - radius * 2.0).max(0.0);
            let x = self.rand_range(0.0, max_x);
            let y = self.rand_range(0.0, max_y);

            if let Some(transform) = self.world.get_component_mut::<Transform2D>(entity) {
                transform.position = (x, y);
            }
        }
    }

    fn spawn_object(&mut self, viewport: (u32, u32)) {
        let radius = self.rand_range(MIN_RADIUS, MAX_RADIUS);
        let x = self.rand_range(0.0, (viewport.0 as f32 - radius * 2.0).max(0.0));
        let y = self.rand_range(0.0, (viewport.1 as f32 - radius * 2.0).max(0.0));
        let speed = self.rand_range(30.0, 420.0);
        let angle = self.rand_range(0.0, std::f32::consts::TAU);
        let color_r = self.rand_range(0.25, 1.0);
        let color_g = self.rand_range(0.25, 1.0);
        let color_b = self.rand_range(0.25, 1.0);

        let entity = self.world.create_entity();
        self.world.add_component(entity, SimObjectTag);
        self.world.add_component(
            entity,
            Transform2D {
                position: (x, y),
                rotation: 0.0,
                scale: (1.0, 1.0),
            },
        );
        self.world.add_component(
            entity,
            SpriteComponent {
                texture_id: WHITE_TEXTURE,
                draw_size: (radius * self.size_scale * 2.0, radius * self.size_scale * 2.0),
                z: 2,
                tint: [color_r, color_g, color_b, 1.0],
                ..Default::default()
            },
        );
        self.world.add_component(
            entity,
            Velocity2D {
                x: speed * angle.cos(),
                y: speed * angle.sin(),
            },
        );
        self.objects.push(entity);
    }

    fn object_radius(&self, entity: Entity) -> f32 {
        self.world
            .get_component::<SpriteComponent>(entity)
            .map(|s| s.draw_size.0 * 0.5)
            .unwrap_or(5.0)
    }

    fn simulate(&mut self, dt: f32, viewport: (u32, u32)) {
        let max_x = viewport.0 as f32;
        let max_y = viewport.1 as f32;
        let speed_scale = self.speed_scale;

        for &entity in &self.objects {
            let radius = self.object_radius(entity);
            let mut vx = 0.0;
            let mut vy = 0.0;

            if let Some(v) = self.world.get_component::<Velocity2D>(entity) {
                vx = v.x;
                vy = v.y;
            }

            if let Some(transform) = self.world.get_component_mut::<Transform2D>(entity) {
                let mut x = transform.position.0 + vx * speed_scale * dt;
                let mut y = transform.position.1 + vy * speed_scale * dt;

                if x <= 0.0 {
                    x = 0.0;
                    vx = vx.abs();
                } else if x + radius * 2.0 >= max_x {
                    x = (max_x - radius * 2.0).max(0.0);
                    vx = -vx.abs();
                }

                if y <= 0.0 {
                    y = 0.0;
                    vy = vy.abs();
                } else if y + radius * 2.0 >= max_y {
                    y = (max_y - radius * 2.0).max(0.0);
                    vy = -vy.abs();
                }

                transform.position = (x, y);
            }

            if let Some(v) = self.world.get_component_mut::<Velocity2D>(entity) {
                v.x = vx;
                v.y = vy;
            }
        }
    }

    fn rand_range(&mut self, min: f32, max: f32) -> f32 {
        if max <= min {
            return min;
        }
        self.rng_state = self
            .rng_state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1);
        let unit = ((self.rng_state >> 33) as u32) as f32 / u32::MAX as f32;
        min + (max - min) * unit
    }
}

fn load_textures(renderer: &mut rusty::render::Renderer2D) {
    let white = [255, 255, 255, 255];
    if let Err(err) = renderer.load_texture_rgba(WHITE_TEXTURE, 1, 1, &white) {
        eprintln!("Failed to load simulation texture: {err}");
    }
    if let Err(err) = renderer.load_texture_rgba(UI_TEXTURE, 1, 1, &white) {
        eprintln!("Failed to load ui texture: {err}");
    }
}

fn main() {
    let scene = Rc::new(RefCell::new(None::<PerfScene>));
    let scene_for_init = Rc::clone(&scene);
    let scene_for_keys = Rc::clone(&scene);
    let scene_for_frame = Rc::clone(&scene);

    let run_result = run_2d_game(
        "2D ECS Performance Test",
        move |window, renderer| {
            renderer.set_background([0.03, 0.03, 0.06, 1.0]);
            load_textures(renderer);
            let initial_view = window.inner_size();
            *scene_for_init.borrow_mut() =
                Some(PerfScene::new((initial_view.width, initial_view.height)));
            Ok(RuntimeControl::Continue)
        },
        move |key, _window, renderer| {
            let viewport = renderer.camera_mut().viewport;
            if let Some(scene) = scene_for_keys.borrow_mut().as_mut() {
                scene.handle_key(key, viewport);
            }
            RuntimeControl::Continue
        },
        move |dt, viewport, window, renderer| {
            if let Some(scene) = scene_for_frame.borrow_mut().as_mut() {
                scene.update(dt, viewport);
                if scene.needs_title_refresh() {
                    window.set_title(&scene.make_title());
                }
                renderer.render_world(scene.world())?;
            }
            Ok(RuntimeControl::Continue)
        },
    );

    if let Err(err) = run_result {
        eprintln!("Event loop failed: {err}");
    }
}