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

The artifact is `ANGNQJSB1` (8 bytes), followed by a fixed little-endian `u32`
payload length, followed by raw rquickjs module bytecode. The loader rejects
wrong magic, truncated/mismatched length, empty payload, and payloads over
`64 * 1024 * 1024` bytes before calling the one unsafe `Module::load` API.

A new Rust binary
`crates/angnative-host-engine/src/bin/dump-bytecode.rs` reads a self-contained
AOT IIFE source, declares it as a module with a stable virtual name, writes
little-endian module bytecode with source/debug information stripped, and emits
the versioned artifact:

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

Invalid magic, truncation, length mismatch, empty payload, and oversized input
return `EngineError::Bytecode` with stable messages. QuickJS parse/runtime
failures keep exception detail in the error. There is no automatic fallback
from bytecode to text: a failed bytecode load is explicit.

Tests:

1. Unit tests for each header rejection case and the size cap.
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
- No checksum, encryption, signature, or version auto-negotiation.
- No startup timing/performance claim; this priority proves load and execute.
- No ES module loader (P1-08), Hermes/V8 adapter, or platform shell wiring.
- Deterministic text-IIFE AOT loading and the socket-only MarketPulse feed are
  unchanged.
