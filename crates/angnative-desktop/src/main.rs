/**
 * main.rs — AngNative desktop shell entry point.
 *
 * Creates a winit window, initializes the Rust host bridge, sends a demo
 * command batch (counter app), runs layout, and renders via wgpu.
 *
 * Usage:
 *   cargo run --release --package angnative-desktop
 */

use angnative_desktop::DemoApp;
use pollster::block_on;
use winit::event::{ElementState, Event, MouseButton};
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Initialize the demo app asynchronously (wgpu device creation is async)
    let mut app = block_on(DemoApp::new_with_event_loop(&event_loop))
        .expect("Failed to initialize AngNative desktop app");

    let mut cursor = (0.0f32, 0.0f32);
    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    winit::event::WindowEvent::CursorMoved { position, .. } => {
                        cursor = (position.x as f32, position.y as f32);
                    }
                    winit::event::WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        button: MouseButton::Left,
                        ..
                    } => {
                        app.on_click(cursor.0, cursor.1);
                    }
                    winit::event::WindowEvent::MouseInput {
                        state: ElementState::Released,
                        button: MouseButton::Left,
                        ..
                    } => {
                        app.release_button();
                    }
                    winit::event::WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    winit::event::WindowEvent::Resized(new_size) => {
                        app.window_mut().resize(new_size);
                    }
                    winit::event::WindowEvent::RedrawRequested => {
                        if let Err(e) = app.render() {
                            eprintln!("Render error: {e}");
                            elwt.exit();
                        }
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                app.window_mut().request_redraw();
            }
            _ => {}
        }
    }).unwrap();
}
