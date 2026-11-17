/**
 * lib.rs — angnative-host-core: the Rust host's retained element tree and
 * command-protocol mutation engine.
 *
 * Mirrors the TypeScript `FakeBackend` (`packages/testing/src/fake-backend.ts`)
 * and `ElementRegistry` (`packages/core/src/native-element.ts`). The TS fake
 * backend and this Rust host must produce identical tree state from identical
 * command batches (Task R4 — the keystone).
 *
 * Modules:
 *   - node       — ViewNode element tree node (NodeId, NodeType, ViewNode)
 *   - element_registry — ElementRegistry: allocate/register/lookup/remove
 *   - command    — Command / CommandBatch protocol types (serde)
 *   - mutation   — MutationEngine: apply commands to the tree
 */
pub mod command;
pub mod element_registry;
pub mod mutation;
pub mod node;

pub use command::{validate_version, Command, CommandBatch, PROTOCOL_VERSION};
pub use element_registry::ElementRegistry;
pub use mutation::{MutationEngine, MutationError, NodeSnapshot, TreeSnapshot};
pub use node::{NodeId, NodeType, ViewNode};
