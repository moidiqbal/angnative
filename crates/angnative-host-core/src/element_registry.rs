/**
 * element_registry.rs — Node registry for angnative-host-core.
 *
 * Mirrors the TypeScript `ElementRegistry` (`packages/core/src/native-element.ts`):
 * allocates unique node IDs, registers/lookup/remove nodes, and detaches removed
 * nodes from their parents.
 */
use crate::mutation::MutationError;
use crate::node::{NodeId, ViewNode};
use std::collections::HashMap;

/// Registry of all live native elements, keyed by node id.
#[derive(Debug, Clone, Default)]
pub struct ElementRegistry {
    elements: HashMap<NodeId, ViewNode>,
    next_id: NodeId,
}

impl ElementRegistry {
    pub fn new() -> Self {
        Self {
            elements: HashMap::new(),
            next_id: 1,
        }
    }

    /// Allocate the next unique node id.
    pub fn allocate_id(&mut self) -> NodeId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    /// Register a newly created element with an allocated id.
    pub fn register(&mut self, element: ViewNode) -> Result<NodeId, MutationError> {
        let id = element.id;
        if self.elements.contains_key(&id) {
            return Err(MutationError::DuplicateId(id));
        }
        self.elements.insert(id, element);
        Ok(id)
    }

    /// Register an element with an explicit id (used when decoding command
    /// batches that carry their own ids, mirroring the TS fake backend).
    /// Duplicate ids are a typed error, never a panic — a bad batch from JS
    /// must not abort the host process.
    pub fn register_with_id(&mut self, element: ViewNode) -> Result<(), MutationError> {
        let id = element.id;
        if self.elements.contains_key(&id) {
            return Err(MutationError::DuplicateId(id));
        }
        if id >= self.next_id {
            self.next_id = id + 1;
        }
        self.elements.insert(id, element);
        Ok(())
    }

    /// Look up an element by id. Missing ids are a typed error (spec §5.3).
    pub fn get(&self, id: NodeId) -> Result<&ViewNode, MutationError> {
        self.elements
            .get(&id)
            .ok_or(MutationError::NodeNotFound(id))
    }

    pub fn get_mut(&mut self, id: NodeId) -> Result<&mut ViewNode, MutationError> {
        self.elements
            .get_mut(&id)
            .ok_or(MutationError::NodeNotFound(id))
    }

    pub fn has(&self, id: NodeId) -> bool {
        self.elements.contains_key(&id)
    }

    /// Remove an element and detach it from its parent's child list.
    pub fn remove(&mut self, id: NodeId) {
        if let Some(element) = self.elements.get(&id) {
            if let Some(parent_id) = element.parent_id {
                if let Some(parent) = self.elements.get_mut(&parent_id) {
                    parent.remove_child_id(id);
                }
            }
        }
        self.elements.remove(&id);
    }

    pub fn size(&self) -> usize {
        self.elements.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &ViewNode> {
        self.elements.values()
    }
}
