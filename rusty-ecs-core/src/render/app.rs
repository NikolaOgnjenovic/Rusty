use crate::render::renderer::{RenderError, Renderer2D};
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

/// Control signal used by runtime callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeControl {
    Continue,
    Exit,
}

/// Returns `Some(code)` only when a keyboard key was pressed.
pub fn pressed_key_code(event: &WindowEvent) -> Option<KeyCode> {
    match event {
        WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
            match event.physical_key {
                PhysicalKey::Code(code) => Some(code),
                _ => None,
            }
        }
        _ => None,
    }
}

struct GameApp<FI, FK, FF>
where
    FI: FnMut(&Window, &mut Renderer2D) -> Result<RuntimeControl, RenderError> + 'static,
    FK: FnMut(KeyCode, &Window, &mut Renderer2D) -> RuntimeControl + 'static,
    FF: FnMut(f32, (u32, u32), &Window, &mut Renderer2D) -> Result<RuntimeControl, RenderError>
        + 'static,
{
    title: String,
    window: Option<Window>,
    renderer: Option<Renderer2D>,
    last_frame: Instant,
    on_init: FI,
    on_key_pressed: FK,
    on_frame: FF,
}

impl<FI, FK, FF> ApplicationHandler for GameApp<FI, FK, FF>
where
    FI: FnMut(&Window, &mut Renderer2D) -> Result<RuntimeControl, RenderError> + 'static,
    FK: FnMut(KeyCode, &Window, &mut Renderer2D) -> RuntimeControl + 'static,
    FF: FnMut(f32, (u32, u32), &Window, &mut Renderer2D) -> Result<RuntimeControl, RenderError>
        + 'static,
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let window = match event_loop
            .create_window(WindowAttributes::default().with_title(self.title.clone()))
        {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Window creation failed: {err}");
                event_loop.exit();
                return;
            }
        };

        let mut renderer = match Renderer2D::new(&window) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Renderer initialization failed: {err}");
                event_loop.exit();
                return;
            }
        };

        match (self.on_init)(&window, &mut renderer) {
            Ok(RuntimeControl::Continue) => {}
            Ok(RuntimeControl::Exit) => {
                event_loop.exit();
                return;
            }
            Err(err) => {
                eprintln!("Initialization failed: {err}");
                event_loop.exit();
                return;
            }
        }

        self.last_frame = Instant::now();
        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let (window, renderer) = match (self.window.as_ref(), self.renderer.as_mut()) {
            (Some(window), Some(renderer)) if window.id() == window_id => (window, renderer),
            _ => return,
        };

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let frame_dt = (now - self.last_frame).as_secs_f32();
                self.last_frame = now;

                let size = window.inner_size();
                match (self.on_frame)(
                    frame_dt.max(0.0),
                    (size.width, size.height),
                    window,
                    renderer,
                ) {
                    Ok(RuntimeControl::Continue) => {}
                    Ok(RuntimeControl::Exit) => event_loop.exit(),
                    Err(err) => {
                        eprintln!("Frame update/render failed: {err}");
                        event_loop.exit();
                    }
                }
            }
            other => {
                if let Some(code) = pressed_key_code(&other) {
                    if (self.on_key_pressed)(code, window, renderer) == RuntimeControl::Exit {
                        event_loop.exit();
                    }
                }
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = self.window.as_ref() {
            window.request_redraw();
        }
    }
}

/// Runs a 2D game event loop and delegates game-specific logic via callbacks.
pub fn run_2d_game<FI, FK, FF>(
    title: &str,
    on_init: FI,
    on_key_pressed: FK,
    on_frame: FF,
) -> Result<(), winit::error::EventLoopError>
where
    FI: FnMut(&Window, &mut Renderer2D) -> Result<RuntimeControl, RenderError> + 'static,
    FK: FnMut(KeyCode, &Window, &mut Renderer2D) -> RuntimeControl + 'static,
    FF: FnMut(f32, (u32, u32), &Window, &mut Renderer2D) -> Result<RuntimeControl, RenderError>
        + 'static,
{
    let event_loop = EventLoop::new()?;
    let mut app = GameApp {
        title: title.to_string(),
        window: None,
        renderer: None,
        last_frame: Instant::now(),
        on_init,
        on_key_pressed,
        on_frame,
    };
    event_loop.run_app(&mut app)
}
