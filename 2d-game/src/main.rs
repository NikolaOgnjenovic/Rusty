use rusty_ecs_core::render::{Renderer2D, SpriteComponent, TextureId, Transform2D};
use rusty_ecs_core::{Entity, World};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Instant;
use winit::event::{ElementState, Event, KeyEvent, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::WindowAttributes;

const PLAYER_TEXTURE: TextureId = TextureId(1);
const OBSTACLE_TEXTURE: TextureId = TextureId(2);
const GROUND_TEXTURE: TextureId = TextureId(3);

const GROUND_HEIGHT: f32 = 28.0;
const PLAYER_X: f32 = 120.0;
const PLAYER_SIZE: (f32, f32) = (42.0, 42.0);
const OBSTACLE_SIZE: (f32, f32) = (30.0, 50.0);

const GRAVITY: f32 = 1900.0;
const JUMP_VELOCITY: f32 = -700.0;
const START_SPEED: f32 = 260.0;
const SPEED_GAIN_PER_SECOND: f32 = 4.0;
const MIN_SPAWN_DISTANCE: f32 = 185.0;
const MAX_SPAWN_DISTANCE_CAP: f32 = 340.0;

#[derive(Clone, Copy, Serialize, Deserialize)]
struct VelocityY(f32);

#[derive(Clone, Copy, Serialize, Deserialize)]
struct RunnerTag;

#[derive(Clone, Copy, Serialize, Deserialize)]
struct ObstacleTag;

struct RunnerGame {
    world: World,
    ground: Entity,
    player: Entity,
    obstacles: Vec<Entity>,
    speed: f32,
    spawn_timer: f32,
    next_obstacle_x: f32,
    score_time: f32,
    is_game_over: bool,
    jump_queued: bool,
    rng_state: u64,
}

impl RunnerGame {
    fn ground_top(viewport: (u32, u32)) -> f32 {
        viewport.1 as f32 - GROUND_HEIGHT
    }

    fn grounded_y(viewport: (u32, u32), sprite_height: f32) -> f32 {
        Self::ground_top(viewport) - sprite_height
    }

    fn current_ground_top(&self) -> f32 {
        self.world
            .get_component::<Transform2D>(self.ground)
            .map(|t| t.position.1)
            .unwrap_or(0.0)
    }

    fn new(viewport: (u32, u32)) -> Self {
        let mut world = World::new();
        let ground_top = Self::ground_top(viewport);
        let ground = world.create_entity();
        world.add_component(
            ground,
            Transform2D {
                position: (0.0, ground_top),
                rotation: 0.0,
                scale: (1.0, 1.0),
            },
        );
        world.add_component(
            ground,
            SpriteComponent {
                texture_id: GROUND_TEXTURE,
                draw_size: (viewport.0 as f32, GROUND_HEIGHT),
                z: 1,
                ..Default::default()
            },
        );

        let player = world.create_entity();
        world.add_component(player, RunnerTag);
        world.add_component(player, VelocityY(0.0));
        world.add_component(
            player,
            Transform2D {
                position: (PLAYER_X, Self::grounded_y(viewport, PLAYER_SIZE.1)),
                rotation: 0.0,
                scale: (1.0, 1.0),
            },
        );
        world.add_component(
            player,
            SpriteComponent {
                texture_id: PLAYER_TEXTURE,
                draw_size: PLAYER_SIZE,
                z: 2,
                ..Default::default()
            },
        );

        let mut game = Self {
            world,
            ground,
            player,
            obstacles: Vec::new(),
            speed: START_SPEED,
            spawn_timer: 0.0,
            next_obstacle_x: viewport.0 as f32 + 80.0,
            score_time: 0.0,
            is_game_over: false,
            jump_queued: false,
            rng_state: 0x9E37_79B9_7F4A_7C15,
        };
        game.schedule_next_spawn();
        game.spawn_obstacle(viewport.0 as f32 + 80.0);
        game
    }

    fn queue_jump(&mut self) {
        self.jump_queued = true;
    }

    fn restart(&mut self, viewport: (u32, u32)) {
        for obstacle in self.obstacles.drain(..) {
            self.world.destroy_entity(obstacle);
        }
        self.speed = START_SPEED;
        self.spawn_timer = 0.0;
        self.next_obstacle_x = 0.0;
        self.score_time = 0.0;
        self.is_game_over = false;
        self.jump_queued = false;

        self.update_ground(viewport);
        if let Some(transform) = self.world.get_component_mut::<Transform2D>(self.player) {
            transform.position = (PLAYER_X, Self::grounded_y(viewport, PLAYER_SIZE.1));
        }
        if let Some(vy) = self.world.get_component_mut::<VelocityY>(self.player) {
            vy.0 = 0.0;
        }

        self.next_obstacle_x = PLAYER_X + 420.0;
        self.schedule_next_spawn();
    }

    fn update(&mut self, dt: f32, viewport: (u32, u32)) {
        self.update_ground(viewport);

        if self.is_game_over {
            self.jump_queued = false;
            return;
        }

        self.score_time += dt;
        self.speed += SPEED_GAIN_PER_SECOND * dt;
        self.spawn_timer -= dt;
        if self.spawn_timer <= 0.0 {
            let spawn_x = self.next_obstacle_x.max(viewport.0 as f32 + 20.0);
            self.spawn_obstacle(spawn_x);
            self.schedule_next_spawn();
        }

        self.update_player_physics(dt);
        self.update_obstacles(dt);
        self.check_collisions();
        self.jump_queued = false;
    }

    fn update_player_physics(&mut self, dt: f32) {
        let player_ground_y = self.current_ground_top() - PLAYER_SIZE.1;
        let mut on_ground = false;
        if let Some(transform) = self.world.get_component::<Transform2D>(self.player) {
            on_ground = transform.position.1 >= player_ground_y;
        }

        if self.jump_queued && on_ground {
            if let Some(vy) = self.world.get_component_mut::<VelocityY>(self.player) {
                vy.0 = JUMP_VELOCITY;
            }
        }

        let mut velocity = 0.0;
        if let Some(vy) = self.world.get_component_mut::<VelocityY>(self.player) {
            vy.0 += GRAVITY * dt;
            velocity = vy.0;
        }

        if let Some(transform) = self.world.get_component_mut::<Transform2D>(self.player) {
            transform.position.1 += velocity * dt;
            if transform.position.1 > player_ground_y {
                transform.position.1 = player_ground_y;
                if let Some(vy) = self.world.get_component_mut::<VelocityY>(self.player) {
                    vy.0 = 0.0;
                }
            }
        }
    }

    fn update_obstacles(&mut self, dt: f32) {
        let mut to_destroy = Vec::new();
        for &entity in &self.obstacles {
            if let Some(transform) = self.world.get_component_mut::<Transform2D>(entity) {
                transform.position.0 -= self.speed * dt;
                if transform.position.0 < -OBSTACLE_SIZE.0 - 10.0 {
                    to_destroy.push(entity);
                }
            }
        }

        if !to_destroy.is_empty() {
            self.obstacles.retain(|entity| !to_destroy.contains(entity));
            for entity in to_destroy {
                self.world.destroy_entity(entity);
            }
        }
    }

    fn check_collisions(&mut self) {
        let Some(player_transform) = self.world.get_component::<Transform2D>(self.player) else {
            return;
        };
        let player_rect = rect(player_transform.position, PLAYER_SIZE);

        for &obstacle in &self.obstacles {
            let Some(obstacle_transform) = self.world.get_component::<Transform2D>(obstacle) else {
                continue;
            };
            let obstacle_rect = rect(obstacle_transform.position, OBSTACLE_SIZE);
            if intersects(player_rect, obstacle_rect) {
                self.is_game_over = true;
                break;
            }
        }
    }

    fn spawn_obstacle(&mut self, x: f32) {
        let obstacle_y = self.current_ground_top() - OBSTACLE_SIZE.1;
        let entity = self.world.create_entity();
        self.world.add_component(entity, ObstacleTag);
        self.world.add_component(
            entity,
            Transform2D {
                position: (x, obstacle_y),
                rotation: 0.0,
                scale: (1.0, 1.0),
            },
        );
        self.world.add_component(
            entity,
            SpriteComponent {
                texture_id: OBSTACLE_TEXTURE,
                draw_size: OBSTACLE_SIZE,
                z: 2,
                ..Default::default()
            },
        );
        self.obstacles.push(entity);
    }

    fn schedule_next_spawn(&mut self) {
        let hang_time = (-2.0 * JUMP_VELOCITY / GRAVITY).max(0.15);
        let reachable_distance = (self.speed * hang_time * 0.9).min(MAX_SPAWN_DISTANCE_CAP);
        let max_distance = reachable_distance.max(MIN_SPAWN_DISTANCE + 20.0);
        let min_distance = MIN_SPAWN_DISTANCE.min(max_distance - 1.0);
        let distance = self.rand_range(min_distance, max_distance);

        self.next_obstacle_x += distance;
        self.spawn_timer = distance / self.speed.max(1.0);
    }

    fn update_ground(&mut self, viewport: (u32, u32)) {
        let ground_top = Self::ground_top(viewport);
        if let Some(transform) = self.world.get_component_mut::<Transform2D>(self.ground) {
            transform.position = (0.0, ground_top);
        }
        if let Some(sprite) = self.world.get_component_mut::<SpriteComponent>(self.ground) {
            sprite.draw_size = (viewport.0 as f32, GROUND_HEIGHT);
        }

        if let Some(player_transform) = self.world.get_component_mut::<Transform2D>(self.player) {
            let player_bottom = player_transform.position.1 + PLAYER_SIZE.1;
            if player_bottom > ground_top {
                player_transform.position.1 = ground_top - PLAYER_SIZE.1;
                if let Some(vy) = self.world.get_component_mut::<VelocityY>(self.player) {
                    vy.0 = 0.0;
                }
            }
        }

        for &entity in &self.obstacles {
            if let Some(transform) = self.world.get_component_mut::<Transform2D>(entity) {
                transform.position.1 = ground_top - OBSTACLE_SIZE.1;
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
        let value = ((self.rng_state >> 33) as u32) as f32 / (u32::MAX as f32);
        min + (max - min) * value
    }

    fn world(&self) -> &World {
        &self.world
    }
}

fn rect(position: (f32, f32), size: (f32, f32)) -> (f32, f32, f32, f32) {
    (position.0, position.1, position.0 + size.0, position.1 + size.1)
}

fn intersects(a: (f32, f32, f32, f32), b: (f32, f32, f32, f32)) -> bool {
    a.0 < b.2 && a.2 > b.0 && a.1 < b.3 && a.3 > b.1
}

fn load_game_textures(renderer: &mut Renderer2D) {
    let player_path = Path::new("src\\images\\knight.png");
    let obstacle_path = Path::new("src\\images\\ftn.jpg");

    if let Err(err) = renderer.load_texture(PLAYER_TEXTURE, player_path.to_string_lossy().as_ref()) {
        eprintln!("Failed to load player sprite ({}): {err}", player_path.display());
    }
    if let Err(err) = renderer.load_texture(OBSTACLE_TEXTURE, obstacle_path.to_string_lossy().as_ref()) {
        eprintln!("Failed to load obstacle sprite ({}): {err}", obstacle_path.display());
    }

    let ground_rgba = [
        55, 44, 28, 255, 66, 53, 35, 255, 66, 53, 35, 255, 55, 44, 28, 255,
    ];
    if let Err(err) = renderer.load_texture_rgba(GROUND_TEXTURE, 2, 2, &ground_rgba) {
        eprintln!("Failed to load ground texture: {err}");
    }
}

fn main() {
    let event_loop = match EventLoop::new() {
        Ok(loop_ref) => loop_ref,
        Err(err) => {
            eprintln!("Failed to create event loop: {err}");
            return;
        }
    };

    let window = match event_loop.create_window(
        WindowAttributes::default().with_title("Rusty ECS 2D Runner (press Space to jump)"),
    ) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Failed to create window: {err}");
            return;
        }
    };

    let mut renderer = match Renderer2D::new(&window) {
        Ok(value) => value,
        Err(err) => {
            eprintln!("Failed to create renderer: {err}");
            return;
        }
    };
    renderer.set_background([0.08, 0.08, 0.1, 1.0]);
    load_game_textures(&mut renderer);

    let viewport = window.inner_size();
    let mut game = RunnerGame::new((viewport.width, viewport.height));
    let mut last_frame = Instant::now();

    let run_result = event_loop.run(move |event, target| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => target.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        physical_key,
                        state,
                        ..
                    },
                ..
            } => {
                if state == ElementState::Pressed {
                    match physical_key {
                        PhysicalKey::Code(KeyCode::Space) => {
                            if game.is_game_over {
                                let size = window.inner_size();
                                game.restart((size.width, size.height));
                            } else {
                                game.queue_jump();
                            }
                        }
                        PhysicalKey::Code(KeyCode::KeyR) => {
                            let size = window.inner_size();
                            game.restart((size.width, size.height));
                        }
                        _ => {}
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - last_frame).as_secs_f32().min(0.05);
                last_frame = now;

                let size = window.inner_size();
                game.update(dt, (size.width, size.height));
                let _ = renderer.render_world(game.world());
            }
            _ => {}
        },
        Event::AboutToWait => window.request_redraw(),
        _ => {}
    });

    if let Err(err) = run_result {
        eprintln!("Event loop failed: {err}");
    }
}