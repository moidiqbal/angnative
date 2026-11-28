# StubEngine conformance — retained proof (2026-11-27)

`StubEngine` is a **non-executing** `JsEngine` contributor template behind the
default-off `engine-stub` Cargo feature. It is not Hermes, not V8, not a second
production engine, and not a dynamic plugin ABI. Hermes/V8 are **not integrated**
(ADR 0001 and `docs/evidence/RUNTIME-BRIDGE-GATE-F.md` are unchanged). `ReferenceEngine` remains the established second in-repo engine
used to prove `RuntimeBridge<E>` abstraction.

## Source

- `crates/angnative-host-engine/src/stub_engine.rs`
- `crates/angnative-host-engine/tests/engine_conformance.rs`
- Feature: `engine-stub = []` (default `[]`), no new dependencies or link flags.

Guard behaviour: `eval` returns `EngineError::Internal` containing "stub" and
"unimplemented"; `drain_microtasks` explicitly returns `Ok(0)` (no JS realm);
`engine_name()` is exactly `"StubEngine (non-executing test double)"`. Two tests
assert the name contains none of `Hermes`, `hermes`, `V8`, `verified` or
`integrated`.

## Exact commands and results

Default (stub absent from the shipping path):

```
cargo test -p angnative-host-engine --tests
  lib: 26 passed / 0 failed
  engine_conformance: 4 passed / 0 failed
    tier_a_api_shape_quickjs
    tier_a_api_shape_reference_engine
    tier_a_engine_names_are_unique
    tier_b_quickjs_evaluates_and_rejects_invalid_syntax
```

Feature enabled (contributor template compiled):

```
cargo test -p angnative-host-engine --features engine-stub --tests
  lib: 28 passed / 0 failed
  engine_conformance: 5 passed / 0 failed
    (adds tier_a_api_shape_stub_engine)
```

Tier A is API shape only (globals, registered functions, unknown-function
error, idempotent `gc`, bounded microtask drain, unique non-empty engine name).
Tier B (JS parsing/syntax errors) applies to QuickJS only.

## Claim boundary

Allowed: "compile-time generic `JsEngine`; QuickJS-ng is the sole production
engine; `ReferenceEngine` proves abstraction; `StubEngine` is a test-only
non-executing template behind `engine-stub`."

Not allowed: Hermes/V8 supported, verified or integrated; engine parity;
runtime-switchable engines; AOT or HMR verified on the stub; any iOS runtime
claim.
