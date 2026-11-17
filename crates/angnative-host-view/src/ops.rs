/**
 * ops.rs — Platform operation types.
 *
 * These are the operations emitted by the OpEmitter and consumed by platform
 * shells via the ViewFactory trait. They are deterministic and ordered — a
 * commit is a sequence of ops, not a rebuild (Fabric's model).
 *
 * Each op carries enough information for a platform shell to apply it to its
 * native view hierarchy without knowing about AngNative's internal tree.
 */
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A platform operation emitted during a layout/commit pass.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PlatformOp {
    /// Create a native view for the given element node.
    Create {
        node_id: u64,
        view_type: String, // e.g. "View", "Text", "Button", "TextInput"
    },
    /// Set the frame (geometry) of a native view from layout results.
    SetFrame {
        node_id: u64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    },
    /// Set properties on a native view (text content, styles, etc.).
    SetProps { node_id: u64, props: ViewProps },
    /// Set text content on a text node.
    SetText { node_id: u64, text: String },
    /// Append a child view to a parent.
    Append { parent_id: u64, child_id: u64 },
    /// Insert a child view at a specific index.
    Insert {
        parent_id: u64,
        child_id: u64,
        index: usize,
    },
    /// Remove a child view from a parent.
    Remove { parent_id: u64, child_id: u64 },
    /// Destroy a native view (remove from tree and deallocate).
    Destroy { node_id: u64 },
}

/// Properties that can be set on a native view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ViewProps {
    pub text: Option<String>,
    pub styles: HashMap<String, String>,
    pub properties: HashMap<String, serde_json::Value>,
}

impl ViewProps {
    pub fn new() -> Self {
        Self::default()
    }
}

/// What the op targets — used for diagnostics and replay.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OpTarget {
    Node(u64),
    ParentChild { parent: u64, child: u64 },
}
