use crate::render::renderer::{RenderError, Renderer2D};
use std::time::Instant;
use winit::event::{ElementState, Event, WindowEvent};
use winit::event_loop::EventLoop;
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowAttributes};

/// Control signal used by runtime callbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeControl {
    Continue,
    Exit,
}

/// Creates a default window and renderer pair for 2D games.
pub fn init_2d_window_and_renderer(
    title: &str,
) -> Result<(EventLoop<()>, Window, Renderer2D), RenderError> {
    let event_loop = EventLoop::new().map_err(|err| RenderError::SurfaceCreation(err.to_string()))?;
    let window = event_loop
        .create_window(WindowAttributes::default().with_title(title))
        .map_err(|err| RenderError::SurfaceCreation(err.to_string()))?;
    let renderer = Renderer2D::new(&window)?;
    Ok((event_loop, window, renderer))
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

/// Runs a 2D game event loop and delegates game-specific logic via callbacks.
pub fn run_2d_game<FK, FF>(
    event_loop: EventLoop<()>,
    window: Window,
    mut renderer: Renderer2D,
    mut on_key_pressed: FK,
    mut on_frame: FF,
) -> Result<(), winit::error::EventLoopError>
where
    FK: FnMut(KeyCode, &Window, &mut Renderer2D) -> RuntimeControl + 'static,
    FF: FnMut(f32, (u32, u32), &mut Renderer2D) -> Result<RuntimeControl, RenderError> + 'static,
{
    let mut last_frame = Instant::now();
    event_loop.run(move |event, target| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => target.exit(),
            WindowEvent::Resized(size) => renderer.resize(size.width, size.height),
            WindowEvent::RedrawRequested => {
                let now = Instant::now();
                let dt = (now - last_frame).as_secs_f32().min(0.05);
                last_frame = now;

                let size = window.inner_size();
                match on_frame(dt, (size.width, size.height), &mut renderer) {
                    Ok(RuntimeControl::Continue) => {}
                    Ok(RuntimeControl::Exit) => target.exit(),
                    Err(err) => {
                        eprintln!("Frame update/render failed: {err}");
                        target.exit();
                    }
                }
            }
            other => {
                if let Some(code) = pressed_key_code(&other) {
                    if on_key_pressed(code, &window, &mut renderer) == RuntimeControl::Exit {
                        target.exit();
                    }
                }
            }
        },
        Event::AboutToWait => window.request_redraw(),
        _ => {}
    })
}
