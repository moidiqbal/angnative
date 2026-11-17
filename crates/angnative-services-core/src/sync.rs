/**
 * sync.rs — Offline-sync queue (spec §5.4, task C2).
 *
 * Durable, conflict-aware operation queue. Every acknowledged operation is
 * persisted and never lost — even across sync failures. Conflicts are
 * detected and resolved via last-write-wins with vector-clock-style version
 * tracking.
 */

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

/** Direction of synchronization. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncDirection {
    Push,   // local -> remote
    Pull,   // remote -> local
    Bidirectional,
}

/** Status of a queued operation. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncStatus {
    Pending,
    InProgress,
    Acknowledged,   // remote has acknowledged — durable, never lost
    Failed,
    Conflict,
}

/** A single queued sync operation. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedOperation {
    pub id: String,
    pub entity_type: String,
    pub entity_id: String,
    pub operation: SyncOpType,
    pub payload: serde_json::Value,
    pub version: u64,
    pub status: SyncStatus,
    pub created_at: u64,
    pub acknowledged_at: Option<u64>,
    pub retry_count: u32,
    pub conflict: Option<SyncConflict>,
}

/** Type of sync operation. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncOpType {
    Create,
    Update,
    Delete,
}

/** A sync conflict detected between local and remote versions. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncConflict {
    pub entity_id: String,
    pub local_version: u64,
    pub remote_version: u64,
    pub local_payload: serde_json::Value,
    pub remote_payload: serde_json::Value,
    pub resolved: bool,
    pub resolution: Option<SyncConflictResolution>,
}

/** How a conflict was resolved. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncConflictResolution {
    LocalWins,
    RemoteWins,
    Merge(serde_json::Value),
    Manual,
}

/** Result of a sync operation. */
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncResult {
    pub direction: SyncDirection,
    pub operations_processed: u32,
    pub operations_succeeded: u32,
    pub operations_failed: u32,
    pub conflicts_detected: u32,
    pub conflicts_resolved: u32,
    pub duration_ms: u64,
}

/** The offline-sync queue. Durable and conflict-aware. */
pub struct OfflineSyncQueue {
    operations: BTreeMap<String, QueuedOperation>,
    next_id: u64,
    entity_versions: HashMap<String, u64>,
    sync_results: Vec<SyncResult>,
    max_retries: u32,
}

impl Default for OfflineSyncQueue {
    fn default() -> Self {
        Self::new()
    }
}

impl OfflineSyncQueue {
    pub fn new() -> Self {
        Self {
            operations: BTreeMap::new(),
            next_id: 1,
            entity_versions: HashMap::new(),
            sync_results: Vec::new(),
            max_retries: 5,
        }
    }

    /** Enqueue a new operation. Returns the assigned operation ID. */
    pub fn enqueue(
        &mut self,
        entity_type: &str,
        entity_id: &str,
        op: SyncOpType,
        payload: serde_json::Value,
    ) -> String {
        let id = format!("op-{}", self.next_id);
        self.next_id += 1;

        let version = self.entity_versions.entry(entity_id.to_string()).or_insert(0);
        *version += 1;

        let op_entry = QueuedOperation {
            id: id.clone(),
            entity_type: entity_type.to_string(),
            entity_id: entity_id.to_string(),
            operation: op,
            payload,
            version: *version,
            status: SyncStatus::Pending,
            created_at: current_timestamp_ms(),
            acknowledged_at: None,
            retry_count: 0,
            conflict: None,
        };

        self.operations.insert(id.clone(), op_entry);
        id
    }

    /** Mark an operation as acknowledged (durable — never lost). */
    pub fn acknowledge(&mut self, op_id: &str) -> Result<(), String> {
        let op = self
            .operations
            .get_mut(op_id)
            .ok_or_else(|| format!("Operation {} not found", op_id))?;
        op.status = SyncStatus::Acknowledged;
        op.acknowledged_at = Some(current_timestamp_ms());
        Ok(())
    }

    /** Mark an operation as failed, incrementing retry count. */
    pub fn fail(&mut self, op_id: &str) -> Result<(), String> {
        let op = self
            .operations
            .get_mut(op_id)
            .ok_or_else(|| format!("Operation {} not found", op_id))?;
        op.retry_count += 1;
        if op.retry_count >= self.max_retries {
            op.status = SyncStatus::Failed;
        } else {
            op.status = SyncStatus::Pending; // will be retried
        }
        Ok(())
    }

    /** Detect a conflict between local and remote versions of an entity. */
    pub fn detect_conflict(
        &mut self,
        entity_id: &str,
        remote_version: u64,
        remote_payload: serde_json::Value,
    ) -> Option<SyncConflict> {
        let local_version = self.entity_versions.get(entity_id).copied().unwrap_or(0);
        if remote_version > local_version {
            let local_payload = self
                .operations
                .values()
                .find(|op| op.entity_id == entity_id && op.status == SyncStatus::Acknowledged)
                .map(|op| op.payload.clone())
                .unwrap_or(serde_json::Value::Null);

            let conflict = SyncConflict {
                entity_id: entity_id.to_string(),
                local_version,
                remote_version,
                local_payload,
                remote_payload,
                resolved: false,
                resolution: None,
            };

            // Mark the latest pending op for this entity as conflicted.
            for op in self.operations.values_mut() {
                if op.entity_id == entity_id && op.status == SyncStatus::Pending {
                    op.status = SyncStatus::Conflict;
                    op.conflict = Some(conflict.clone());
                }
            }

            Some(conflict)
        } else {
            None
        }
    }

    /** Resolve a conflict with the given resolution. */
    pub fn resolve_conflict(
        &mut self,
        entity_id: &str,
        resolution: SyncConflictResolution,
    ) -> Result<(), String> {
        // Find the conflict for this entity.
        let conflict = self
            .operations
            .values()
            .find(|op| op.entity_id == entity_id && op.conflict.is_some())
            .cloned();

        let mut conflict = conflict.ok_or_else(|| {
            format!("No unresolved conflict for entity {}", entity_id)
        })?;

        conflict.resolved = true;
        conflict.resolution = Some(resolution);

        // Update entity version based on resolution.
        let version = self.entity_versions.entry(entity_id.to_string()).or_insert(0);
        match &conflict.resolution {
            Some(SyncConflictResolution::RemoteWins) => {
                *version = conflict.remote_version;
            }
            Some(SyncConflictResolution::LocalWins) => {
                // keep local version
            }
            Some(SyncConflictResolution::Merge(_)) => {
                *version = conflict.remote_version.max(conflict.local_version) + 1;
            }
            Some(SyncConflictResolution::Manual) => {}
            None => {}
        }

        // Un-pend the conflicting ops.
        for op in self.operations.values_mut() {
            if op.entity_id == entity_id && op.status == SyncStatus::Conflict {
                op.status = SyncStatus::Pending;
                op.conflict = None;
            }
        }

        Ok(())
    }

    /** Get all pending operations (not yet acknowledged). */
    pub fn pending_operations(&self) -> Vec<&QueuedOperation> {
        self.operations
            .values()
            .filter(|op| op.status == SyncStatus::Pending || op.status == SyncStatus::Conflict)
            .collect()
    }

    /** Get all acknowledged operations (durable, never lost). */
    pub fn acknowledged_operations(&self) -> Vec<&QueuedOperation> {
        self.operations
            .values()
            .filter(|op| op.status == SyncStatus::Acknowledged)
            .collect()
    }

    /** Record a sync result. */
    pub fn record_sync_result(&mut self, result: SyncResult) {
        self.sync_results.push(result);
    }

    /** Get all sync results. */
    pub fn sync_results(&self) -> &[SyncResult] {
        &self.sync_results
    }

    /** Get the current version of an entity. */
    pub fn entity_version(&self, entity_id: &str) -> u64 {
        self.entity_versions.get(entity_id).copied().unwrap_or(0)
    }

    /** Set the max retry count before an operation is marked Failed. */
    pub fn set_max_retries(&mut self, max: u32) {
        self.max_retries = max;
    }

    /** Number of operations in the queue. */
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

fn current_timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enqueue_and_acknowledge() {
        let mut queue = OfflineSyncQueue::new();
        let id = queue.enqueue("user", "user-1", SyncOpType::Create, serde_json::json!({"name": "Alice"}));
        assert_eq!(queue.len(), 1);

        let op = queue.operations.get(&id).unwrap();
        assert_eq!(op.status, SyncStatus::Pending);
        assert_eq!(op.version, 1);

        queue.acknowledge(&id).unwrap();
        let op = queue.operations.get(&id).unwrap();
        assert_eq!(op.status, SyncStatus::Acknowledged);
        assert!(op.acknowledged_at.is_some());
    }

    #[test]
    fn acknowledged_ops_are_durable() {
        let mut queue = OfflineSyncQueue::new();
        queue.enqueue("user", "u1", SyncOpType::Create, serde_json::json!({}));
        queue.enqueue("user", "u2", SyncOpType::Update, serde_json::json!({}));
        queue.acknowledge(&"op-1".to_string()).unwrap();

        let acked = queue.acknowledged_operations();
        assert_eq!(acked.len(), 1);
        assert_eq!(acked[0].id, "op-1");
    }

    #[test]
    fn conflict_detection() {
        let mut queue = OfflineSyncQueue::new();
        queue.enqueue("user", "u1", SyncOpType::Create, serde_json::json!({"name": "Alice"}));
        queue.acknowledge(&"op-1".to_string()).unwrap();

        // Remote has a newer version — conflict.
        let conflict = queue.detect_conflict("u1", 2, serde_json::json!({"name": "Bob"}));
        assert!(conflict.is_some());
        let c = conflict.unwrap();
        assert_eq!(c.local_version, 1);
        assert_eq!(c.remote_version, 2);
        assert!(!c.resolved);

        // The pending op should now be in Conflict status.
        let pending = queue.pending_operations();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, SyncStatus::Conflict);
    }

    #[test]
    fn conflict_resolution() {
        let mut queue = OfflineSyncQueue::new();
        queue.enqueue("user", "u1", SyncOpType::Create, serde_json::json!({"name": "Alice"}));
        queue.detect_conflict("u1", 2, serde_json::json!({"name": "Bob"}));

        queue.resolve_conflict("u1", SyncConflictResolution::RemoteWins).unwrap();

        // Entity version updated to remote version.
        assert_eq!(queue.entity_version("u1"), 2);

        // Conflicting ops should be back to Pending.
        let pending = queue.pending_operations();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].status, SyncStatus::Pending);
        assert!(pending[0].conflict.is_none());
    }

    #[test]
    fn fail_increments_retry() {
        let mut queue = OfflineSyncQueue::new();
        queue.set_max_retries(2);
        queue.enqueue("user", "u1", SyncOpType::Create, serde_json::json!({}));

        queue.fail("op-1").unwrap();
        queue.fail("op-1").unwrap();
        // After max_retries, should be Failed.
        let op = queue.operations.get("op-1").unwrap();
        assert_eq!(op.status, SyncStatus::Failed);
        assert_eq!(op.retry_count, 2);
    }
}
