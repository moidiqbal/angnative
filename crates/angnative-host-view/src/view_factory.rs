/**
 * view_factory.rs — ViewFactory trait SPI for platform shells.
 *
 * Platform shells implement this trait to map AngNative platform operations
 * to their native view hierarchy. The trait is engine-agnostic — the same
 * trait works for Android (android.view.View), iOS (UIView), macOS (NSView),
 * and Web (DOM).
 *
 * Each operation maps 1:1 to a native platform operation:
 *   - Create    → instantiate native view
 *   - SetFrame  → set native frame/bounds
 *   - SetProps  → set native properties/styles
 *   - SetText   → set native text content
 *   - Append    → addChild at end
 *   - Insert    → addChild at index
 *   - Remove    → removeChild
 *   - Destroy   → removeFromSuperview / dispose
 */
use crate::ops::{PlatformOp, ViewProps};
use std::sync::Mutex;

/// Result of a view factory operation.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ViewFactoryError {
    #[error("View not found: {0}")]
    ViewNotFound(u64),
    #[error("Parent view not found: {0}")]
    ParentNotFound(u64),
    #[error("Child view not found: {0}")]
    ChildNotFound(u64),
    #[error("View already has a parent: {0}")]
    ViewAlreadyHasParent(u64),
    #[error("Unsupported view type: {0}")]
    UnsupportedViewType(String),
    #[error("Operation failed: {0}")]
    OperationFailed(String),
}

pub type ViewFactoryResult<T> = Result<T, ViewFactoryError>;

/// The ViewFactory trait — platform shells implement this SPI.
///
/// Implementations are thin: they translate PlatformOp into native view
/// calls without any AngNative business logic.
pub trait ViewFactory: Send + Sync {
    /// Create a native view for the given node.
    fn create_view(&self, node_id: u64, view_type: &str) -> ViewFactoryResult<()>;

    /// Set the frame (geometry) of a native view.
    fn set_frame(
        &self,
        node_id: u64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> ViewFactoryResult<()>;

    /// Set properties/styles on a native view.
    fn set_props(&self, node_id: u64, props: &ViewProps) -> ViewFactoryResult<()>;

    /// Set text content on a text node.
    fn set_text(&self, node_id: u64, text: &str) -> ViewFactoryResult<()>;

    /// Append a child view to a parent.
    fn append_child(&self, parent_id: u64, child_id: u64) -> ViewFactoryResult<()>;

    /// Insert a child view at a specific index.
    fn insert_child(&self, parent_id: u64, child_id: u64, index: usize) -> ViewFactoryResult<()>;

    /// Remove a child view from a parent.
    fn remove_child(&self, parent_id: u64, child_id: u64) -> ViewFactoryResult<()>;

    /// Destroy a native view.
    fn destroy_view(&self, node_id: u64) -> ViewFactoryResult<()>;

    /// Apply a single platform op. Default implementation dispatches to the
    /// appropriate method above.
    fn apply_op(&self, op: &PlatformOp) -> ViewFactoryResult<()> {
        match op {
            PlatformOp::Create { node_id, view_type } => self.create_view(*node_id, view_type),
            PlatformOp::SetFrame {
                node_id,
                x,
                y,
                width,
                height,
            } => self.set_frame(*node_id, *x, *y, *width, *height),
            PlatformOp::SetProps { node_id, props } => self.set_props(*node_id, props),
            PlatformOp::SetText { node_id, text } => self.set_text(*node_id, text),
            PlatformOp::Append {
                parent_id,
                child_id,
            } => self.append_child(*parent_id, *child_id),
            PlatformOp::Insert {
                parent_id,
                child_id,
                index,
            } => self.insert_child(*parent_id, *child_id, *index),
            PlatformOp::Remove {
                parent_id,
                child_id,
            } => self.remove_child(*parent_id, *child_id),
            PlatformOp::Destroy { node_id } => self.destroy_view(*node_id),
        }
    }

    /// Apply a sequence of platform ops in order. Returns on first error.
    fn apply_ops(&self, ops: &[PlatformOp]) -> ViewFactoryResult<()> {
        for op in ops {
            self.apply_op(op)?;
        }
        Ok(())
    }
}

/// A no-op ViewFactory for unit testing — records ops via interior mutability.
#[derive(Debug, Default)]
pub struct RecordingViewFactory {
    ops: Mutex<Vec<PlatformOp>>,
}

impl RecordingViewFactory {
    pub fn new() -> Self {
        Self {
            ops: Mutex::new(Vec::new()),
        }
    }

    /// Get a clone of all recorded ops.
    pub fn recorded_ops(&self) -> Vec<PlatformOp> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// Get ops of a specific kind.
    pub fn ops_of_kind(&self, kind: &str) -> Vec<PlatformOp> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .iter()
            .filter(|op| op_kind_str(op) == kind)
            .cloned()
            .collect()
    }

    /// Clear all recorded ops.
    pub fn clear(&self) {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();
    }
}

impl ViewFactory for RecordingViewFactory {
    fn create_view(&self, node_id: u64, view_type: &str) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::Create {
                node_id,
                view_type: view_type.to_string(),
            });
        Ok(())
    }

    fn set_frame(
        &self,
        node_id: u64,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::SetFrame {
                node_id,
                x,
                y,
                width,
                height,
            });
        Ok(())
    }

    fn set_props(&self, node_id: u64, props: &ViewProps) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::SetProps {
                node_id,
                props: props.clone(),
            });
        Ok(())
    }

    fn set_text(&self, node_id: u64, text: &str) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::SetText {
                node_id,
                text: text.to_string(),
            });
        Ok(())
    }

    fn append_child(&self, parent_id: u64, child_id: u64) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::Append {
                parent_id,
                child_id,
            });
        Ok(())
    }

    fn insert_child(&self, parent_id: u64, child_id: u64, index: usize) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::Insert {
                parent_id,
                child_id,
                index,
            });
        Ok(())
    }

    fn remove_child(&self, parent_id: u64, child_id: u64) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::Remove {
                parent_id,
                child_id,
            });
        Ok(())
    }

    fn destroy_view(&self, node_id: u64) -> ViewFactoryResult<()> {
        self.ops
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(PlatformOp::Destroy { node_id });
        Ok(())
    }
}

fn op_kind_str(op: &PlatformOp) -> &'static str {
    match op {
        PlatformOp::Create { .. } => "Create",
        PlatformOp::SetFrame { .. } => "SetFrame",
        PlatformOp::SetProps { .. } => "SetProps",
        PlatformOp::SetText { .. } => "SetText",
        PlatformOp::Append { .. } => "Append",
        PlatformOp::Insert { .. } => "Insert",
        PlatformOp::Remove { .. } => "Remove",
        PlatformOp::Destroy { .. } => "Destroy",
    }
}
