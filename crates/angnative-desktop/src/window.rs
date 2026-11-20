/**
 * window.rs — AngNativeDesktop window wrapper.
 *
 * Combines winit window, wgpu renderer, and Rust host bridge into a single
 * desktop window that can render native AngNative view trees. Ops emitted by
 * the host are applied to a retained local view tree instead of discarded.
 */

use crate::{DesktopBridge, DesktopRenderer, LocalViewTree};
use angnative_host_core::command::CommandBatch;
use angnative_host_layout::LayoutResult;
use std::collections::HashMap;
use winit::dpi::PhysicalSize;
use winit::window::{Window, WindowBuilder};

/**
 * AngNativeDesktop — the main desktop window.
 *
 * Owns the winit window, wgpu renderer, Rust host bridge, and a retained
 * view tree. Processes command batches, runs layout, applies the resulting
 * ops to the view tree, and renders via wgpu.
 */
pub struct AngNativeDesktop {
    // Declaration order is drop order: the renderer (and its GPU surface)
    // must be dropped before the window the surface was created from.
    renderer: DesktopRenderer,
    window: Window,
    bridge: DesktopBridge,
    view_tree: LocalViewTree,
    /// Interaction state forwarded to the renderer for visual feedback.
    counter: i32,
    pressed: Option<u64>,
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
            view_tree: LocalViewTree::new(),
            counter: 0,
            pressed: None,
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

    /// Process a command batch through the Rust host and apply the emitted
    /// ops to the retained local view tree.
    pub fn process_batch(&mut self, batch: &CommandBatch) -> Result<(), String> {
        let ops = self.bridge.process_batch(batch)?;
        self.view_tree.apply_ops(&ops);
        Ok(())
    }

    /// Run layout and remember the result for rendering.
    pub fn run_layout(
        &mut self,
        nodes: &HashMap<u64, angnative_host_layout::LayoutNodeInput>,
        root_id: u64,
        available_width: f32,
        available_height: f32,
    ) -> Result<LayoutResult, String> {
        self.bridge
            .run_layout(nodes, root_id, available_width, available_height)
    }

    /// Apply a computed layout as SetFrame ops to the retained view tree.
    pub fn apply_layout(&mut self, layout: &LayoutResult) -> Result<(), String> {
        let ops = self.bridge.apply_layout(layout)?;
        self.view_tree.apply_ops(&ops);
        Ok(())
    }

    /// Set interaction state used for button highlight + counter dots.
    pub fn set_visual_state(&mut self, counter: i32, pressed: Option<u64>) {
        self.counter = counter;
        self.pressed = pressed;
        self.renderer.set_state(counter, pressed);
    }

    /// Render the current frame.
    pub fn render(&mut self) -> Result<(), String> {
        let layout = self.bridge.last_layout();
        self.renderer.render(layout)
    }

    /// Inspect the retained local view tree (tests/diagnostics).
    pub fn view_tree(&self) -> &LocalViewTree {
        &self.view_tree
    }

    /// Get a reference to the bridge (for sending commands).
    pub fn bridge_mut(&mut self) -> &mut DesktopBridge {
        &mut self.bridge
    }
}
