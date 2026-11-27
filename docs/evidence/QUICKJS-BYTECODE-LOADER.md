# QuickJS bytecode loader — verified evidence (priority #5)

**Date:** 2026-11-27
**Branch:** `feat/quickjs-bytecode-loader` (from local #4 commit `93932bf`)
**Supersedes:** `docs/evidence/BYTECODE-FEASIBILITY-SPIKE.md` (deferred spike only)
**Outcome:** **Implemented and verified, Rust/runtime scope only.** No push.

## 1. What was verified

- `JsEngine::load_bytecode()` exists with an `EngineError::Unsupported` default.
- `QuickJsEngine::load_bytecode()` validates the versioned container, loads
  rquickjs module bytecode, evaluates the module, drains native jobs and the
  JS microtask queue, then returns `JsValue::Null`.
- `AngNativeRuntime::load_bytecode()` and `RuntimeBridge::load_bytecode()` mirror
  `load_script()` for lifecycle, metrics and error reporting.
- A Rust generator emits a real AOT bytecode artifact from
  `dist/aot/animations-app-bundle.js`. The generator uses QuickJS
  `JS_EVAL_FLAG_COMPILE_ONLY`, so it does not execute the AOT source.
- The ignored acceptance test boots that artifact from bytecode, finds the AOT
  native nodes, then drives a live Angular animation and verifies native
  `opacity` mutations and `start,done` callbacks.
- No C ABI, JNI, Swift, header, Android asset, iOS path or platform-shell entry
  point was added. FFI declaration counts are unchanged.

## 2. Artifact format

Header: `ANGNQJSB1` (9 bytes) + little-endian `u32` payload length = 13 bytes,
followed by raw rquickjs module bytecode. The loader rejects wrong magic,
truncated or mismatched length, empty payload, and payloads over
`64 * 1024 * 1024` bytes before the single unsafe `Module::load` call. This is
accidental-misuse framing, not encryption, signature or a security boundary.

## 3. Generator and artifact hashes

Command:

```powershell
pnpm run compile:aot:animations *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-compile-aot.log'; $c1=$LASTEXITCODE; pnpm run bundle:framework *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-bundle-framework.log'; $c2=$LASTEXITCODE; pnpm run bundle:bytecode *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-bundle-bytecode.log'; $c3=$LASTEXITCODE; Write-Output "EXITS=$c1,$c2,$c3"; Get-Content 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-bundle-bytecode.log'; Get-FileHash 'dist/aot/animations-app-bundle.js' -Algorithm SHA256 | Select-Object -ExpandProperty Hash; Get-FileHash 'dist/aot/animations-app-bundle.qjsbc' -Algorithm SHA256 | Select-Object -ExpandProperty Hash; (Get-Item 'dist/aot/animations-app-bundle.qjsbc').Length
```

Observed:

```text
EXITS=0,0,0

> angnative-monorepo@0.1.0 bundle:bytecode C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing
> cargo run -p angnative-host-engine --bin dump-bytecode -- dist/aot/animations-app-bundle.js dist/aot/animations-app-bundle.qjsbc

   Compiling angnative-host-engine v0.1.0 (C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\priority-4-easing)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.39s
     Running `target\debug\dump-bytecode.exe dist/aot/animations-app-bundle.js dist/aot/animations-app-bundle.qjsbc`
[angnative bytecode] 1412561 bytes from dist/aot/animations-app-bundle.js -> dist/aot/animations-app-bundle.qjsbc
DA06702FAF1B59A913937D344B845FB64B98CAAF1B8770B34D94CDD5858653AE
174F0B970E1BC0F295208FC3FD0AB169AE09332496704C9AE07C05A812AD9172
1412561
```

`dist/` is gitignored. CI regenerates the artifact with `pnpm run bundle:bytecode`
after the AOT/bundle steps and before the ignored acceptance tests.

## 4. Unit and bridge tests

Command:

```powershell
cargo test -p angnative-host-engine
```

Observed summary:

```text
running 37 tests
test result: ok. 37 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

Included cases: header round-trip, short artifact, wrong magic, length mismatch,
empty payload, oversized declared length, compile-only generation, QuickJS
bytecode round-trip (`globalThis.__bytecodeProbe === 42`), invalid container
rejection, and `ReferenceEngine` returning `Unsupported("bytecode loading")`.

Command:

```powershell
cargo test -p angnative-runtime-bridge
```

Observed summary:

```text
test result: ok. 12 passed; 0 failed; 19 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

This includes `test_load_bytecode_rejects_invalid_header`, which asserts the
bridge reports `phase = "load_bytecode"` and increments `reported_error_count`;
there is no silent fallback to text loading.

## 5. Ignored AOT bytecode acceptance test

Command:

```powershell
cargo test -p angnative-runtime-bridge -- --ignored --exact tests::angular_aot_bytecode_loads_and_mutates_native_tree --nocapture
```

Observed:

```text
running 1 test
test tests::angular_aot_bytecode_loads_and_mutates_native_tree ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 29 filtered out; finished in 2.22s
```

The test asserts: the `.qjsbc` magic is `ANGNQJSB1`; `load_bytecode()` succeeds;
`__animationsError` is null; `__setAnimationState` is a function; the native
tree contains the AOT text nodes `animated` and `eased`; triggering the real
trigger emits opacity samples including `0 → 0.5 → 1`; events are `start,done`;
and a native node ends at `opacity: 1`.

Full CI-equivalent ignored suite:

```powershell
cargo test -p angnative-runtime-bridge -- --ignored
```

Observed:

```text
running 19 tests
test tests::angular_aot_bytecode_loads_and_mutates_native_tree ... ok
...
test result: ok. 19 passed; 0 failed; 0 ignored; 0 measured; 12 filtered out; finished in 8.02s
```

## 6. Full Rust workspace gate

Command:

```powershell
cargo test --workspace *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-cargo-workspace.log'; $code=$LASTEXITCODE; $lines = Select-String -Path 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-cargo-workspace.log' -Pattern 'test result: ok\. (\d+) passed; (\d+) failed; (\d+) ignored'; $p=0;$f=0;$i=0; foreach($m in $lines){ $p += [int]$m.Matches[0].Groups[1].Value; $f += [int]$m.Matches[0].Groups[2].Value; $i += [int]$m.Matches[0].Groups[3].Value }; Write-Output "TOTAL passed=$p failed=$f ignored=$i lines=$($lines.Count) EXIT=$code"; Get-Content 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-cargo-workspace.log' -Tail 8; exit $code
```

Observed:

```text
TOTAL passed=166 failed=0 ignored=19 lines=23 EXIT=0
```

Cargo prints per-crate summaries; the total above sums the 23 `test result: ok`
lines. The runtime-bridge line is:

```text
test result: ok. 12 passed; 0 failed; 19 ignored; 0 measured; 0 filtered out; finished in 0.03s
```

## 7. JS/TS gate

The worktree checkout has `scripts/check-size-budget.mjs` with CRLF line endings;
plain `pnpm test` fails to import it. The file was temporarily normalized to LF,
the suite was run, and the tracked file was restored with `git checkout --`.

Command:

```powershell
$f='scripts/check-size-budget.mjs'; $t=[System.IO.File]::ReadAllText((Resolve-Path $f)); [System.IO.File]::WriteAllText((Resolve-Path $f), ($t -replace "`r`n","`n"), (New-Object System.Text.UTF8Encoding($false))); pnpm test *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-pnpm-test.log'; $code=$LASTEXITCODE; git checkout -- 'scripts/check-size-budget.mjs'; Get-Content 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-pnpm-test.log' -Tail 10; exit $code
```

Observed:

```text
 Test Files  18 passed (18)
      Tests  266 passed (266)
   Start at  21:13:52
   Duration  15.66s (transform 15.08s, setup 0ms, collect 66.49s, tests 6.71s, environment 5ms, prepare 23.72s)
```

## 8. Web size and FFI gates

Command:

```powershell
pnpm --filter @angnative/web run build *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-web-build.log'; $c1=$LASTEXITCODE; pnpm run compile:aot:web *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-web-compile.log'; $c2=$LASTEXITCODE; pnpm run bundle:aot:web *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-web-bundle.log'; $c3=$LASTEXITCODE; pnpm check:web-size *> 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-web-size.log'; $c4=$LASTEXITCODE; Write-Output "EXITS=$c1,$c2,$c3,$c4"; Get-Content 'C:\Users\Moid Iqbal\AppData\Local\Temp\opencode\priority5-web-size.log'
```

Observed:

```text
EXITS=0,0,0,0

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
```

No JNI/Swift/header file was changed by this priority.

## 9. Diff hygiene

Command:

```powershell
git diff --check; git status --short
```

Observed (no whitespace errors; only the intended implementation, evidence and
tracker files):

```text
 M .github/workflows/ci.yml
 M STATUS.md
 M crates/angnative-host-engine/src/engine.rs
 M crates/angnative-host-engine/src/errors.rs
 M crates/angnative-host-engine/src/lib.rs
 M crates/angnative-host-engine/src/quickjs.rs
 M crates/angnative-host-engine/src/reference_engine.rs
 M crates/angnative-host-engine/src/runtime.rs
 M crates/angnative-runtime-bridge/src/lib.rs
 M docs/UNSUPPORTED-FEATURES.md
 M docs/architecture/issue-register.md
 M docs/plans/2026-11-27-quickjs-bytecode-loader-design.md
 M package.json
?? crates/angnative-host-engine/src/bin/
?? crates/angnative-host-engine/src/bytecode.rs
?? docs/evidence/QUICKJS-BYTECODE-LOADER.md
```

## 10. Scope limits

- No startup-speed, load-time or performance improvement is claimed.
- No checksum, encryption, signature or authenticity check is claimed.
- Bytecode is trusted build output; header and size checks only prevent
  accidental misuse before the unsafe load.
- No ES module loader (P1-08), Hermes/V8 adapter, SSR, DOM-library parity or
  platform-shell wiring is added.
- The socket-only MarketPulse feed and the #4 deterministic/host-clock
  animation paths are unchanged.
- The main repository's unrelated Taffy 0.15.1 working-tree changes were not
  touched; all work stayed in the isolated worktree and no push was performed.
