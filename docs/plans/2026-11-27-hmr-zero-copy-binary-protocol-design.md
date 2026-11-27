# Design — HMR, zero-copy JNI and versioned binary mutation protocol (priority #6+)

**Date:** 2026-11-27
**Status:** validated with the user; not yet implemented.
**Round bar:** all three features get real code, passing tests and retained evidence. No deferred claim for the declared scope.

## Objective

Lift three deferrals in `docs/UNSUPPORTED-FEATURES.md` with scoped, verifiable implementations:

1. hot module replacement on web and embedded QuickJS with component-state preservation,
2. a borrowed-buffer JNI transport with an explicit per-call lifetime, and
3. a versioned binary mutation protocol alongside the existing JSON fallback.

Out of scope remains: Hermes/V8, SSR/hydration, full DOM-library parity, and AG Grid/ECharts full features. No iOS/macOS runtime claim is added by this round.

## Decisions (validated with the user)

- **Transport:** dual-path, versioned binary framing plus JSON fallback. JSON encoders, decoders and conformance fixtures stay unchanged so existing gates remain green.
- **JNI:** borrowed direct `ByteBuffer` per call. The Java/Kotlin side allocates; Rust borrows for the synchronous call only; no pointer is persisted. Return strings keep the existing owned/free contract.
- **HMR:** one invalidation/reload pipeline, proven on both the web adapter and native QuickJS. Local component state (for example a counter value) must survive a code edit.
- **HMR claim boundary:** verified for counter-style apps that opt into the dev HMR contract. Generic third-party Angular components without HMR hooks remain not verified and are not claimed.
- **Physical device:** OnePlus Open `b6640c8f` is attached and may be used for retained JNI/HMR evidence. Emulator `emulator-5554` (API 34) remains a fallback; every device claim records the exact device and APK hash.

## Architecture — binary protocol

`packages/contract/src/binary.ts` encodes batches; `crates/angnative-host-core` decodes them. Framing:

| Offset | Type | Field |
|---|---|---|
| 0 | 4 bytes | magic `ANB1` |
| 4 | `u16` LE | version, currently `1` |
| 6 | `u32` LE | frame id |
| 10 | `u64` LE | timestamp (ms) |
| 18 | `u32` LE | command count |
| 22 | repeated | commands |

Each command is a `u8` opcode followed by little-endian fields. Strings are `u32` byte length + UTF-8 bytes; node ids are `u64` LE. Opcode mapping follows the canonical table in `docs/SPECIFICATIONS.md`: `0x01 CreateNode`, `0x02 SetProps`, `0x03 SetText`, `0x04 InsertChild`, `0x05 RemoveChild`, `0x06 DropNode`. Current `RemoveNode` maps to `DropNode`; current `SetStyle`, which has no canonical opcode, is `0x07 SetStyle` and is documented as an extension.

Decoding is strict: reject wrong magic, unsupported version, truncated fields, invalid UTF-8, unknown opcodes, and trailing bytes. A versioned decode error names the offending field. The decoder produces the same `CommandBatch`/`Command` model as JSON, so `MutationEngine`, the host bridge, view emitters and all existing conformance tests are shared by both paths.

## Architecture — borrowed-buffer JNI

Kotlin allocates a direct `ByteBuffer` per batch and passes it to C++. C++ obtains the address with `GetDirectBufferAddress` and calls a new C ABI entry:

`angnative_host_process_batch_bytes(handle, const uint8_t *ptr, size_t len, uint32_t *out_len) -> char *`

Rust wraps `ptr/len` in `std::slice::from_raw_parts` for the duration of that synchronous call, decodes with the binary decoder, and processes through the same staging/commit path as `Host::process_batch_json`. No slice or pointer is stored after return. Errors are returned through the existing `last_error` mechanism; the returned ops string keeps the existing caller-frees contract.

`angnative_host_process_batch` (NUL-terminated JSON string) remains for compatibility and as the JSON fallback. The new ABI declaration is added to `crates/angnative-host-ffi/include/angnative_host_ffi.h`; Kotlin externals and C++ JNI exports are added to the two Android shells checked by `scripts/check-ffi-declarations.py`. Swift/iOS is not changed.

## Architecture — HMR pipeline

`packages/cli/src/dev.ts` replaces its placeholder with a real loop:

1. resolve the genuine Angular compiler from `@angular/compiler-cli` (same resolution used by `scripts/compile-aot.mjs`) and run an incremental AOT compile,
2. rebundle with the existing esbuild path, measuring compile and bundle separately,
3. enforce `maxComponentCompileMs = 200` and `maxInnerLoopMs = 500` and record both values in `CompileResult`,
4. watch source files (not only the generated entry), debounce bursts, and publish a versioned reload event on `/hmr` (SSE) with a cache-busted bundle URL.

A small `HmrRuntime` in `packages/contract` owns module registration and replacement. A module registers `id`, `factory` and optional `snapshot()` / `restore(state)` hooks. `replace(id, factory)` snapshots the live instance, runs the new factory **in the same JS realm**, restores state, and triggers re-render. No page reload or JS-context reset occurs.

- **Web:** the dev bundle registers the root factory; on SSE the client fetches the new bundle, imports it via a blob URL, calls `replace()`, then `appRef.tick()`. `packages/web/src/renderer.ts` and `DomBackend` are unchanged.
- **Native:** a runtime-bridge reload entry evaluates the new source in the existing QuickJS realm, invokes the same `HmrRuntime.replace()` through the JS shim, drains microtasks and flushes mutations into the host. The existing `load_script` path and all production bundles are unaffected.

The counter fixture implements `snapshot()` / `restore()` with its signal value, so tests assert a visible count survives the edit on both paths. An unmodified app with no hooks falls back to a full re-evaluation; state preservation is not claimed for it.

## Tests

- **Codec:** TS and Rust round-trip tests for every command kind; negative cases for magic, version, truncation, invalid UTF-8, unknown opcode and trailing bytes; one shared golden-vector fixture checked by both languages; JSON path regression tests remain untouched.
- **JNI/lifetime:** Rust test proving the byte-slice consumer cannot retain data after return; JNI instrumentation test on OnePlus (emulator fallback) that submits a direct buffer, mutates it after the call, and asserts the earlier result and native tree are unchanged.
- **HMR:** CLI test proving `ngc` is really invoked and budgets are recorded; web Playwright test measuring save → SSE → new DOM text under 1 s with the counter value unchanged; Rust QuickJS test measuring reload → state restore → mutation flush with the same count, under 1 s.
- **Gates:** `cargo test --workspace`, `cargo test -p angnative-runtime-bridge -- --ignored`, `pnpm test`, `python scripts/check-ffi-declarations.py`, plus APK size check against the ≤20 MB decimal cap.

## Evidence and delivery

Retain exact commands, outputs, device id and APK SHA-256 in a new `docs/evidence/HMR-ZEROCOPY-BINARY-PROTOCOL.md`. Update `docs/UNSUPPORTED-FEATURES.md` only for verified items; where a proof is missing, write “not verified”. Keep `docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md` unchanged. Do not commit or push until every gate is green.

The working tree contains unrelated local modifications (including `Cargo.toml`, Rust bridge code, JNI wrappers, docs and Taiga `.a` artifacts). They are preserved; implementation edits must be additive and must not revert or overwrite them. The design doc is committed on its own.
