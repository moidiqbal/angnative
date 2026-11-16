use crate::node::{NodeId, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Operations emitted by Angular's Custom Renderer to mutate the native view tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ViewMutation {
    CreateNode {
        id: NodeId,
        node_type: NodeType,
    },
    RemoveNode {
        id: NodeId,
    },
    InsertChild {
        parent_id: NodeId,
        child_id: NodeId,
        index: usize,
    },
    RemoveChild {
        parent_id: NodeId,
        child_id: NodeId,
    },
    SetProperty {
        id: NodeId,
        key: String,
        value: serde_json::Value,
    },
    SetStyle {
        id: NodeId,
        key: String,
        value: String,
    },
    SetTextContent {
        id: NodeId,
        text: String,
    },
}

/// A batched collection of view mutations dispatched from the JS/TS context.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MutationBatch {
    pub frame_id: u64,
    pub timestamp_ms: u64,
    pub mutations: Vec<ViewMutation>,
}

impl MutationBatch {
    pub fn new(frame_id: u64, timestamp_ms: u64) -> Self {
        Self {
            frame_id,
            timestamp_ms,
            mutations: Vec::new(),
        }
    }

    pub fn push(&mut self, mutation: ViewMutation) {
        self.mutations.push(mutation);
    }
}
