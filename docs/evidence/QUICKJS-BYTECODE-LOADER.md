# QuickJS bytecode loader — verified evidence (priority #5)

**Date:** 2026-11-28
**Branch:** `feat/quickjs-bytecode-loader` (from local #4 commit `93932bf`)
**Supersedes:** `docs/evidence/BYTECODE-FEASIBILITY-SPIKE.md` (deferred spike only)
**Outcome:** **Implemented and verified, Rust/runtime scope only.** No push, no merge.

**2026-11-28 integration:** three code-review should-fixes were implemented in
separate isolated worktrees at `d4e355c` and integrated here, then the full gate
set was rerun on the combined tree:

1. CI now builds the MarketPulse AOT/bundle artifact before the ignored tests.
2. A pending module-evaluation promise is rejected instead of reported as success.
3. The container is v2 (`ANGNQJSB2`) and carries a producer/format fingerprint.

All numbers below are from the **integrated tree**, not from the individual fix
worktrees. Historical v1 results are retained only where they explain migration.

## 1. What was verified

- `JsEngine::load_bytecode()` exists with an `EngineError::Unsupported` default.
- `QuickJsEngine::load_bytecode()` validates the v2 container and its producer
  fingerprint (`decode()` in `bytecode.rs`), loads rquickjs module bytecode,
  drains native jobs and the JS microtask queue, then returns `JsValue::Null`.
  A module eval promise that is still pending after the 1024-job drain cap now
  returns `EngineError::Bytecode` instead of a false success.
- `AngNativeRuntime::load_bytecode()` and `RuntimeBridge::load_bytecode()` mirror
  `load_script()` for lifecycle, metrics and error reporting.
- A Rust generator emits a real AOT bytecode artifact from
  `dist/aot/animations-app-bundle.js`. The generator uses QuickJS
  `JS_EVAL_FLAG_COMPILE_ONLY`, so it does not execute the AOT source.
- The ignored acceptance tests boot that artifact from bytecode, find the AOT
  native nodes, then drive a live Angular animation and verify native
  `opacity` mutations and `start,done` callbacks. All 19 ignored tests pass on
  the integrated tree with a freshly generated v2 artifact.
- No C ABI, JNI, Swift, header, Android asset, iOS path or platform-shell entry
  point was added. FFI declaration counts are unchanged.

## 2. Artifact format (v2)

```text
ANGNQJSB2 (9 bytes) | u32 LE payload length | u32 LE fingerprint length |
UTF-8 fingerprint | raw rquickjs module bytecode
```

Fixed header = 17 bytes; payload cap unchanged at `64 * 1024 * 1024` bytes;
fingerprint cap = 256 bytes. Observed fingerprint on this Windows/x86_64 host:

```text
qjsbc-v2/rquickjs-0.14.0/quickjs-ng-0.16.2/x86_64-windows-le-64
```

`runtime_fingerprint()` composes: format tag `qjsbc-v2`; compile-time constant
`RQUICKJS_VERSION = "0.14.0"` (no stable rquickjs version API; pinned by
workspace `Cargo.toml`/`Cargo.lock`); `qjs::JS_GetVersion()` from the linked
QuickJS-ng C library; and `std::env::consts::{ARCH, OS}`, endianness and pointer
width. The loader rejects wrong magic (with a dedicated legacy `ANGNQJSB1`
migration message), truncated/mismatched lengths, empty payload, oversized
declared lengths, and missing/unknown/mismatched/non-UTF-8 fingerprints, all
before the single unsafe `Module::load` call. This is accidental-misuse and
producer-compatibility framing, **not** encryption, signature or a security
boundary.

Because arch/OS are part of the fingerprint, an artifact must be generated for
the target it will be loaded on; a Windows-generated `.qjsbc` is intentionally
rejected by an Android arm64 runtime. No platform wiring exists in this scope,
so this is a packaging consideration for a future integration, not a current
failure.

## 3. Generator, CI artifact order and hashes

The CI `angular-runtime` job now builds the missing MarketPulse artifact before
the ignored tests. Exact order in `.github/workflows/ci.yml`:

```powershell
pnpm run compile:aot
pnpm run compile:aot:router
pnpm run compile:aot:forms
pnpm run compile:aot:i18n
pnpm run compile:aot:animations
pnpm run compile:aot:marketpulse
pnpm run bundle:quickjs
pnpm run bundle:forms
pnpm run bundle:framework
pnpm run bundle:marketpulse
pnpm run bundle:third-party
pnpm run bundle:bytecode
cargo test -p angnative-runtime-bridge -- --ignored
```

`compile:aot:marketpulse` and `bundle:marketpulse` are required because
`market_pulse_bundle_drives_candlestick_and_animation` reads
`dist/aot/market-pulse-bundle.js` directly; `bundle:bytecode` generates the
separate `dist/aot/animations-app-bundle.qjsbc` artifact. The CI fix agent's
local replay of the MarketPulse commands observed `EXIT[compile:aot:marketpulse]=0`,
`EXIT[bundle:marketpulse]=0`, `EXIT[bundle:bytecode]=0` and the two acceptance
tests passing. That replay ran on Windows/Node v26.13.1 with prebuilt
`@angnative/*` package output; GitHub Actions itself (`ubuntu-latest`, Node
22.22.x) was **not** executed.

`dist/` is gitignored. On the integrated tree the v2 artifact was regenerated
from the existing AOT source with:

```powershell
pnpm run bundle:bytecode
```

Observed:

```text
[angnative bytecode] 1412628 bytes from dist/aot/animations-app-bundle.js -> dist/aot/animations-app-bundle.qjsbc (fingerprint: qjsbc-v2/rquickjs-0.14.0/quickjs-ng-0.16.2/x86_64-windows-le-64)
BUNDLE_EXIT=0
MAGIC=ANGNQJSB2 PAYLOAD_LEN=1412548 FINGERPRINT_LEN=63 FINGERPRINT=qjsbc-v2/rquickjs-0.14.0/quickjs-ng-0.16.2/x86_64-windows-le-64 TOTAL=1412628
QJSBC_SHA256=4E637574E109946D2536E8E65EBFC723A4DF2880382D673D4C9B353E6EABB2FC
SOURCE_SHA256=DA06702FAF1B59A913937D344B845FB64B98CAAF1B8770B34D94CDD5858653AE
```

The previous v1 artifact was 1412561 bytes with SHA-256
`174F0B970E1BC0F295208FC3FD0AB169AE09332496704C9AE07C05A812AD9172`. v1 files
are not auto-migrated; they fail with a regenerate message.

## 4. Unit and bridge tests

Command:

```powershell
cargo test -p angnative-host-engine
```

Observed (integrated tree):

```text
test result: ok. 45 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

Included cases: fingerprinted encode/decode round-trip, deterministic
fingerprint contents, truncated header (all `0..HEADER_LEN` prefixes plus a
short fingerprint body), wrong magic, legacy `ANGNQJSB1` magic with migration
message, payload/fingerprint length mismatch, empty payload, oversized declared
payload and fingerprint lengths, wrong/missing/unknown/non-UTF-8 fingerprints,
the never-settling module-promise regression, compile-only generation, QuickJS
bytecode round-trip (`globalThis.__bytecodeProbe === 42`), invalid container
rejection, and `ReferenceEngine` returning `Unsupported("bytecode loading")`.

Command:

```powershell
cargo test -p angnative-runtime-bridge
```

Observed (integrated tree):

```text
test result: ok. 12 passed; 0 failed; 19 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

This includes `test_load_bytecode_rejects_invalid_header`, which asserts the
bridge reports `phase = "load_bytecode"` and increments `reported_error_count`;
there is no silent fallback to text loading. The ignored AOT test's expected
magic is now `ANGNQJSB2`.

## 5. Ignored AOT acceptance suite (v2 artifact)

Command:

```powershell
cargo test -p angnative-runtime-bridge -- --ignored
```

Observed on the integrated tree with the freshly regenerated v2 artifact:

```text
running 19 tests
test tests::angular_aot_bytecode_loads_and_mutates_native_tree ... ok
test tests::market_pulse_bundle_drives_candlestick_and_animation ... ok
test tests::angular_animations_load_in_quickjs ... ok
test tests::angular_timed_easing_drives_native_styles ... ok
... 19 entries total ...
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 9.54s
```

The bytecode test asserts: `.qjsbc` magic `ANGNQJSB2`; `load_bytecode()` succeeds;
`__animationsError` is null; `__setAnimationState` is a function; the native
tree contains the AOT text nodes `animated` and `eased`; the real trigger emits
opacity samples including `0 → 0.5 → 1`; events are `start,done`; and a native
node ends at `opacity: 1`.

## 6. Full Rust workspace gate

Command:

```powershell
cargo test --workspace *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\integrated-workspace.log'; $code=$LASTEXITCODE; $lines=Select-String -Path 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\integrated-workspace.log' -Pattern 'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored'; $p=0;$f=0;$i=0; foreach($m in $lines){ $p += [int]$m.Matches[0].Groups[1].Value; $f += [int]$m.Matches[0].Groups[2].Value; $i += [int]$m.Matches[0].Groups[3].Value }; Write-Output "TOTAL passed=$p failed=$f ignored=$i lines=$($lines.Count) EXIT=$code"
```

Observed:

```text
TOTAL passed=174 failed=0 ignored=19 lines=23 EXIT=0
```

Cargo prints per-crate summaries; the total sums the 23 `test result: ok` lines.
The increase from 166 to 174 is the 7 new bytecode-container tests plus the 1
pending-promise regression test.

## 7. JS/TS gate

The checkout has CRLF line endings in `scripts/check-size-budget.mjs` and
`scripts/check-size-budget.spec.ts`; plain `pnpm test` fails to import them. Both
files were temporarily normalized to LF, the suite was run, and both were
restored with `git checkout --` immediately afterwards.

Observed:

```text
 Test Files  18 passed (18)
      Tests  266 passed (266)
   Start at  00:33:48
   Duration  6.93s (transform 7.85s, setup 0ms, collect 21.95s, tests 9.09s, environment 3ms, prepare 3.42s)
PNPM_TEST_EXIT=0
```

## 8. Web size and FFI gates

Command:

```powershell
pnpm check:web-size
```

Observed:

```text
[angnative web] raw bundle: 355177 B
[angnative web] gzip bundle: 112461 B (budget 128000 B / 125.0 KB, headroom 15539 B)
[angnative web] forbidden packages: 0
[angnative web] unlinked partial declarations: 0
WEB GATE EXIT=0
```

Command:

```powershell
python scripts/check-ffi-declarations.py
```

Observed:

```text
FFI declaration check OK: 36 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 29 header declarations; iOS unified staticlib wiring OK
FFI_EXIT=0
```

No JNI/Swift/header file was changed by this priority.

## 9. Pending async module promise regression

`QuickJsEngine::load_bytecode()` no longer treats `promise.result::<()>() == None`
as success. `None` means the module evaluation promise is still pending, so the
loader returns:

```text
EngineError::Bytecode("module promise: still pending after draining up to 1024 context jobs")
```

The test `quickjs::tests::quickjs_load_bytecode_rejects_never_settling_module_promise`
compiles a module that top-level `await`s a promise which never settles while
recursively re-queuing native jobs (exercising the 1024-job cap). With the old
`Some(Ok(())) | None => {}` behavior the same test failed with
`called Result::unwrap_err() on an Ok value: Null`. It passes on the integrated
tree as part of the 45-test host-engine gate above.

## 10. Diff hygiene

Command:

```powershell
git diff --check; Write-Output "check_exit=$LASTEXITCODE"; git status --short
```

Observed after the integration:

```text
check_exit=0
 M .github/workflows/ci.yml
 M crates/angnative-host-engine/src/bin/dump-bytecode.rs
 M crates/angnative-host-engine/src/bytecode.rs
 M crates/angnative-host-engine/src/quickjs.rs
 M crates/angnative-runtime-bridge/src/lib.rs
 M docs/evidence/QUICKJS-BYTECODE-LOADER.md
 M docs/plans/2026-11-27-quickjs-bytecode-loader-design.md
```

No whitespace errors. Changes are uncommitted for review; no push, merge or tag
change was performed.

## 11. Scope limits

- No startup-speed, load-time or performance improvement is claimed.
- No checksum, encryption, signature or authenticity check is claimed. The v2
  fingerprint is a producer/compatibility check only.
- v1 `ANGNQJSB1` artifacts are not migrated or auto-detected; they must be
  regenerated with `pnpm run bundle:bytecode` and fail with a migration message.
- Bytecode is trusted build output; header, size and fingerprint checks only
  prevent accidental misuse and cross-build loading before the unsafe load.
- `RQUICKJS_VERSION` is a compile-time constant; bumping rquickjs without
  updating it requires regenerating all `.qjsbc` artifacts.
- No ES module loader (P1-08), Hermes/V8 adapter, SSR, DOM-library parity or
  platform-shell wiring is added.
- The socket-only MarketPulse feed and the #4 deterministic/host-clock
  animation paths are unchanged.
- The main repository's unrelated working-tree changes were not touched; all
  work stayed in isolated worktrees and no push was performed.
