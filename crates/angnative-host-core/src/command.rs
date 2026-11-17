/**
 * command.rs — TS↔Rust command protocol types for angnative-host-core.
 *
 * Mirrors the TypeScript command protocol
 * (`packages/contract/src/protocol.ts`) 1:1 — same enum names, same fields,
 * same JSON shape — so a command batch round-trips through Rust serde without
 * an intermediate schema.
 *
 * Version: PROTOCOL_VERSION = 1
 */
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u64 = 1;

/// Operations emitted by Angular's custom renderer to mutate the native view tree.
/// Mirrors `packages/contract/src/protocol.ts::Command` and
/// `crates/angnative-core/src/mutation.rs::ViewMutation`.
///
/// JSON shape is flat: `{ "kind": "CreateNode", "id": 1, "nodeType": "View" }`
/// — identical to the TypeScript command protocol fixtures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Command {
    #[serde(rename = "CreateNode")]
    CreateNode { id: NodeId, node_type: String },
    #[serde(rename = "RemoveNode")]
    RemoveNode { id: NodeId },
    #[serde(rename = "InsertChild")]
    InsertChild {
        parent_id: NodeId,
        child_id: NodeId,
        index: usize,
    },
    #[serde(rename = "RemoveChild")]
    RemoveChild { parent_id: NodeId, child_id: NodeId },
    #[serde(rename = "SetProperty")]
    SetProperty {
        id: NodeId,
        key: String,
        value: serde_json::Value,
    },
    #[serde(rename = "SetStyle")]
    SetStyle {
        id: NodeId,
        key: String,
        value: String,
    },
    #[serde(rename = "SetTextContent")]
    SetTextContent { id: NodeId, text: String },
}

/// A batched collection of commands dispatched from the JS/TS context.
/// Mirrors `packages/contract/src/protocol.ts::CommandBatch`.
///
/// JSON shape uses camelCase: `{ "protocolVersion": 1, "frameId": 0, ... }`
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandBatch {
    pub protocol_version: u64,
    pub frame_id: u64,
    pub timestamp_ms: u64,
    pub commands: Vec<Command>,
}

impl CommandBatch {
    pub fn new(frame_id: u64, timestamp_ms: u64) -> Self {
        Self {
            protocol_version: PROTOCOL_VERSION,
            frame_id,
            timestamp_ms,
            commands: Vec::new(),
        }
    }

    pub fn push(&mut self, command: Command) {
        self.commands.push(command);
    }
}

/// Re-export NodeId for convenience.
pub use crate::node::NodeId;

/// Validate a batch's protocol version. Returns an error if unsupported.
pub fn validate_version(version: u64) -> Result<(), String> {
    if version != PROTOCOL_VERSION {
        Err(format!(
            "Unsupported protocol version {version}, expected {PROTOCOL_VERSION}"
        ))
    } else {
        Ok(())
    }
}
