# Runtime Bridge Gate F — engine abstraction proof

**Date:** 2026-11-23
**Gate result:** **Passed** for the `JsEngine` abstraction; **no Hermes/V8 integration**.

## What was verified

`RuntimeBridge` is generic over a `JsEngine` interface. An in-repo
`ReferenceEngine` (`crates/angnative-host-engine/src/reference_engine.rs`)
implements that interface and proves the bridge does not depend on concrete
QuickJS types.

## Test (exact name)

- `test_reference_engine_proves_bridge_engine_abstraction`

## Scope caveats

- The `ReferenceEngine` is an in-repo reference implementation. It is
  **explicitly not Hermes and not V8**; no Hermes/V8 adapter, build or runtime
  is integrated or verified.
- This gate proves the abstraction boundary only. It does not add a second
  production engine, engine parity, bytecode loading or a module loader.
- No iOS/macOS runtime claim is made.

**Related register rows:** P2-12, P2-13.
