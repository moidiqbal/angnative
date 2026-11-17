/**
 * lib.rs — angnative-desktop: Windows/macOS/Linux desktop shell.
 *
 * Implements a native desktop application shell using winit for window
 * management and wgpu for GPU-accelerated rendering. Connects to the Rust
 * host crates (host-core, host-layout, host-view) directly — no FFI needed
 * on desktop since everything is Rust.
 *
 * Architecture:
 *   winit window  →  winit event loop  →  AngNativeDesktop shell
 *                                    ↓
 *                           angnative-host-core (MutationEngine)
 *                           angnative-host-layout (Taffy layout)
 *                           angnative-host-view (OpEmitter + ViewFactory)
 *                                    ↓
 *                           wgpu GPU rendering
 *
 * No WebView, no Electron — pure native desktop rendering.
 */

pub mod window;
pub mod renderer;
pub mod bridge;
pub mod app;

pub use window::AngNativeDesktop;
pub use renderer::DesktopRenderer;
pub use bridge::DesktopBridge;
pub use app::DemoApp;
