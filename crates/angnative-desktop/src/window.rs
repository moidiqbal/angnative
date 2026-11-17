/**
 * window.rs — AngNativeDesktop window wrapper.
 *
 * Combines winit window, wgpu renderer, and Rust host bridge into a single
 * desktop window that can render native AngNative view trees.
 */

use angnative_host_core::command::{Command, CommandBatch};
use angnative_desktop::{DesktopBridge, DesktopRenderer};
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowBuilder};

/**
 * AngNativeDesktop — the main desktop window.
 *
 * Owns the winit window, wgpu renderer, and Rust host bridge. Processes
 * command batches, runs layout, and renders via wgpu.
 */
pub struct AngNativeDesktop {
    window: Window,
    renderer: DesktopRenderer,
    bridge: DesktopBridge,
}

impl AngNativeDesktop {
    /// Create a new desktop window.
    pub async fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Result<Self, String> {
        let window = WindowBuilder::new()
            .with_title("AngNative Desktop")
            .with_inner_size(PhysicalSize::new(800, 600))
            .with_resizable(true)
            .build(event_loop)
            .map_err(|e| format!("Failed to create window: {e}"))?;

        let renderer = DesktopRenderer::new(&window).await?;
        let bridge = DesktopBridge::new();

        Ok(Self {
            window,
            renderer,
            bridge,
        })
    }

    /// Get a reference to the winit window.
    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Get a mutable reference to the winit window.
    pub fn window_mut(&mut self) -> &mut Window {
        &mut self.window
    }

    /// Handle window resize.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        self.renderer.resize(new_size);
    }

    /// Request a redraw.
    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    /// Process a command batch through the Rust host.
    pub fn process_batch(&mut self, batch: &CommandBatch) -> Result<(), String> {
        let ops = self.bridge.process_batch(batch)?;
        // Apply ops to view tree (in a full implementation, this would
        // update native widgets; for now we just track them)
        let _ = ops;
        Ok(())
    }

    /// Run layout and render.
    pub fn render(&mut self) -> Result<(), String> {
        let layout = self.bridge.last_layout();
        self.renderer.render(layout)
    }

    /// Get a reference to the bridge (for sending commands).
    pub fn bridge_mut(&mut self) -> &mut DesktopBridge {
        &mut self.bridge
    }
}
