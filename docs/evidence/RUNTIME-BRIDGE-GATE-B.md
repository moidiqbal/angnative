# Runtime Bridge Gate B — deterministic lifecycle and validated handles

**Date:** 2026-11-23
**Gate result:** **Passed** for the in-repo bridge scope described below.

## What was verified

`crates/angnative-runtime-bridge/src/lib.rs` now provides a validated `u64`
bridge-handle registry with generation, and a `RuntimeController` with a
deterministic lifecycle. Errors, error codes and metrics are unified in
`crates/angnative-host-engine/src/errors.rs`
(`AngNativeError`, `ErrorCode`, `RuntimeErrorInfo`, `RuntimeMetrics`) and are
reachable through bridge accessors, including a native callback path.

## Tests (exact names)

Unit tests:

- `test_thread_affinity_and_validated_handles`
- `test_lifecycle_transitions_are_deterministic`
- `test_metrics_error_handler_and_native_callback`
- `test_reference_engine_proves_bridge_engine_abstraction`

Ignored stress/memory tests:

- `stress_10000_dispatches_keep_one_delivery_per_event`
- `stress_1000_create_destroy_cycles_do_not_leak_handles`
- `memory_growth_analysis_registry_returns_to_baseline`

The dispatch stress assertion is one delivery per event (canonical JS
thread-local path + legacy FFI fallback); no duplicate delivery claim is made.

## Scope caveats

- This evidence covers the Rust bridge and in-repo engine paths only.
- Physical OnePlus `b6640c8f` ABI re-verification is **not verified** — the
  device is online again, but no separate ABI re-verification was run in this
  round. Retained earlier device results are historical evidence only.
- No iOS/macOS runtime claim is made.
- Ignored tests require their explicit test invocation; no local run result is
  asserted by this document beyond the named tests' presence and scope.

**Related register rows:** P1-01, P1-02, P1-03, P1-04, P1-05, P1-09, P1-10,
P1-11, P2-05, P2-06, P3-02, P3-04.
