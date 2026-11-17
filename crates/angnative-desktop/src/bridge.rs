/**
 * bridge.rs — Connects the desktop shell to the Rust host crates.
 *
 * Wraps angnative-host-core (MutationEngine), angnative-host-layout
 * (LayoutEngine), and angnative-host-view (OpEmitter) into a single
 * interface that the desktop shell can call.
 */

use angnative_host_core::command::{Command, CommandBatch};
use angnative_host_core::mutation::MutationEngine;
use angnative_host_layout::{LayoutEngine, LayoutNodeInput, LayoutResult};
use angnative_host_view::op_emitter::OpEmitter;
use angnative_host_view::ops::PlatformOp;
use std::collections::HashMap;

/**
 * DesktopBridge — bridges the desktop window/renderer to the Rust host.
 *
 * Owns the MutationEngine, LayoutEngine, and OpEmitter. Processes command
 * batches from the desktop shell, runs layout, and emits platform ops
 * that the renderer draws.
 */
pub struct DesktopBridge {
    engine: MutationEngine,
    layout: LayoutEngine,
    emitter: OpEmitter,
    last_layout: Option<LayoutResult>,
}

impl DesktopBridge {
    pub fn new() -> Self {
        Self {
            engine: MutationEngine::new(),
            layout: LayoutEngine::default(),
            emitter: OpEmitter::new(),
            last_layout: None,
        }
    }

    /// Process a command batch (JSON or raw commands).
    pub fn process_batch(&mut self, batch: &CommandBatch) -> Result<Vec<PlatformOp>, String> {
        self.emitter.process_batch(batch).map_err(|e| e.to_string())
    }

    /// Process raw commands.
    pub fn process_commands(&mut self, commands: &[Command]) -> Result<Vec<PlatformOp>, String> {
        let mut ops = Vec::new();
        for cmd in commands {
            let op_kind = self.engine.run_command(cmd.clone()).map_err(|e| e.to_string())?;
            ops.push(PlatformOp::Create {
                node_id: match cmd {
                    Command::CreateNode { id, .. } => id,
                    _ => 0,
                },
                view_type: match cmd {
                    Command::CreateNode { node_type, .. } => node_type,
                    _ => String::new(),
                },
            });
        }
        Ok(ops)
    }

    /// Run layout over the current node tree.
    pub fn run_layout(
        &mut self,
        nodes: &HashMap<u64, LayoutNodeInput>,
        root_id: u64,
        available_width: f32,
        available_height: f32,
    ) -> Result<LayoutResult, String> {
        let result = self.layout.layout(root_id, nodes, available_width, available_height)?;
        self.last_layout = Some(result.clone());
        Ok(result)
    }

    /// Apply layout results as SetFrame ops.
    pub fn apply_layout(&mut self, layout: &LayoutResult) -> Result<Vec<PlatformOp>, String> {
        self.emitter.apply_layout(layout).map_err(|e| e.to_string())
    }

    /// Get all pending ops.
    pub fn take_ops(&mut self) -> Vec<PlatformOp> {
        self.emitter.ops().to_vec()
    }

    /// Clear pending ops.
    pub fn clear_ops(&mut self) {
        self.emitter.clear_ops();
    }

    /// Get the last layout result (for rendering).
    pub fn last_layout(&self) -> Option<&LayoutResult> {
        self.last_layout.as_ref()
    }

    /// Get the mutation engine (for tree inspection).
    pub fn engine(&mut self) -> &mut MutationEngine {
        &mut self.engine
    }
}

impl Default for DesktopBridge {
    fn default() -> Self {
        Self::new()
    }
}
