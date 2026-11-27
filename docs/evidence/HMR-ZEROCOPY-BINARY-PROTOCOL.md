# HMR + borrowed-buffer JNI + ANB1 binary protocol — retained proof (2026-11-27)

Scope: priority #6+ implementation in the web adapter, embedded QuickJS runtime
bridge, host FFI and the flagship `showcases/ecosystem-core` Android shell.
No WebView, no iOS/macOS runtime claim, no Hermes/V8 adapter, no SSR/hydration,
and no head-to-head performance superiority claim is made. Every claim below is
backed by a passing test or a retained artifact; anything else says
**not verified**.

Approved design: `docs/plans/2026-11-27-hmr-zero-copy-binary-protocol-design.md`
(commit `9b9aaea`; the 26-byte ANB1 header correction is an uncommitted working
tree edit). Decisions implemented: web + native HMR in one invalidation/reload
pipeline; versioned ANB1 binary framing with JSON fallback; borrowed direct
`ByteBuffer` per synchronous JNI call; component state preserved.

## 1. ANB1 binary mutation protocol (versioned, JSON fallback)

- TypeScript encoder/decoder: `packages/contract/src/binary.ts`, exported via
  `packages/contract/src/index.ts` and `packages/contract/package.json`.
- Rust encoder/decoder: `crates/angnative-host-core/src/binary.rs`, exported via
  `crates/angnative-host-core/src/lib.rs`.
- Header is **26 bytes** (`ANB1` magic, `u16` binary version, `u64` frame id,
  `u64` timestamp, `u32` command count). Decoders reject bad magic, unsupported
  versions, truncated/trailing bytes, unknown opcodes, invalid UTF-8 and invalid
  JSON property values; decoded commands are owned and survive input reuse.
- Shared golden vector `packages/testing/fixtures/binary-batch.hex` is verified
  byte-for-byte in both languages:
  - `pnpm --filter @angnative/contract test` → **36 passed** (9 binary tests).
  - `cargo test -p angnative-host-core --test binary_spec` → **10 passed**,
    including `decodes_shared_golden_vector`,
    `encodes_shared_golden_vector_byte_for_byte` and
    `decoder_output_is_owned_and_survives_the_input_buffer`.
- `Host::process_batch_binary` stages then commits through the same path as
  JSON, so a failed batch leaves the shadow tree unchanged. C ABI:
  `angnative_host_process_batch_bytes(handle, const uint8_t*, size_t, uint32_t*)`.
  `cargo test -p angnative-host-ffi --test ffi_spec` → **17 passed**, including
  `process_batch_bytes_rejects_malformed_frames` and
  `process_batch_bytes_borrows_only_for_the_synchronous_call`.
- JSON remains the fallback wire format; no claim that binary replaces it.

## 2. Zero-copy borrowed-buffer JNI

- Kotlin/JNI (both `platforms/android/app` and `showcases/ecosystem-core`):
  `external fun nativeHostProcessBatchBytes(handle: Long, buffer: ByteBuffer)`,
  wrapped by `processBatchDirect(buffer: ByteBuffer)`. The caller allocates the
  direct buffer; Rust borrows the address with `GetDirectBufferAddress` for one
  synchronous call, decodes into an owned batch, and retains no pointer after
  return. `processBatchBinary(ByteArray)` delegates to the same entry point.
- Static FFI gate (run after every edit):
  `FFI declaration check OK: 40 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 31 header declarations; iOS unified staticlib wiring OK`.
  This is static consistency only; iOS execution remains **not verified**.
- Physical-device lifetime proof: the instrumentation test zeroes the caller's
  direct buffer after the call and asserts the committed tree is unchanged
  (see §4). No throughput/benchmark comparison against JSON was run, so no
  "faster than JSON" claim is made.

## 3. HMR — one pipeline, web + native, state preserved

- Shared contract `packages/contract/src/hmr.ts`: module registry with
  `snapshot → dispose → factory → restore` and generation counters
  (`packages/contract/src/hmr.spec.ts`, 4 tests).
- CLI dev server `packages/cli/src/dev.ts`: real Angular `ngc` via
  `scripts/compile-aot.mjs` (extra args forwarded, `outDir` resolved relative to
  the tsconfig), file watching, dual web/native bundles and an `/hmr` SSE
  endpoint. `pnpm --filter @angnative/cli test` → **34 passed**; observed inner
  loop logs across root runs: `48 ms / 80 ms` and `53 ms / 85 ms` (an earlier
  run showed 47 ms / 79 ms). These are dev-server inner-loop timings, not a
  browser save-to-screen measurement.
  Fixture: `packages/testing/fixtures/hmr/counter.component.ts` +
  `tsconfig.aot.hmr.json`, compiled by real ngc.
- Web client `packages/web/src/hmr.ts`: subscribes to the **named** `event: hmr`
  SSE message (the previous `source.onmessage` default silently missed every
  DevServer update), imports a fresh blob module, replaces DOM content. The
  regression guard is `packages/web/src/hmr.spec.ts` → **2 passed**, including
  `subscribes to the named "hmr" SSE event by default`. The remaining
  happy-dom test injects `subscribe`/`loadModule`, so it cannot catch the SSE
  wiring bug; the browser harness below is the real-client proof.
- Native QuickJS: idempotent `crates/angnative-runtime-bridge/src/hmr_shim.js`
  installed into the existing realm (also at bridge creation),
  `RuntimeBridge::hmr_reload`, C ABI
  `angnative_runtime_bridge_hmr_reload` (0 success / 1 error / 2 wrong thread),
  and Kotlin wrappers in both Android shells.
  `hmr_reload_preserves_counter_state_in_same_quickjs_realm` proves: same realm
  marker survives, old instance `dispose` runs, new factory code is live,
  counter state `7` survives, generation advances to `2`, and the new code's
  mutation batch is committed to the Host tree. Runtime-bridge lib suite:
  **12 passed / 0 failed / 17 ignored**.
- Event delivery wording is unchanged and exact: **one delivery per event (canonical JS thread-local path + legacy FFI fallback)**.

### 3.1 Real-browser web HMR harness (`web HMR client under a harness`)

`scripts/verify-hmr-browser.py` + `scripts/hmr-browser/harness-entry.ts`
exercise the **real** `DevServer` (no `compileHook`), the real `startWebHmr`
(no `subscribe`, `loadModule` or `fetchImpl` override), a real Chromium
`EventSource`, real `fetch` + blob-module import, and a real DOM mutation
across 10 measured save→SSE→DOM cycles. Retained evidence:

- `docs/evidence/hmr-browser-save-sse-dom-2026-11-27.json` — 10 raw runs,
  environment/provenance, exact commands and hashes.
- `docs/assets/hmr-browser-dev-server.log`, `docs/assets/hmr-browser-sse.txt`,
  `docs/assets/hmr-browser-before.png`, `docs/assets/hmr-browser-after.png`.

**Verified by that evidence:** named SSE delivery reaches the real client,
local counter state `7` survives every measured replacement, the realm token
and navigation count stay unchanged (no full page reload), and the browser
receives a versioned bundle URL for every applied update.

**Not verified:** save→SSE→DOM under 1 s. Measured save→DOM was
**6,548–6,571 ms** (p50 6,562 ms), dominated by real `ngc` compilation and the
duplicate `fs.watch` reload events; SSE→DOM was 1,616–2,378 ms. The `< 1 s`
end-to-end budget is therefore **not claimed**. This is harness-level proof,
not shipping dev-app auto-reload (the dev root HTML does not wire the client),
and ANB1 is not used on the web SSE path.

## 4. Historical physical-device run (OnePlus `b6640c8f`)

`adb devices -l` at the time of that earlier run:

```
b6640c8f               device product:CPH2551 model:CPH2551 device:OP5973L1 transport_id:1
emulator-5554          device product:sdk_gphone64_x86_64 model:sdk_gphone64_x86_64 device:emu64xa transport_id:2
```

Test `showcases/ecosystem-core/app/src/androidTest/java/com/angnative/app/HmrZeroCopyBinaryTest.kt`
was installed and invoked on the physical OnePlus in that earlier run:

```
$env:ANDROID_SERIAL='b6640c8f'; adb -s b6640c8f install -r app-debug.apk
adb -s b6640c8f install -r app-debug-androidTest.apk
adb -s b6640c8f shell am instrument -w -e class com.angnative.app.HmrZeroCopyBinaryTest \
  com.angnative.demo.ecosystem.core.test/androidx.test.runner.AndroidJUnitRunner

com.angnative.app.HmrZeroCopyBinaryTest:.

Time: 0.043

OK (1 test)
```

Retained raw log: `docs/assets/hmr-zerocopy-binary-oneplus.log`
(79 bytes, SHA-256 `2F6268B9FE3CB3E925713E1C74D0FD28058734D5822FF1E2C31D06C2A5CD615C`).
It records the passing result but **not** device properties, APK hashes or
exact command provenance, so no new physical-device claim is made from it.
The test asserts, on device: ANB1 binary batch creates two native nodes with the
expected text; caller-buffer zeroing afterwards does not change the tree; and
HMR reload yields the exact native text `7|424242|true|2` — restored state,
same-realm marker, disposed old instance and advanced generation.

### 4.1 Emulator execution with full provenance

`python scripts/collect-android-hmr-evidence.py emulator-5554 docs/assets/hmr-zerocopy-binary-emulator.log`
installed the same APKs and ran the same test on `emulator-5554`
(`sdk_gphone64_x86_64`, Android 14 / SDK 34, `x86_64`). Raw output is retained
in `docs/assets/hmr-zerocopy-binary-emulator.log`: UTC timestamps, exact `adb`
commands, device properties, both APK hashes and `OK (1 test)` in 0.102 s.
APK identity used for that run:

- `app-debug.apk` — 22,805,896 bytes, SHA-256
  `D7BE870A73F646FD7ED17C9BD29DE47FF4EE08B15615F2EA139CA9F6B500584A`
- `app-debug-androidTest.apk` — 413,350 bytes, SHA-256
  `35F6C0A6C1FB980B8802F191D3D348F214D029BA22A5C10285843109E0371E86`

**Physical-device rerun with these identical hashes is not verified**: the
`adb devices -l` transcript at the time of the emulator run listed only
`emulator-5554`, and `adb -s b6640c8f get-state` returned
`error: device 'b6640c8f' not found`. No emulator-vs-physical comparison or
superiority claim is made.

## 5. APK size cap (non-negotiable ≤ 20 MB)

Built with `.\gradlew.bat clean assembleRelease assembleDebugAndroidTest --no-daemon`
from `showcases/ecosystem-core` after the 3-ABI Rust rebuild
(`pwsh -File scripts/build-android-native.ps1`: `arm64-v8a`, `armeabi-v7a`,
`x86_64`, all "Android Rust libraries rebuilt (unified runtime bridge).").

- `app/build/outputs/apk/release/app-release.apk`
- Bytes: **17,752,298** (16.930 MiB / 17.752 decimal MB)
- SHA-256: `164EE6CB84F2ACC7AD49DCD719770539F903B4B0E2F678C4E06B9E26FBAC48D2`
- Under the 20,000,000-byte cap by **2,247,702** bytes.
- Prior retained APK was 17,539,306 bytes (16.727 MiB); this round adds
  212,992 bytes and remains under the cap.
- androidTest APK SHA-256:
  `35F6C0A6C1FB980B8802F191D3D348F214D029BA22A5C10285843109E0371E86`.

## 6. Full gate (after all final edits)

| Command | Exact result |
|---|---|
| `cargo test --workspace` | **167 passed / 0 failed / 17 ignored** |
| `cargo test -p angnative-runtime-bridge -- --ignored` | **17 passed / 0 failed / 12 filtered out** |
| `pnpm test` | **20 test files passed / 276 tests passed** |
| `python scripts/check-ffi-declarations.py` | `FFI declaration check OK: 40 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 31 header declarations; iOS unified staticlib wiring OK` |

One root-cwd-only failure was found and fixed during the gate: the CLI dev spec
resolved the shared HMR fixture relative to `process.cwd()`, which broke
workspace-root `pnpm test`; it now resolves from the test file's URL. No product
code behavior was changed by that fix.

## 7. Explicitly not verified

- Real browser save→SSE→DOM `< 1 s` Playwright timing: **not verified**. The
  harness measured 6,548–6,571 ms save→DOM (see §3.1); only delivery, state
  preservation and no-reload are claimed.
- iOS/macOS build, simulator or device execution (static FFI wiring only).
- Physical-device rerun of this round's binary/HMR instrumentation with the
  documented APK hashes (`b6640c8f` was not attached; see §4.1).
- Zero-copy throughput comparison against JSON; no performance superiority claim.
- WebView, Hermes/V8, SSR/hydration and unchanged DOM-library parity (out of scope).
