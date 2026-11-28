# Release v0.4.0 — fresh-checkout determinism evidence

**Date:** 2026-11-28
**Commit under test:** `29af2487c1cdcd3a854d33618db8ef6916bc7a4e` (`release/v0.4.0`)
**Parents:** `26772309f460b4eb776f78eb0f28def9042bd06a` (feature branch) and
`dea0beb0bc836b1ed62e2f12951e0b5c92d5f018` (pushed master)
**Full command logs:** `docs/evidence/release-0.4.0-logs/`

## 1. What "fresh" means here

A detached git worktree was created directly from the reconciled commit:

```powershell
git worktree add --detach C:\Users\Moid Iqbal\.local\share\opencode\worktree\73380c\phase-0.5-fresh 29af248
```

Observed at start (`00-environment.log`):

```text
node=v26.13.1
pnpm=10.34.9
cargo=cargo 1.102.0-nightly (ede6cefea 2026-11-11)
rustc=rustc 1.102.0-nightly (5f4effd57 2026-11-13)
commit=29af2487c1cdcd3a854d33618db8ef6916bc7a4e
node_modules=False
target=False
```

`dist/` existed only because three showcase artifacts are tracked
(`dist/showcases/market-pulse/app-debug.apk`, `compatibility-report.md`,
`source.zip`). No generated AOT bundle, QuickJS bundle, bytecode artifact,
`node_modules` or `target/` was present.

**Toolchain caveat:** the repository CI pins Node `22.22.x` and
`nightly-2026-11-14`; this local replay used Node `26.13.1` and nightlies dated
2026-11-11/13. This proves determinism of the checked-out source and scripts on
this Windows host, not byte-identical reproduction of the GitHub Actions runner.

## 2. Merge conflict resolution

The merge of `master@dea0beb` into the feature branch had two real conflicts:

- `crates/angnative-runtime-bridge/src/lib.rs`: kept **both** `load_bytecode`
  (branch) and `hmr_reload` (master), including the `HMR_SHIM` import and
  `HmrReloadResult` type from master.
- `docs/UNSUPPORTED-FEATURES.md`: combined master's implemented HMR / ANB1 /
  zero-copy facts with the branch's host-clock easing and `ANGNQJSB2` facts,
  and removed the obsolete "bytecode deferred" and "easing deferred" rows.
  Device results are labelled as retained historical evidence, not freshly
  repeated in this tree.

Additional auto-merge cleanup:

- Removed a duplicated `pnpm run bundle:marketpulse` line from
  `.github/workflows/ci.yml`; the final order is AOT → bundles → bytecode →
  ignored acceptance suite.
- Removed one extra trailing blank line from
  `docs/assets/hmr-zerocopy-binary-oneplus.log` so `git diff --cached --check`
  is clean.

Before committing the merge: `cargo check --workspace` → exit 0.
Merge commit: `29af248 merge(master): reconcile release/v0.4.0 with pushed master`.

## 3. Gate results

Every command below ran in the fresh worktree. Full stdout/stderr is in the
named log under `docs/evidence/release-0.4.0-logs/`.

| # | Command | Exit | Key observed output |
|---|---|---:|---|
| 00 | `git rev-parse HEAD`, tool versions, generated-dir probe | 0 | Commit `29af248`; `node_modules=False`, `target=False` |
| 01 | `pnpm install --frozen-lockfile` | 0 | `Done in 14.4s using pnpm v10.34.9`; Angular `22.3.0`, TypeScript `6.0.3`, `typescript7` alias, Vitest `5.1.2` |
| 02 | `pnpm run build:packages` | 0 | Real `ngc` partial compilation for `@angnative/common` with Ivy metadata verified; all packages built |
| 03 | `pnpm run check:toolchain` | 0 | Toolchain check passed |
| 04–09 | `pnpm run compile:aot` / `:router` / `:forms` / `:i18n` / `:animations` / `:marketpulse` | 0 each | All six AOT targets compiled |
| 10–14 | `pnpm run bundle:quickjs` / `:forms` / `:framework` / `:marketpulse` / `:third-party` | 0 each | All five self-contained bundles generated |
| 15 | `pnpm run bundle:bytecode` | 0 | `[angnative bytecode] 1404074 bytes ... (fingerprint: qjsbc-v2/rquickjs-0.14.0/quickjs-ng-0.16.2/x86_64-windows-le-64)` |
| 17 | `pnpm test` | **1** | First run: `Test Files 1 failed \| 21 passed (22)`, `Tests 1 failed \| 286 passed (287)` — see §4 |
| 19 | `pnpm exec vitest run packages/cli/src/dev.spec.ts --reporter=verbose` | **1** (once), then **0** (rerun) | First targeted run: `HMR event timed out`; rerun `6 passed`, watcher inner loop `16 ms` |
| 22 | Clean `master@dea0beb` probe: same targeted spec | 0 | `6 passed`, watcher inner loop `20 ms` — the failure is not a deterministic master/merge diff |
| 25 | `pnpm test` (full rerun) | 0 | `Test Files 22 passed (22)`, `Tests 287 passed (287)`, `Duration 6.05s` |
| 27 | `cargo test --workspace` | 0 | Sum over 25 per-crate result lines: **192 passed / 0 failed / 19 ignored** |
| 28 | `cargo test -p angnative-runtime-bridge -- --ignored` | 0 | `test result: ok. 19 passed; 0 failed; 0 ignored; 13 filtered out` |
| 29–31 | `pnpm --filter @angnative/web run build`, `pnpm run compile:aot:web`, `pnpm run bundle:aot:web` | 0 each | Web AOT production bundle built |
| 32 | `pnpm check:web-size` | 0 | `raw bundle: 354434 B`; `gzip bundle: 112263 B (budget 128000 B / 125.0 KB, headroom 15737 B)`; `forbidden packages: 0`; `unlinked partial declarations: 0` |
| 33–34 | `pnpm run compile:fixtures`, `pnpm run bundle:fixtures` | 0 each | Fixture bundle generated |
| 35 | `node scripts/check-size-budget.mjs` | 0 | `JS payload: 203 B gzip (budget 61440 B / 60 KB)`; `@angular/compiler and zone.js: 0 B in the bundle`; `GATE EXIT=0` |
| 36 | `python scripts/check-ffi-declarations.py` | 0 | `FFI declaration check OK: 40 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 31 header declarations; iOS unified staticlib wiring OK` |

## 4. Bytecode artifact

Generated from `dist/aot/animations-app-bundle.js` in the fresh tree:

```text
path=dist/aot/animations-app-bundle.qjsbc
size=1404074
sha256=FA53851E9F63515CDD1C10B586FD2ECE373A0E75DE1C961B8FC1CE67A8030154
```

The fingerprint is target-specific (`x86_64-windows-le-64`), so this hash is the
verified artifact for this host only. A different CI target must regenerate it;
no cross-target hash equality is claimed. The ignored bridge suite boots this
fresh artifact successfully (19/19).

## 5. Local-checkout caveats and flake history

1. **CRLF workaround (`16`, `18`, `24`, `26` logs).** On this Windows checkout,
   `pnpm test` needs `scripts/check-size-budget.mjs` and
   `scripts/check-size-budget.spec.ts` temporarily rewritten CRLF → LF. Both
   files were restored with `git checkout --` immediately after each full test
   run; restore logs confirm `hasCRLF=True`. This is a local checkout
   workaround, not a CI step and not a source change.
2. **First full test run failed once (`17`).** The failing case was
   `packages/cli/src/dev.spec.ts > ... watches source files, rebundles and
   publishes a versioned HMR event`, which timed out after 10 s with
   `HMR event timed out`. A targeted rerun failed once the same way (`19`), then
   passed on the next run in 107 ms (`23`). A clean `master@dea0beb` worktree
   passed the same spec on its first attempt (`22`). The merged source files
   `dev.ts`, `dev.spec.ts`, `tsconfig.aot.hmr.json`, `compile-aot.mjs` and
   `bundle-quickjs.mjs` are byte-identical to master. This is recorded as a
   first-run Windows `fs.watch` flake in this worktree, not a claimed
   deterministic regression; the final full suite is green (`25`).
3. The first `pnpm install` reported pnpm's standard ignored-build-script
   warning for `esbuild@0.28.2`. Install still exited 0 and all downstream
   bundles/gates passed.

## 6. Not verified in this evidence

- GitHub Actions itself: this is a local replay of the CI command sequence,
  not a hosted-runner result.
- Node `22.22.x` and the pinned `nightly-2026-11-14` toolchain were not used.
- iOS/macOS build, `swift test`, Simulator and device runs (Ammar's scope).
- Android device runs were **not repeated** in the reconciled tree. The
  emulator/OnePlus results referenced in `docs/UNSUPPORTED-FEATURES.md` remain
  retained historical evidence.
- No startup-speed, signature/authenticity, C ABI/JNI/Swift platform-shell,
  Hermes/V8, SSR, full-DOM-parity or real-time guarantee is claimed here.
- Real-browser save→SSE→DOM `< 1 s` HMR timing remains unverified; only the
  inner-loop compile/bundle numbers are evidenced.
