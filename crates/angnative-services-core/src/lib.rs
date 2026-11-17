/**
 * lib.rs — angnative-services-core: capability resolver, offline-sync,
 * crypto, serialization, and diagnostics — usable without Angular.
 *
 * Identity D-4 / spec §5.9: these services are backend-agnostic and do not
 * depend on Angular. They support the capability system (C1–C5) and the
 * diagnostics channel (§9).
 *
 * Modules:
 *   - capability  — CapabilityRegistry interface + resolver + fallback chain
 *   - sync        — Offline-sync queue (durable, conflict-aware, no lost acknowledged op)
 *   - crypto      — Crypto + secure storage (behind `crypto` feature)
 *   - serialization — Serialization helpers
 *   - diagnostics — Performance diagnostics channel
 */

#[cfg(feature = "crypto")]
pub mod crypto;
pub mod capability;
pub mod diagnostics;
pub mod serialization;
pub mod sync;

pub use capability::{
    Capability, CapabilityDeclaration, CapabilityRegistry, CapabilityResolver,
    CapabilityStatus, DegradationDiagnostic,
};
pub use diagnostics::{
    DiagnosticsChannel, DiagnosticsEvent, DiagnosticsLevel, PerformanceDiagnostics,
};
pub use serialization::{serialize_json, serialize_messagepack, DeserializeError, SerializeError};
pub use sync::{
    OfflineSyncQueue, QueuedOperation, SyncConflict, SyncDirection, SyncResult, SyncStatus,
};
