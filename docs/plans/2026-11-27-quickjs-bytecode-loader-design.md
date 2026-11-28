# QuickJS bytecode loader — validated design (priority #5)

**Date:** 2026-11-27
**Branch:** `feat/quickjs-bytecode-loader` from local #4 commit `93932bf`
**Supersedes:** `docs/evidence/BYTECODE-FEASIBILITY-SPIKE.md` (deferred spike only)
**Target register row:** P2-02 in `docs/architecture/issue-register.md`

## Decision summary

Add an opt-in QuickJS module-bytecode path beside the supported text-IIFE path.
No startup-speed, ES-module, Hermes/V8, platform-shell or authenticity claim is
made. The change stays in the Rust/runtime artifact layer; no C ABI, JNI, Swift,
header, Android asset or iOS change is part of this priority.

## 1. Artifact format and generator

The v2 artifact layout is:

```text
ANGNQJSB2 (9 bytes) | u32 LE payload length | u32 LE fingerprint length |
UTF-8 fingerprint | raw rquickjs module bytecode
```

The fixed header is 17 bytes. The fingerprint is `runtime_fingerprint()` from
`bytecode.rs`: `qjsbc-v2/rquickjs-<crate version>/quickjs-ng-<JS_GetVersion()>/<arch>-<os>-<endian>-<pointer bits>`.
The rquickjs crate version is a documented compile-time constant (`0.14.0`,
pinned by workspace `Cargo.toml`/`Cargo.lock`) because rquickjs exposes no
stable version API; the QuickJS-ng version comes from the linked
`qjs::JS_GetVersion()`. The payload cap remains `64 * 1024 * 1024` bytes;
fingerprints are capped at 256 bytes. The loader retains every v1 rejection
case (wrong magic, truncated header, length mismatch, empty payload, oversized
declared length) and additionally rejects legacy `ANGNQJSB1` containers,
missing/unknown/mismatched fingerprints, and non-UTF-8 fingerprints, all as
`EngineError::Bytecode`, before calling the one unsafe `Module::load` API.

A Rust binary
`crates/angnative-host-engine/src/bin/dump-bytecode.rs` reads a self-contained
AOT IIFE source, declares it as a module with a stable virtual name, writes
little-endian module bytecode with source/debug information stripped, and emits
the fingerprinted artifact:

```powershell
cargo run -p angnative-host-engine --bin dump-bytecode -- <input.js> <output.qjsbc>
```

A root `pnpm bundle:bytecode` script wraps the command for
`dist/aot/animations-app-bundle.qjsbc`. CI generates the artifact after the
AOT/bundle steps and before the ignored runtime acceptance tests.

## 2. Engine-neutral API and data flow

`JsEngine` gains:

```rust
fn load_bytecode(&mut self, bytes: &[u8]) -> Result<JsValue, EngineError>;
```

The default implementation returns a new `EngineError::Unsupported(&'static
str)`, so `ReferenceEngine` and future engines are not forced to implement
QuickJS bytecode semantics. A new `EngineError::Bytecode(String)` variant
carries format/load failures distinctly from text `ScriptError`.

`QuickJsEngine` overrides `load_bytecode`: validate header → `Module::load` →
`Module::eval()` → drain native promise jobs and the JS microtask queue →
return `JsValue::Null`. QuickJS exception detail is captured the same way as
`eval` so failures remain actionable.

`AngNativeRuntime` gets `load_bytecode()` mirroring `load_script()`;
`RuntimeBridge` gets a matching method that maps errors to `AngNativeError`
through the existing error path. No C ABI/JNI/Swift symbol is added, and the
FFI declaration counts remain unchanged.

The AOT IIFE is declared as a module only for bytecode encoding/evaluation. No
ES module imports or exports are introduced, and the text-IIFE path remains the
default supported loader.

## 3. Error handling and tests

Invalid magic (including legacy v1 magic), truncation, length mismatch, empty
payload, oversized input, and missing/unknown/mismatched/non-UTF-8 fingerprints
return `EngineError::Bytecode` with stable messages that name the stored and
expected fingerprint on mismatch. QuickJS parse/runtime failures keep exception
detail in the error. There is no automatic fallback from bytecode to text, and
no automatic regeneration/migration of v1 artifacts: a failed bytecode load is
explicit.

Tests:

1. Unit tests for each header rejection case, the size cap, and all fingerprint
   rejection cases (wrong, missing, unknown format, non-UTF-8), plus a
   fingerprinted encode/decode round trip.
2. Unit test that `ReferenceEngine::load_bytecode` returns `Unsupported`.
3. Round-trip test: dump a tiny module to bytecode, load it, and assert an
   observable side effect.
4. Ignored AOT acceptance test `angular_aot_bytecode_loads_and_mutates_native_tree`:
   load `dist/aot/animations-app-bundle.qjsbc`, assert no bootstrap error and
   the expected native nodes, then trigger the existing deterministic
   animation and assert native opacity mutations.

Gates: `pnpm test`, `cargo test --workspace`, the relevant ignored acceptance
tests, `pnpm check:web-size`, and `python scripts/check-ffi-declarations.py`.
Evidence will be recorded in `docs/evidence/QUICKJS-BYTECODE-LOADER.md` before
tracker updates.

## 4. Scope limits

- Bytecode is trusted build output, not user input; header/size checks are
  accidental-misuse guards, not a security boundary.
- No checksum, encryption, signature, or version auto-negotiation. The
  fingerprint is a producer/compatibility check, not authenticity.
- No migration of `ANGNQJSB1` files; they must be regenerated with the v2 generator.
- No startup timing/performance claim; this priority proves load and execute.
- No ES module loader (P1-08), Hermes/V8 adapter, or platform shell wiring.
- Deterministic text-IIFE AOT loading and the socket-only MarketPulse feed are
  unchanged.
