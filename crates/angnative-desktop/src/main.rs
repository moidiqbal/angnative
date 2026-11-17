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
use winit::event::Event;
use winit::event_loop::{ControlFlow, EventLoop};

fn main() {
    env_logger::init();

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Poll);

    // Initialize the demo app asynchronously (wgpu device creation is async)
    let mut app = block_on(DemoApp::new_with_event_loop(&event_loop))
        .expect("Failed to initialize AngNative desktop app");

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent { event, .. } => {
                match event {
                    winit::event::WindowEvent::CloseRequested => {
                        elwt.exit();
                    }
                    winit::event::WindowEvent::Resized(new_size) => {
                        app.window_mut().resize(new_size);
                    }
                    _ => {}
                }
            }
            Event::AboutToWait => {
                app.window_mut().request_redraw();
            }
            Event::RedrawRequested(_) => {
                match app.render() {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("Render error: {e}");
                        elwt.exit();
                    }
                }
            }
            _ => {}
        }
    }).unwrap();
}
