/**
 * mutation.rs — Apply command-protocol commands to the retained element tree.
 *
 * Mirrors the TypeScript `FakeBackend.apply()`
 * (`packages/testing/src/fake-backend.ts`). Each command mutates the tree and
 * is recorded as a mutation kind. Unknown node types, unknown style keys, and
 * non-member child removals throw loudly (spec §5.3).
 */
use crate::command::{validate_version, Command, CommandBatch, PROTOCOL_VERSION};
use crate::element_registry::ElementRegistry;
use crate::node::{NodeId, NodeType, ViewNode};
use serde::{Deserialize, Serialize};

/// Error returned when a command cannot be applied.
#[derive(Debug, thiserror::Error)]
pub enum MutationError {
    #[error("Unknown node type: {0}")]
    UnknownNodeType(String),
    #[error("No native element with id {0}")]
    NodeNotFound(NodeId),
    #[error("Element with id {0} is already registered")]
    DuplicateId(NodeId),
    #[error("Element {child_id} is not a child of element {parent_id}")]
    NotACheckedChild { child_id: NodeId, parent_id: NodeId },
    #[error("Unsupported protocol version {0}, expected {1}")]
    UnsupportedVersion(u64, u64),
}

/// The mutation engine. Owns an `ElementRegistry` and applies commands to it,
/// recording each mutation kind as it goes.
#[derive(Debug, Clone, Default)]
pub struct MutationEngine {
    registry: ElementRegistry,
    /// Accumulated mutation kinds per batch, in order.
    batches: Vec<Vec<String>>,
    current_batch: Vec<String>,
}

impl MutationEngine {
    pub fn new() -> Self {
        Self::default()
    }

    /// Execute a single command. Returns the mutation kind recorded.
    pub fn run_command(&mut self, cmd: Command) -> Result<String, MutationError> {
        let kind = self.apply(cmd)?;
        self.current_batch.push(kind.clone());
        Ok(kind)
    }

    /// Execute a command batch, then flush it.
    pub fn run_batch(&mut self, batch: CommandBatch) -> Result<(), MutationError> {
        validate_version(batch.protocol_version).map_err(|_| {
            MutationError::UnsupportedVersion(batch.protocol_version, PROTOCOL_VERSION)
        })?;
        for cmd in batch.commands {
            self.run_command(cmd)?;
        }
        self.flush_batch();
        Ok(())
    }

    /// Flush the accumulated mutation-kind batch.
    pub fn flush_batch(&mut self) {
        if !self.current_batch.is_empty() {
            self.batches.push(std::mem::take(&mut self.current_batch));
        }
    }

    /// Snapshot the full shadow tree for conformance assertions.
    pub fn snapshot(&mut self) -> TreeSnapshot {
        self.flush_batch();
        let mut nodes: Vec<NodeSnapshot> = Vec::new();
        for el in self.registry.iter() {
            nodes.push(NodeSnapshot {
                id: el.id,
                tag: el.node_type.canonical_tag().to_string(),
                node_type: el.node_type.wire_name(),
                parent_id: el.parent_id,
                children: el.children.clone(),
                properties: el.properties.clone(),
                styles: el.styles.clone(),
                text_content: el.text_content.clone(),
            });
        }
        nodes.sort_by_key(|n| n.id);
        TreeSnapshot {
            batches: self.batches.clone(),
            nodes,
        }
    }

    /// Look up a single node snapshot. Returns None if the node was removed.
    pub fn get_node(&self, id: NodeId) -> Option<NodeSnapshot> {
        let el = self.registry.get(id).ok()?;
        Some(NodeSnapshot {
            id: el.id,
            tag: el.node_type.canonical_tag().to_string(),
            node_type: el.node_type.wire_name(),
            parent_id: el.parent_id,
            children: el.children.clone(),
            properties: el.properties.clone(),
            styles: el.styles.clone(),
            text_content: el.text_content.clone(),
        })
    }

    /// All mutation kinds emitted so far, flattened across batches.
    pub fn mutation_kinds(&mut self) -> Vec<String> {
        self.flush_batch();
        self.batches.iter().flatten().cloned().collect()
    }

    pub fn size(&self) -> usize {
        self.registry.size()
    }

    /// Allocate the next unique node id (used by the JS runtime adapter).
    pub fn allocate_id(&mut self) -> NodeId {
        self.registry.allocate_id()
    }

    /// Apply a command to the tree.
    fn apply(&mut self, cmd: Command) -> Result<String, MutationError> {
        match cmd {
            Command::CreateNode { id, node_type } => {
                // Validate against known node types (§5.3 analogue). Tagged
                // customs (`Custom:na-list`, …) keep their suffix so native
                // shells can materialize the matching primitive view.
                let nt = NodeType::from_wire(&node_type)
                    .ok_or_else(|| MutationError::UnknownNodeType(node_type.clone()))?;
                let el = ViewNode::new(id, nt);
                self.registry.register_with_id(el)?;
                Ok("CreateNode".to_string())
            }
            Command::RemoveNode { id } => {
                self.registry.remove(id);
                Ok("RemoveNode".to_string())
            }
            Command::InsertChild {
                parent_id,
                child_id,
                index,
            } => {
                // Extract child's current parent before taking mutable borrows.
                let child_old_parent = self.registry.get(child_id)?.parent_id;

                // Detach from previous parent if re-parenting (T5 fix #1).
                if let Some(old_parent_id) = child_old_parent {
                    if old_parent_id != parent_id {
                        let old_parent = self.registry.get_mut(old_parent_id)?;
                        old_parent.remove_child_id(child_id);
                    }
                }

                // Now mutate parent and child.
                let child = self.registry.get_mut(child_id)?;
                child.parent_id = Some(parent_id);
                let parent = self.registry.get_mut(parent_id)?;
                parent.insert_child_at(index, child_id);

                Ok("InsertChild".to_string())
            }
            Command::RemoveChild {
                parent_id,
                child_id,
            } => {
                // T5 fix #2: verify child belongs to parent before removing.
                let child_parent = self.registry.get(child_id)?.parent_id;
                if child_parent != Some(parent_id) {
                    return Err(MutationError::NotACheckedChild {
                        child_id,
                        parent_id,
                    });
                }

                let child = self.registry.get_mut(child_id)?;
                child.parent_id = None;
                let parent = self.registry.get_mut(parent_id)?;
                parent.remove_child_id(child_id);

                Ok("RemoveChild".to_string())
            }
            Command::SetProperty { id, key, value } => {
                let el = self.registry.get_mut(id)?;
                // T5 fix #7: null value removes the property.
                if value.is_null() {
                    el.remove_property(&key);
                } else {
                    el.set_property(key, value);
                }
                Ok("SetProperty".to_string())
            }
            Command::SetStyle { id, key, value } => {
                let el = self.registry.get_mut(id)?;
                // T5 fix #4: unknown style keys are validated by the style
                // table before reaching the engine; here we store the value.
                let value = if matches!(
                    key.as_str(),
                    "width"
                        | "height"
                        | "minWidth"
                        | "minHeight"
                        | "maxWidth"
                        | "maxHeight"
                        | "top"
                        | "right"
                        | "bottom"
                        | "left"
                ) && value.parse::<f64>().is_ok()
                {
                    format!("{value}px")
                } else {
                    value
                };
                el.set_style(key.clone(), value);
                Ok("SetStyle".to_string())
            }
            Command::SetTextContent { id, text } => {
                let el = self.registry.get_mut(id)?;
                // T5 fix #3: text payloads are kept (not a no-op).
                el.set_text_content(text);
                Ok("SetTextContent".to_string())
            }
        }
    }
}

/// Snapshot of a node's observable state, for conformance comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSnapshot {
    pub id: NodeId,
    pub tag: String,
    pub node_type: String,
    pub parent_id: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub properties: std::collections::HashMap<String, serde_json::Value>,
    pub styles: std::collections::HashMap<String, String>,
    pub text_content: Option<String>,
}

/// Result of running commands through the engine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TreeSnapshot {
    pub batches: Vec<Vec<String>>,
    pub nodes: Vec<NodeSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create(id: NodeId, node_type: &str) -> Command {
        Command::CreateNode {
            id,
            node_type: node_type.to_string(),
        }
    }

    #[test]
    fn tagged_custom_create_nodes_are_preserved() {
        let mut engine = MutationEngine::new();
        for (id, wire) in [
            (1, "Custom:na-list"),
            (2, "Custom:na-switch"),
            (3, "Custom:na-activity-indicator"),
            (4, "Custom"),
        ] {
            engine.run_command(create(id, wire)).unwrap();
            assert_eq!(engine.get_node(id).unwrap().node_type, wire);
        }
        assert_eq!(engine.snapshot().nodes.len(), 4);
    }

    #[test]
    fn unknown_node_types_are_still_rejected() {
        let mut engine = MutationEngine::new();
        let err = engine.run_command(create(1, "Customish")).unwrap_err();
        assert!(matches!(err, MutationError::UnknownNodeType(_)));
    }
}
