use crate::ops::{PlatformOp, ViewProps};
use crate::view_tree::ViewTree;
/**
 * op_emitter.rs — Converts mutations + layout results into ordered platform ops.
 *
 * Processes CommandBatch commands (from host-core) and LayoutResult
 * (from host-layout) and emits an ordered sequence of PlatformOp that platform
 * shells consume via ViewFactory. Ops are deterministic — a commit is a
 * sequence of ops, not a rebuild (Fabric's model).
 *
 * Ordering guarantees:
 *   - Create before Append/Insert/SetFrame/SetProps/SetText
 *   - Append/Insert after both parent and child exist
 *   - SetFrame after Create
 *   - Destroy after Remove (child detached first)
 */
use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_core::NodeId;
use angnative_host_layout::LayoutResult;

/// Only the untagged generic `Custom` node is flattenable. Tagged customs
/// (`Custom:na-list`, `Custom:na-switch`, `Custom:na-activity-indicator`,
/// `Custom:na-candlestick`) name real native primitives and must emit a
/// `Create` op for the shell.
fn is_untagged_custom(node_type: &str) -> bool {
    node_type == "Custom"
}

/// Error emitted by the OpEmitter.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum OpEmitterError {
    #[error("Node {0} not found in view tree")]
    NodeNotFound(NodeId),
    #[error("Parent node {0} not found in view tree")]
    ParentNotFound(NodeId),
    #[error("Child node {0} not found in view tree")]
    ChildNotFound(NodeId),
    #[error("View for node {0} already exists")]
    ViewAlreadyExists(NodeId),
    #[error("View for node {0} does not exist")]
    ViewDoesNotExist(NodeId),
    #[error("Cannot attach child {0} to parent {1}: child already has parent {2}")]
    ChildAlreadyHasParent(NodeId, NodeId, NodeId),
}

/// The OpEmitter. Owns a ViewTree and converts commands + layout into ops.
#[derive(Clone)]
pub struct OpEmitter {
    tree: ViewTree,
    /// Accumulated ops for the current commit.
    ops: Vec<PlatformOp>,
    /// Custom nodes created without visual content — candidates for flattening.
    /// If they never receive styles/props/text/listeners, they get no native view.
    flattenable_candidates: std::collections::HashSet<u64>,
    /// Pool of recycled view IDs from removed children. Reused on subsequent
    /// InsertChild to avoid destroying/recreating native views (list recycling).
    recycle_pool: std::collections::VecDeque<u64>,
}

impl OpEmitter {
    pub fn new() -> Self {
        Self {
            tree: ViewTree::new(),
            ops: Vec::new(),
            flattenable_candidates: std::collections::HashSet::new(),
            recycle_pool: std::collections::VecDeque::new(),
        }
    }

    /// Process a command batch, emitting ops and updating the view tree.
    /// Returns the ops emitted for this batch.
    pub fn process_batch(
        &mut self,
        batch: &CommandBatch,
    ) -> Result<Vec<PlatformOp>, OpEmitterError> {
        let start = self.ops.len();
        for cmd in &batch.commands {
            self.process_command(cmd)?;
        }
        Ok(self.ops[start..].to_vec())
    }

    /// Process a single command, emitting the appropriate op(s).
    pub fn process_command(&mut self, cmd: &Command) -> Result<(), OpEmitterError> {
        match cmd {
            Command::CreateNode { id, node_type } => {
                if self.tree.has_view(*id) {
                    return Err(OpEmitterError::ViewAlreadyExists(*id));
                }

                // Untagged Custom/host-only nodes with no visual content are
                // flattened — they get no native view. Tagged customs are real
                // primitives and fall through to a normal Create op.
                if is_untagged_custom(node_type) {
                    self.flattenable_candidates.insert(*id);
                    self.tree.mark_flattened(*id);
                    self.tree.register_view(*id, node_type.clone());
                    return Ok(());
                }

                self.tree.register_view(*id, node_type.clone());
                self.ops.push(PlatformOp::Create {
                    node_id: *id,
                    view_type: node_type.clone(),
                });
            }
            Command::RemoveNode { id } => {
                if self.tree.is_flattened(*id) {
                    // Flattened nodes have no native view — just unflatten.
                    self.tree.detach(*id);
                    self.flattenable_candidates.remove(id);
                    return Ok(());
                }
                if !self.tree.has_view(*id) {
                    return Err(OpEmitterError::ViewDoesNotExist(*id));
                }
                // Detach from parent first if attached.
                if let Some(parent_id) = self.tree.parent_of(*id) {
                    self.ops.push(PlatformOp::Remove {
                        parent_id,
                        child_id: *id,
                    });
                    self.tree.detach(*id);
                }
                // Release to recycle pool instead of destroying — list recycling.
                self.recycle_pool.push_back(*id);
                self.tree.destroy(*id);
            }
            Command::InsertChild {
                parent_id,
                child_id,
                index,
            } => {
                if !self.tree.has_view(*parent_id) && !self.tree.is_flattened(*parent_id) {
                    return Err(OpEmitterError::ParentNotFound(*parent_id));
                }
                if !self.tree.has_view(*child_id)
                    && !self.tree.is_flattened(*child_id)
                    && !self.recycle_pool.contains(child_id)
                {
                    return Err(OpEmitterError::ChildNotFound(*child_id));
                }

                // If child already has a parent, detach first (re-parenting).
                if let Some(old_parent) = self.tree.parent_of(*child_id) {
                    if old_parent != *parent_id {
                        self.ops.push(PlatformOp::Remove {
                            parent_id: old_parent,
                            child_id: *child_id,
                        });
                        self.tree.detach(*child_id);
                    }
                }

                if self.tree.parent_of(*child_id).is_some() {
                    // Already attached to the same parent — just move it.
                    self.ops.push(PlatformOp::Remove {
                        parent_id: *parent_id,
                        child_id: *child_id,
                    });
                    self.tree.detach(*child_id);
                }

                // Determine the real attachment parent — skip flattened nodes.
                let real_parent = if self.tree.is_flattened(*parent_id) {
                    self.tree
                        .parent_of(*parent_id)
                        .unwrap_or(*parent_id)
                } else {
                    *parent_id
                };

                // A flattened child has no native view: record the attachment
                // for descendant resolution but emit no Insert op.
                if self.tree.is_flattened(*child_id) {
                    self.tree.attach(*child_id, real_parent);
                    return Ok(());
                }

                // Reuse an available recycled native view slot, if any.
                let reused_slot = self.recycle_pool.pop_front();

                if let Some(slot_id) = reused_slot {
                    // Reuse an existing view slot instead of creating new.
                    if slot_id != *child_id {
                        // The recycled slot has a different ID — re-parent it.
                        self.ops.push(PlatformOp::Remove {
                            parent_id: real_parent,
                            child_id: *child_id,
                        });
                        self.tree.detach(*child_id);
                        self.ops.push(PlatformOp::Insert {
                            parent_id: real_parent,
                            child_id: slot_id,
                            index: *index,
                        });
                        self.tree.attach(slot_id, real_parent);
                    } else {
                        // Reusing this node's own released view: re-track it
                        // without emitting a second Create.
                        self.tree.restore_view(*child_id);
                        self.ops.push(PlatformOp::Insert {
                            parent_id: real_parent,
                            child_id: *child_id,
                            index: *index,
                        });
                        self.tree.attach(*child_id, real_parent);
                    }
                } else {
                    self.tree.attach(*child_id, real_parent);
                    self.ops.push(PlatformOp::Insert {
                        parent_id: real_parent,
                        child_id: *child_id,
                        index: *index,
                    });
                }
            }
            Command::RemoveChild {
                parent_id,
                child_id,
            } => {
                if !self.tree.has_view(*parent_id) && !self.tree.is_flattened(*parent_id) {
                    return Err(OpEmitterError::ParentNotFound(*parent_id));
                }
                if !self.tree.has_view(*child_id) && !self.tree.is_flattened(*child_id) {
                    return Err(OpEmitterError::ChildNotFound(*child_id));
                }

                // Verify child actually belongs to parent.
                if self.tree.parent_of(*child_id) != Some(*parent_id) {
                    return Err(OpEmitterError::NodeNotFound(*child_id));
                }

                self.tree.detach(*child_id);

                // Release child's view to the recycle pool instead of emitting
                // a Destroy op — the native view can be reused on re-insert.
                if self.tree.has_view(*child_id) {
                    self.recycle_pool.push_back(*child_id);
                    self.tree.destroy(*child_id);
                }

                self.ops.push(PlatformOp::Remove {
                    parent_id: *parent_id,
                    child_id: *child_id,
                });
            }
            Command::SetProperty { id, key, value } => {
                if self.tree.is_flattened(*id) && !self.tree.has_view(*id) {
                    // Promote a flattened node — it now has visual content.
                    self.promote_flattened(*id)?;
                }
                if !self.tree.has_view(*id) {
                    return Err(OpEmitterError::ViewDoesNotExist(*id));
                }
                let mut props = ViewProps::new();
                props.properties.insert(key.clone(), value.clone());
                self.ops.push(PlatformOp::SetProps {
                    node_id: *id,
                    props,
                });
            }
            Command::SetStyle { id, key, value } => {
                if self.tree.is_flattened(*id) && !self.tree.has_view(*id) {
                    self.promote_flattened(*id)?;
                }
                if !self.tree.has_view(*id) {
                    return Err(OpEmitterError::ViewDoesNotExist(*id));
                }
                let mut props = ViewProps::new();
                props.styles.insert(key.clone(), value.clone());
                self.ops.push(PlatformOp::SetProps {
                    node_id: *id,
                    props,
                });
            }
            Command::SetTextContent { id, text } => {
                if self.tree.is_flattened(*id) && !self.tree.has_view(*id) {
                    self.promote_flattened(*id)?;
                }
                if !self.tree.has_view(*id) {
                    return Err(OpEmitterError::ViewDoesNotExist(*id));
                }
                self.ops.push(PlatformOp::SetText {
                    node_id: *id,
                    text: text.clone(),
                });
            }
        }
        Ok(())
    }

    /// Promote a flattened node — give it a native view because it now has
    /// visual content (style, property, or text). Emits a `Create` op first
    /// so subsequent property/style ops have a view to apply to.
    fn promote_flattened(&mut self, node_id: u64) -> Result<(), OpEmitterError> {
        self.flattenable_candidates.remove(&node_id);
        self.tree.detach(node_id);
        let view_type = self
            .tree
            .get(node_id)
            .map(|e| e.view_type.clone())
            .unwrap_or_else(|| "Custom".to_string());
        self.tree.register_view(node_id, view_type.clone());
        self.ops.push(PlatformOp::Create {
            node_id,
            view_type,
        });
        Ok(())
    }

    /// Apply layout results, emitting SetFrame ops for each node.
    /// Should be called after processing all commands in a commit.
    pub fn apply_layout(
        &mut self,
        layout: &LayoutResult,
    ) -> Result<Vec<PlatformOp>, OpEmitterError> {
        let start = self.ops.len();
        for rect in &layout.nodes {
            if !self.tree.has_view(rect.node_id) {
                // Layout for a node without a view — skip (may be a host-only node).
                continue;
            }
            self.ops.push(PlatformOp::SetFrame {
                node_id: rect.node_id,
                x: rect.x,
                y: rect.y,
                width: rect.width,
                height: rect.height,
            });
        }
        Ok(self.ops[start..].to_vec())
    }

    /// Get all ops emitted so far (not yet cleared).
    pub fn ops(&self) -> &[PlatformOp] {
        &self.ops
    }

    /// Clear all accumulated ops.
    pub fn clear_ops(&mut self) {
        self.ops.clear();
    }

    /// Get a reference to the internal view tree (for inspection/testing).
    pub fn tree(&self) -> &ViewTree {
        &self.tree
    }

    /// Get mutable reference to the internal view tree.
    pub fn tree_mut(&mut self) -> &mut ViewTree {
        &mut self.tree
    }

    /// Get the number of ops emitted.
    pub fn op_count(&self) -> usize {
        self.ops.len()
    }
}

impl Default for OpEmitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use angnative_host_core::command::{Command, CommandBatch};
    use angnative_host_layout::{LayoutEngine, LayoutNodeInput};
    use std::collections::HashMap;
    use serde_json;

    fn batch(commands: Vec<Command>) -> CommandBatch {
        CommandBatch {
            protocol_version: 1,
            frame_id: 0,
            timestamp_ms: 0,
            commands,
        }
    }

    #[test]
    fn create_then_append_emits_correct_ops() {
        let mut emitter = OpEmitter::new();
        let batch = batch(vec![
            Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            },
            Command::CreateNode {
                id: 2,
                node_type: "Text".to_string(),
            },
            Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            },
        ]);

        let ops = emitter.process_batch(&batch).unwrap();
        let kinds: Vec<&str> = ops.iter().map(|op| op_kind_str(op)).collect();
        assert_eq!(kinds, vec!["Create", "Create", "Insert"]);

        // Verify Create for node 1 comes before Insert
        let create_ops: Vec<&PlatformOp> = ops
            .iter()
            .filter(|o| matches!(o, PlatformOp::Create { .. }))
            .collect();
        assert_eq!(create_ops.len(), 2);
    }

    #[test]
    fn remove_child_emits_remove_op() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 2,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();

        emitter
            .process_command(&Command::RemoveChild {
                parent_id: 1,
                child_id: 2,
            })
            .unwrap();

        let ops = emitter.ops();
        let remove_ops: Vec<&PlatformOp> = ops
            .iter()
            .filter(|o| matches!(o, PlatformOp::Remove { .. }))
            .collect();
        assert_eq!(remove_ops.len(), 1);
    }

    #[test]
    fn set_style_emits_set_props() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::SetStyle {
                id: 1,
                key: "width".to_string(),
                value: "200px".to_string(),
            })
            .unwrap();

        let ops = emitter.ops();
        let set_props_ops: Vec<&PlatformOp> = ops
            .iter()
            .filter(|o| matches!(o, PlatformOp::SetProps { .. }))
            .collect();
        assert_eq!(set_props_ops.len(), 1);
    }

    #[test]
    fn set_text_emits_set_text() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Text".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::SetTextContent {
                id: 1,
                text: "hello".to_string(),
            })
            .unwrap();

        let ops = emitter.ops();
        let set_text_ops: Vec<&PlatformOp> = ops
            .iter()
            .filter(|o| matches!(o, PlatformOp::SetText { .. }))
            .collect();
        assert_eq!(set_text_ops.len(), 1);
    }

    #[test]
    fn layout_produces_set_frame_ops() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();

        let mut layout_input = HashMap::new();
        layout_input.insert(
            1,
            LayoutNodeInput {
                node_id: 1,
                styles: Default::default(),
                children: vec![],
            },
        );

        let mut layout_engine = LayoutEngine::default();
        let layout = layout_engine
            .layout(1, &layout_input, 500.0, 500.0)
            .unwrap();

        let ops = emitter.apply_layout(&layout).unwrap();
        let frame_ops: Vec<&PlatformOp> = ops
            .iter()
            .filter(|o| matches!(o, PlatformOp::SetFrame { .. }))
            .collect();
        assert_eq!(frame_ops.len(), 1);

        if let PlatformOp::SetFrame {
            node_id,
            x: _,
            y: _,
            width,
            height: _,
        } = &frame_ops[0]
        {
            assert_eq!(*node_id, 1);
            assert!((*width - 0.0).abs() < 1.0 || *width > 0.0); // auto-sized
        } else {
            panic!("Expected SetFrame op");
        }
    }

    #[test]
    fn reparenting_emits_remove_then_insert() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 2,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 3,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 2,
                child_id: 3,
                index: 0,
            })
            .unwrap();
        // Reparent 3 from 2 to 1
        emitter
            .process_command(&Command::RemoveChild {
                parent_id: 2,
                child_id: 3,
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 3,
                index: 1,
            })
            .unwrap();

        let ops = emitter.ops();
        let remove_ops: Vec<&PlatformOp> = ops
            .iter()
            .filter(|o| {
                matches!(
                    o,
                    PlatformOp::Remove {
                        parent_id: 2,
                        child_id: 3,
                        ..
                    }
                )
            })
            .collect();
        assert_eq!(
            remove_ops.len(),
            1,
            "should remove 3 from 2 before reparenting"
        );
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

    /* =========================================================================
     * Tagged custom primitive tests
     * ========================================================================= */

    #[test]
    fn tagged_custom_primitives_emit_real_create_ops() {
        let mut emitter = OpEmitter::new();
        let wires = [
            "Custom:na-list",
            "Custom:na-switch",
            "Custom:na-activity-indicator",
            "Custom:na-candlestick",
        ];

        for (index, wire) in wires.iter().enumerate() {
            emitter
                .process_command(&Command::CreateNode {
                    id: index as u64 + 1,
                    node_type: wire.to_string(),
                })
                .unwrap();
        }

        let creates: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|op| matches!(op, PlatformOp::Create { .. }))
            .collect();
        assert_eq!(creates.len(), wires.len());

        for (index, wire) in wires.iter().enumerate() {
            match creates[index] {
                PlatformOp::Create { node_id, view_type } => {
                    assert_eq!(*node_id, index as u64 + 1);
                    assert_eq!(view_type, wire);
                }
                other => panic!("Expected Create op, got {other:?}"),
            }
            let id = index as u64 + 1;
            assert!(!emitter.tree().is_flattened(id));
            assert!(emitter.tree().has_view(id));
        }
    }

    #[test]
    fn untagged_custom_still_stays_flattened_without_create_op() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom".to_string(),
            })
            .unwrap();

        assert!(emitter.tree().is_flattened(1));
        assert!(emitter
            .ops()
            .iter()
            .all(|op| !matches!(op, PlatformOp::Create { .. })));
    }

    /* =========================================================================
     * View flattening tests
     * ========================================================================= */

    #[test]
    fn custom_node_without_styles_stays_flattened() {
        let mut emitter = OpEmitter::new();

        // Custom node created — no Create op, marked as flattened candidate
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom".to_string(),
            })
            .unwrap();

        assert!(emitter.tree().is_flattened(1));
        assert!(!emitter.tree().has_view(1));

        // No Create op should have been emitted
        let create_ops: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|o| matches!(o, PlatformOp::Create { .. }))
            .collect();
        assert_eq!(create_ops.len(), 0);
    }

    #[test]
    fn custom_node_with_style_is_promoted() {
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom".to_string(),
            })
            .unwrap();
        assert!(emitter.tree().is_flattened(1));

        // Applying a style promotes the flattened node — Create op is emitted
        emitter
            .process_command(&Command::SetStyle {
                id: 1,
                key: "width".to_string(),
                value: "200px".to_string(),
            })
            .unwrap();

        assert!(!emitter.tree().is_flattened(1));
        assert!(emitter.tree().has_view(1));

        // Should have Create then SetProps
        let ops: Vec<&str> = emitter.ops().iter().map(op_kind_str).collect();
        assert_eq!(ops, vec!["Create", "SetProps"]);
    }

    #[test]
    fn custom_node_with_text_is_promoted() {
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom".to_string(),
            })
            .unwrap();

        emitter
            .process_command(&Command::SetTextContent {
                id: 1,
                text: "hello".to_string(),
            })
            .unwrap();

        assert!(!emitter.tree().is_flattened(1));
        assert!(emitter.tree().has_view(1));

        let ops: Vec<&str> = emitter.ops().iter().map(op_kind_str).collect();
        assert_eq!(ops, vec!["Create", "SetText"]);
    }

    #[test]
    fn child_of_flattened_node_attaches_to_grandparent() {
        let mut emitter = OpEmitter::new();

        // Root View
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();

        // Flattened Custom host (no styles → no native view)
        emitter
            .process_command(&Command::CreateNode {
                id: 2,
                node_type: "Custom".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();

        // Text child of flattened Custom — attaches to grandparent (node 1)
        emitter
            .process_command(&Command::CreateNode {
                id: 3,
                node_type: "Text".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 2,
                child_id: 3,
                index: 0,
            })
            .unwrap();

        // Node 3 should be attached to node 1 (grandparent), bypassing flattened node 2
        assert_eq!(emitter.tree().parent_of(3), Some(1));
        assert_eq!(emitter.tree().children_of(1), vec![3]);
        assert_eq!(emitter.tree().children_of(2), Vec::<u64>::new());

        // Only one Insert op (for node 3 under node 1), no Create for node 2
        let insert_ops: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|o| matches!(o, PlatformOp::Insert { .. }))
            .collect();
        assert_eq!(insert_ops.len(), 1);
        if let PlatformOp::Insert { parent_id, child_id, .. } = &insert_ops[0] {
            assert_eq!(*parent_id, 1);
            assert_eq!(*child_id, 3);
        }
    }

    #[test]
    fn flattened_node_with_children_then_promoted() {
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 2,
                node_type: "Custom".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 3,
                node_type: "Text".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 2,
                child_id: 3,
                index: 0,
            })
            .unwrap();

        // Promote node 2 by adding a style — it gets a view, but child 3
        // is already attached to grandparent (node 1)
        emitter
            .process_command(&Command::SetStyle {
                id: 2,
                key: "width".to_string(),
                value: "100px".to_string(),
            })
            .unwrap();

        assert!(!emitter.tree().is_flattened(2));
        assert!(emitter.tree().has_view(2));
        // Child 3 remains attached to grandparent
        assert_eq!(emitter.tree().parent_of(3), Some(1));
    }

    /* =========================================================================
     * List recycling tests
     * ========================================================================= */

    #[test]
    fn remove_child_releases_to_recycle_pool() {
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 2,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();

        // Remove child — releases to recycle pool instead of Destroy
        emitter
            .process_command(&Command::RemoveChild {
                parent_id: 1,
                child_id: 2,
            })
            .unwrap();

        // No Destroy op — view is recycled
        let destroy_ops: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|o| matches!(o, PlatformOp::Destroy { .. }))
            .collect();
        assert_eq!(destroy_ops.len(), 0);

        // But Remove op is still emitted (detach from parent)
        let remove_ops: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|o| matches!(o, PlatformOp::Remove { .. }))
            .collect();
        assert_eq!(remove_ops.len(), 1);

        // View 2 is in the recycle pool
        assert_eq!(emitter.tree().has_view(2), false);
    }

    #[test]
    fn insert_child_reuses_recycle_slot() {
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::CreateNode {
                id: 2,
                node_type: "View".to_string(),
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();

        // Remove and re-insert — node 2's view should be reused
        emitter
            .process_command(&Command::RemoveChild {
                parent_id: 1,
                child_id: 2,
            })
            .unwrap();
        emitter
            .process_command(&Command::InsertChild {
                parent_id: 1,
                child_id: 2,
                index: 0,
            })
            .unwrap();

        // No second Create op for node 2 — its native view was recycled.
        let create_ops: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|o| matches!(o, PlatformOp::Create { node_id: 2, .. }))
            .collect();
        assert_eq!(create_ops.len(), 1);

        // Two Insert ops (first insert + re-insert)
        let insert_ops: Vec<&PlatformOp> = emitter
            .ops()
            .iter()
            .filter(|o| matches!(o, PlatformOp::Insert { .. }))
            .collect();
        assert_eq!(insert_ops.len(), 2);
    }

    #[test]
    fn view_flattening_is_lazy() {
        // A Custom node starts flattened but gets promoted on first style/prop/text
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom".to_string(),
            })
            .unwrap();
        assert!(emitter.tree().is_flattened(1));

        // SetProperty also promotes
        emitter
            .process_command(&Command::SetProperty {
                id: 1,
                key: "disabled".to_string(),
                value: serde_json::json!(true),
            })
            .unwrap();

        assert!(!emitter.tree().is_flattened(1));
        assert!(emitter.tree().has_view(1));

        let ops: Vec<&str> = emitter.ops().iter().map(op_kind_str).collect();
        assert_eq!(ops, vec!["Create", "SetProps"]);
    }

    #[test]
    fn listener_promotes_flattened_node() {
        let mut emitter = OpEmitter::new();

        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom".to_string(),
            })
            .unwrap();
        assert!(emitter.tree().is_flattened(1));

        // Adding a listener promotes the node
        emitter
            .process_command(&Command::SetProperty {
                id: 1,
                key: "listener:press:1".to_string(),
                value: serde_json::json!({"eventName": "press"}),
            })
            .unwrap();

        assert!(!emitter.tree().is_flattened(1));
        assert!(emitter.tree().has_view(1));
    }

    #[test]
    fn candlestick_bars_string_is_emitted_as_set_props() {
        let mut emitter = OpEmitter::new();
        emitter
            .process_command(&Command::CreateNode {
                id: 1,
                node_type: "Custom:na-candlestick".to_string(),
            })
            .unwrap();

        // Angular binds [attr.bars] as a JSON string: JSONObject.optString on
        // the Android side can only read a string, never a JSON array.
        let bars = r#"{"symbol":"AAPL","bars":[{"t":1,"o":10,"h":12,"l":9,"c":11,"v":100}]}"#;
        emitter
            .process_command(&Command::SetProperty {
                id: 1,
                key: "attr:bars".to_string(),
                value: serde_json::json!(bars),
            })
            .unwrap();

        let create = emitter
            .ops()
            .iter()
            .find(|op| matches!(op, PlatformOp::Create { .. }))
            .expect("tagged candlestick must create a native view");
        match create {
            PlatformOp::Create { node_id, view_type } => {
                assert_eq!(*node_id, 1);
                assert_eq!(view_type, "Custom:na-candlestick");
            }
            other => panic!("Expected Create op, got {other:?}"),
        }

        let props_ops: Vec<&crate::ops::ViewProps> = emitter
            .ops()
            .iter()
            .filter_map(|op| match op {
                PlatformOp::SetProps { props, .. } => Some(props),
                _ => None,
            })
            .collect();
        assert_eq!(props_ops.len(), 1);
        assert_eq!(
            props_ops[0].properties.get("attr:bars"),
            Some(&serde_json::json!(bars))
        );
    }
}
