pub mod op_emitter;
/**
 * lib.rs — angnative-host-view: View tree + ViewFactory trait + platform op emission.
 *
 * Mirrors the TypeScript renderer's tree management on the Rust side. Takes
 * mutations from the command protocol (host-core) + layout rectangles
 * (host-layout) and emits platform operations (Create/SetFrame/SetProps/
 * SetText/Append/Insert/Remove/Destroy) that platform shells consume.
 *
 * The ViewFactory trait is the SPI that platform shells implement:
 *   - Android: android.view.View / ViewGroup
 *   - iOS: UIView
 *   - macOS: NSView
 *   - Web: DOM elements
 *
 * Modules:
 *   - ops          — Platform operation types (Create/SetFrame/SetProps/...)
 *   - view_factory — ViewFactory trait SPI for platform shells
 *   - view_tree    — Retained view tree mapping element nodes to native views
 *   - op_emitter   — Converts mutations + layout into ordered platform ops
 */
pub mod ops;
pub mod view_factory;
pub mod view_tree;

pub use op_emitter::OpEmitter;
pub use ops::{OpTarget, PlatformOp, ViewProps};
pub use view_factory::ViewFactory;
pub use view_tree::ViewTree;
