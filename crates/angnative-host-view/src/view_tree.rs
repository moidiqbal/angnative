/**
 * view_tree.rs — Retained view tree mapping element nodes to native views.
 *
 * Tracks which element nodes have corresponding native views, their
 * parent-child relationships, and creation/destruction state. Used by
 * OpEmitter to determine which ops to emit for each mutation.
 */
use std::collections::{HashMap, HashSet};

/// State of a native view in the view tree.
#[derive(Debug, Clone, PartialEq)]
pub enum ViewState {
    /// View has been created but not yet attached to a parent.
    Created,
    /// View is attached to a parent in the native hierarchy.
    Attached,
    /// View has been destroyed (removed from hierarchy).
    Destroyed,
}

/// A tracked view entry.
#[derive(Debug, Clone)]
pub struct ViewEntry {
    pub node_id: u64,
    pub view_type: String,
    pub state: ViewState,
    pub parent_id: Option<u64>,
    pub children: Vec<u64>,
}

/// The retained view tree. Mirrors the element tree but tracks native view
/// lifecycle state separately.
#[derive(Debug, Clone, Default)]
pub struct ViewTree {
    /// node_id → view entry
    views: HashMap<u64, ViewEntry>,
    /// Set of node IDs that have native views created.
    created: HashSet<u64>,
    /// Set of node IDs whose views are attached in the hierarchy.
    attached: HashSet<u64>,
    /// Set of node IDs that are flattened — they have no native view.
    /// These are Custom/host-only nodes with no styles, props, listeners,
    /// or text content. Their children attach to their parent instead.
    flattened: HashSet<u64>,
    /// Per-list-node pool of recycled native view slot IDs.
    /// Keyed by the `na-list` (custom/recycle) node ID, values are view
    /// IDs that can be reused instead of creating new views.
    recycled_slots: HashMap<u64, Vec<u64>>,
    /// Maximum number of recycled slots to keep per list node.
    max_recycle_slots: u32,
}

impl ViewTree {
    pub fn new() -> Self {
        Self {
            views: HashMap::new(),
            created: HashSet::new(),
            attached: HashSet::new(),
            flattened: HashSet::new(),
            recycled_slots: HashMap::new(),
            max_recycle_slots: 10,
        }
    }

    /// Set the maximum number of recycled slots to retain per list node.
    pub fn set_max_recycle_slots(&mut self, max: u32) {
        self.max_recycle_slots = max;
    }

    /// Check if a native view exists for the given node.
    pub fn has_view(&self, node_id: u64) -> bool {
        self.created.contains(&node_id)
    }

    /// Check if a node is flattened (no native view).
    pub fn is_flattened(&self, node_id: u64) -> bool {
        self.flattened.contains(&node_id)
    }

    /// Mark a node as flattened — it gets no native view.
    /// If the node previously had a view registered, it is removed from
    /// the created set so no `Create`/`Destroy` ops are emitted for it.
    pub fn mark_flattened(&mut self, node_id: u64) {
        self.created.remove(&node_id);
        self.flattened.insert(node_id);
    }

    /// Check if a node is a recyclable list node.
    pub fn is_recyclable(&self, node_id: u64) -> bool {
        self.recycled_slots.contains_key(&node_id)
    }

    /// Acquire a recycled view slot for a list node, or None if the pool
    /// is empty. Returns the view ID to reuse.
    pub fn acquire_recycle_slot(&mut self, list_node_id: u64) -> Option<u64> {
        self.recycled_slots
            .get_mut(&list_node_id)
            .and_then(|pool| pool.pop())
    }

    /// Release a view slot back into the recycle pool for a list node.
    /// Keeps at most `max_recycle_slots` slots per list.
    pub fn release_recycle_slot(&mut self, list_node_id: u64, slot_id: u64) {
        let pool = self
            .recycled_slots
            .entry(list_node_id)
            .or_insert_with(Vec::new);
        if pool.len() < self.max_recycle_slots as usize {
            pool.push(slot_id);
        }
    }

    /// Get a view entry by node ID.
    pub fn get(&self, node_id: u64) -> Option<&ViewEntry> {
        self.views.get(&node_id)
    }

    /// Register a newly created view.
    /// If the node is flattened, no native view is created.
    pub fn register_view(&mut self, node_id: u64, view_type: String) {
        if self.flattened.contains(&node_id) {
            return;
        }
        self.created.insert(node_id);
        self.views.insert(
            node_id,
            ViewEntry {
                node_id,
                view_type,
                state: ViewState::Created,
                parent_id: None,
                children: Vec::new(),
            },
        );
    }

    /// Mark a view as attached to a parent.
    /// If the parent is flattened, attach to the grandparent instead
    /// (flattened nodes are transparent in the native hierarchy).
    pub fn attach(&mut self, node_id: u64, parent_id: u64) {
        let real_parent = if self.flattened.contains(&parent_id) {
            self.views.get(&parent_id).and_then(|e| e.parent_id).unwrap_or(parent_id)
        } else {
            parent_id
        };

        if let Some(entry) = self.views.get_mut(&node_id) {
            entry.state = ViewState::Attached;
            entry.parent_id = Some(real_parent);
        }
        self.attached.insert(node_id);

        // Add to real parent's children list
        if let Some(parent) = self.views.get_mut(&real_parent) {
            if !parent.children.contains(&node_id) {
                parent.children.push(node_id);
            }
        }
    }

    /// Mark a view as detached from its parent.
    pub fn detach(&mut self, node_id: u64) {
        if self.flattened.contains(&node_id) {
            self.flattened.remove(&node_id);
            return;
        }
        if let Some(entry) = self.views.get(&node_id) {
            if let Some(parent_id) = entry.parent_id {
                if let Some(parent) = self.views.get_mut(&parent_id) {
                    parent.children.retain(|&id| id != node_id);
                }
            }
        }
        if let Some(entry) = self.views.get_mut(&node_id) {
            entry.state = ViewState::Created;
            entry.parent_id = None;
        }
        self.attached.remove(&node_id);
    }

    /// Mark a view as destroyed.
    pub fn destroy(&mut self, node_id: u64) {
        if self.flattened.contains(&node_id) {
            self.flattened.remove(&node_id);
            return;
        }
        self.detach(node_id);
        if let Some(entry) = self.views.get_mut(&node_id) {
            entry.state = ViewState::Destroyed;
        }
        self.created.remove(&node_id);
        self.attached.remove(&node_id);
    }

    /// Get all attached view IDs.
    pub fn attached_views(&self) -> Vec<u64> {
        let mut ids: Vec<u64> = self.attached.iter().copied().collect();
        ids.sort();
        ids
    }

    /// Get all view entries.
    pub fn all_views(&self) -> Vec<&ViewEntry> {
        let mut entries: Vec<&ViewEntry> = self.views.values().collect();
        entries.sort_by_key(|e| e.node_id);
        entries
    }

    /// Get children of a view.
    pub fn children_of(&self, node_id: u64) -> Vec<u64> {
        self.views
            .get(&node_id)
            .map(|e| e.children.clone())
            .unwrap_or_default()
    }

    /// Get parent of a view.
    pub fn parent_of(&self, node_id: u64) -> Option<u64> {
        self.views.get(&node_id).and_then(|e| e.parent_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_attach() {
        let mut tree = ViewTree::new();
        tree.register_view(1, "View".to_string());
        tree.register_view(2, "View".to_string());
        assert!(tree.has_view(1));
        assert!(!tree.attached_views().contains(&1));

        tree.attach(2, 1);
        assert!(tree.attached_views().contains(&2));
        assert_eq!(tree.parent_of(2), Some(1));
        assert_eq!(tree.children_of(1), vec![2]);
    }

    #[test]
    fn detach_and_destroy() {
        let mut tree = ViewTree::new();
        tree.register_view(1, "View".to_string());
        tree.register_view(2, "View".to_string());
        tree.attach(2, 1);

        tree.detach(2);
        assert!(!tree.attached_views().contains(&2));
        assert_eq!(tree.children_of(1), Vec::<u64>::new());

        tree.destroy(1);
        assert!(!tree.has_view(1));
    }
}
