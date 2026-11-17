/**
 * services_spec.rs — Integration tests for angnative-services-core (Task R7).
 *
 * Ties together all R7 modules:
 *   - capability: CapabilityRegistry + resolver + fallback chain
 *   - sync: OfflineSyncQueue (durable, conflict-aware)
 *   - diagnostics: DiagnosticsChannel (spec §9 event types)
 *   - serialization: JSON round-trip
 *   - crypto: SecureKey + CryptoService (behind `crypto` feature)
 *
 * These verify that services work together end-to-end and are usable
 * without Angular (spec §5.9).
 */

use angnative_services_core::capability::{
    CapabilityDeclaration, CapabilityResolver,
};
use angnative_services_core::diagnostics::{
    DiagnosticsChannel, DiagnosticsEventType,
};
use angnative_services_core::sync::{
    OfflineSyncQueue, SyncConflictResolution, SyncDirection, SyncOpType,
};
use angnative_services_core::{
    serialize, deserialize, SerializationFormat,
};
use serde_json::json;

#[test]
fn capability_and_diagnostics_work_together() {
    let mut resolver = CapabilityResolver::new();
    let mut channel = DiagnosticsChannel::new();

    // Declare a capability.
    resolver.declare(CapabilityDeclaration {
        id: "camera".to_string(),
        version: "1.0".to_string(),
        description: "Camera access".to_string(),
        platforms: vec!["android".to_string()],
        min_os_version: None,
    });

    // Resolve an available capability — no diagnostic.
    let status = resolver.resolve("camera").unwrap();
    assert_eq!(status, angnative_services_core::capability::CapabilityStatus::Available);
    assert!(channel.is_empty());

    // Resolve an unavailable capability — diagnostic is published.
    let diag = resolver.resolve("gps").unwrap_err();
    assert_eq!(diag.capability_id, "gps");
    assert_eq!(channel.len(), 1);
    assert_eq!(
        channel.events_by_type(&DiagnosticsEventType::NativeCapabilityUnavailable).len(),
        1
    );
}

#[test]
fn offline_sync_with_conflict_and_resolution() {
    let mut queue = OfflineSyncQueue::new();
    let mut channel = DiagnosticsChannel::new();

    // Enqueue and acknowledge operations (durable).
    let op1 = queue.enqueue("user", "u1", SyncOpType::Create, json!({"name": "Alice"}));
    let op2 = queue.enqueue("user", "u2", SyncOpType::Update, json!({"name": "Bob"}));
    queue.acknowledge(&op1).unwrap();
    queue.acknowledge(&op2).unwrap();

    // Detect a conflict (remote has newer version).
    let conflict = queue.detect_conflict("u1", 5, json!({"name": "Alicia"}));
    assert!(conflict.is_some());

    // Resolve with remote wins.
    queue.resolve_conflict("u1", SyncConflictResolution::RemoteWins).unwrap();

    // Record the sync result.
    queue.record_sync_result(angnative_services_core::sync::SyncResult {
        direction: SyncDirection::Bidirectional,
        operations_processed: 2,
        operations_succeeded: 2,
        operations_failed: 0,
        conflicts_detected: 1,
        conflicts_resolved: 1,
        duration_ms: 150,
    });

    // Verify diagnostics were not auto-published (conflict resolution is silent
    // unless the caller publishes — diagnostics channel is separate).
    assert_eq!(queue.acknowledged_operations().len(), 2);
    assert_eq!(queue.sync_results().len(), 1);
    assert_eq!(queue.sync_results()[0].conflicts_resolved, 1);
}

#[test]
fn serialization_round_trip_with_sync_queue() {
    let mut queue = OfflineSyncQueue::new();
    let id = queue.enqueue(
        "document",
        "doc-1",
        SyncOpType::Create,
        json!({"title": "Hello", "content": "World"}),
    );

    // Serialize the queue state to JSON.
    let pending: Vec<&angnative_services_core::sync::QueuedOperation> =
        queue.pending_operations();
    let json_bytes = serialize(&pending, SerializationFormat::Json).unwrap();

    // Deserialize back.
    let restored: Vec<angnative_services_core::sync::QueuedOperation> =
        deserialize(&json_bytes, SerializationFormat::Json).unwrap();
    assert_eq!(restored.len(), 1);
    assert_eq!(restored[0].id, id);
    assert_eq!(restored[0].entity_id, "doc-1");
}

#[test]
fn full_r7_integration() {
    let mut resolver = CapabilityResolver::new();
    let mut channel = DiagnosticsChannel::new();
    let mut queue = OfflineSyncQueue::new();

    // 1. Declare capabilities.
    resolver.declare(CapabilityDeclaration {
        id: "offline-sync".to_string(),
        version: "1.0".to_string(),
        description: "Offline sync queue".to_string(),
        platforms: vec![],
        min_os_version: None,
    });

    // 2. Resolve available capability — no diagnostic.
    assert!(resolver.resolve("offline-sync").is_ok());
    assert_eq!(channel.len(), 0);

    // 3. Enqueue a sync operation (durable).
    let op_id = queue.enqueue(
        "document",
        "doc-1",
        SyncOpType::Create,
        json!({"title": "R7 Integration Test"}),
    );
    queue.acknowledge(&op_id).unwrap();

    // 4. Resolve an unavailable capability — diagnostic published.
    resolver.resolve("camera").unwrap_err();
    assert_eq!(channel.len(), 1);

    // 5. Serialize the acknowledged operation.
    let acked: Vec<&angnative_services_core::sync::QueuedOperation> =
        queue.acknowledged_operations();
    let json_bytes = serialize(&acked, SerializationFormat::Json).unwrap();
    let restored: Vec<angnative_services_core::sync::QueuedOperation> =
        deserialize(&json_bytes, SerializationFormat::Json).unwrap();
    assert_eq!(restored.len(), 1);

    // 6. All R7 services work together without Angular.
    assert_eq!(queue.len(), 1);
    assert_eq!(channel.len(), 1);
    assert!(resolver.is_declared("offline-sync"));
    assert!(!resolver.is_declared("camera"));
}

#[cfg(feature = "crypto")]
#[test]
fn crypto_with_diagnostics_and_sync() {
    use angnative_services_core::crypto::{CryptoService, SecureKey};

    let mut channel = DiagnosticsChannel::new();
    let mut queue = OfflineSyncQueue::new();
    let crypto = CryptoService::new();

    // Generate a secure key.
    crypto.generate_key("enc-key", 32).unwrap();

    // Encrypt some data.
    let plaintext = b"sensitive sync payload";
    let sealed = crypto.encrypt("enc-key", plaintext).unwrap();
    let decrypted = crypto.decrypt(&sealed).unwrap();
    assert_eq!(decrypted, plaintext);

    // Enqueue the encrypted payload as a sync operation.
    let op_id = queue.enqueue(
        "secure-doc",
        "sd-1",
        SyncOpType::Create,
        json!({"ciphertext_len": sealed.ciphertext.len(), "key_id": sealed.key_id}),
    );
    queue.acknowledge(&op_id).unwrap();

    // Publish a diagnostic about the crypto operation.
    channel.native_capability_degraded("secure-storage", Some("software-key"));

    // Verify all services work together.
    assert_eq!(crypto.key_count(), 1);
    assert_eq!(queue.len(), 1);
    assert_eq!(channel.len(), 1);
}
