# Evidence — Web demo size-budget gate

**Date:** 2026-11-27
**Repo:** `C:\Users\Moid Iqbal\Projects\angnative`
**Base:** `c919945` (web AOT migration) + `6003a2f` (validated design doc)
**Priority:** #2 after the socket-only MarketPulse release.

## Claim under test

The AOT browser demo bundle is now protected by a dedicated gate:
`pnpm check:web-size` fails if `dist/web-demo/app.js` exceeds 128,000 B gzip,
if its metafile/bundle is missing, if a forbidden package
(`@angular/compiler`, `zone.js`) appears in metafile inputs, or if unlinked
Angular `ɵɵngDeclare*` markers reach the shipped bundle.

---

## 1. Budget source and CI wiring

`packages/cli/src/budgets.json` gained:

```json
"webDemoGzipBytes": 128000
```

`package.json` gained:

```json
"check:web-size": "node scripts/check-web-size-budget.mjs"
```

`.github/workflows/ci.yml` `size` job gained steps to build the web adapter,
compile/bundle the AOT web demo, and run `pnpm check:web-size` before the
existing fixture JS gate. The fixture gate and Rust gate are unchanged.

YAML parse check:

```powershell
python -c "import yaml, pathlib; d=yaml.safe_load(pathlib.Path('.github/workflows/ci.yml').read_text(encoding='utf-8')); print('yaml_ok', 'jobs=' + ','.join(d['jobs'].keys()))"
```

Observed:

```text
yaml_ok jobs=test,rust,angular-runtime,android-e2e,size
```

No GitHub Actions run was performed locally; only the parsed job list is
claimed here.

## 2. AOT artifact regeneration

Command:

```powershell
pnpm compile:aot:web
pnpm bundle:aot:web
```

Observed output (exit `0` for both; ANSI colour codes omitted):

```text
> angnative-monorepo@0.1.0 compile:aot:web C:\Users\Moid Iqbal\Projects\angnative
> node scripts/compile-aot.mjs tsconfig.aot.web.json

[angnative aot] Angular compiler-cli: C:\Users\Moid Iqbal\Projects\angnative\node_modules\.pnpm\@angular+compiler-cli@22.2._0580495e46e9f4bb5e3ba1a71e3a1f1f\node_modules\@angular\compiler-cli\bundles\src\bin\ngc.js
[angnative aot] compiling with tsconfig.aot.web.json...
[angnative aot] AOT compilation complete (Ivy definitions verified).
compile_exit=0

> angnative-monorepo@0.1.0 bundle:aot:web C:\Users\Moid Iqbal\Projects\angnative
> node scripts/bundle-web-aot.mjs

  dist\web-demo\app.js      345.6kb
  dist\web-demo\app.js.map    3.9mb

⚡ Done in 825ms
[angnative web] AOT bundle written to C:\Users\Moid Iqbal\Projects\angnative\dist\web-demo\app.js
[angnative web] JIT compiler packages absent from metafile inputs
bundle_exit=0
```

Bundle hash is unchanged from the priority #1 migration:

```powershell
Get-FileHash -Algorithm SHA256 dist\web-demo\app.js | Format-Table -AutoSize
```

```text
SHA256    832465B703BDE83C563A5D1DFCE5FA29523AF006D8627C373851D9C42C2B245B  dist\web-demo\app.js
```

## 3. Real gate run

Command:

```powershell
pnpm check:web-size
```

Observed output (exit `0`):

```text
> angnative-monorepo@0.1.0 check:web-size C:\Users\Moid Iqbal\Projects\angnative
> node scripts/check-web-size-budget.mjs

[angnative web] raw bundle: 353854 B
[angnative web] gzip bundle: 111977 B (budget 128000 B / 125.0 KB, headroom 16023 B)
[angnative web] forbidden packages: 0
[angnative web] unlinked partial declarations: 0
WEB GATE EXIT=0
gate_exit=0
```

The 111,977 B gzip value is Node `zlib.createGzip()` output, the same algorithm
used by the gate. An earlier exploratory measurement with .NET
`GZipStream(Optimal)` reported 113,224 B; both are below the 128,000 B budget.
The authoritative gate value is the one printed above.

## 4. Gate unit/negative tests

Command:

```powershell
pnpm exec vitest run scripts/check-web-size-budget.spec.ts --reporter=verbose
```

Observed output (exit `0`):

```text
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — configuration > loads the dedicated 128 KB gzip budget and forbidden packages 1ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — helpers > gzipSize returns a positive count smaller than repetitive raw text 3ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — helpers > findForbidden detects banned metafile inputs 0ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — helpers > findPartialDeclarations detects unlinked Angular output 0ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — end-to-end gate > passes for a JIT-free bundle inside the budget 2ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — end-to-end gate > fails with BUDGET_EXCEEDED when gzip is above budget 1ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — end-to-end gate > fails with FORBIDDEN_PACKAGE for @angular/compiler input 2ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — end-to-end gate > fails with UNLINKED_PARTIAL_DECLARATION for ɵɵngDeclare output 2ms
 ✓ scripts/check-web-size-budget.spec.ts > web size budget — end-to-end gate > fails with MISSING_ARTIFACT and a regeneration hint 0ms

 Test Files  1 passed (1)
      Tests  9 passed (9)
   Start at  00:05:28
   Duration  644ms (transform 47ms, setup 0ms, collect 76ms, tests 15ms, environment 0ms, prepare 78ms)

spec_verbose_exit=0
```

### Blocker found and fixed

The first run of the new spec failed before collecting tests:

```text
SyntaxError: scripts/check-web-size-budget.spec.ts: Unexpected token, expected "," (19:24)
function metafile(inputs: Record<string, { bytes: number }>) {
                          ^
Plugin: angnative-angular-linker
```

Cause: `angularLinkerVitePlugin()` transformed any file containing the literal
`ɵɵngDeclare` marker. The new spec contains that marker as test data, so Babel's
JS parser tried to parse TypeScript type annotations.

Fix: `scripts/angular-linker-plugin.mjs` now ignores non-JS ids
(`/\.[cm]?js$/` after stripping the query string). Angular package modules and
the checked-in compiled fixture are still linked; TypeScript specs are not.
After the fix, the verbose run above passed 9/9.

## 5. Web adapter tests after the plugin fix

Command:

```powershell
pnpm --filter @angnative/web test
```

Observed summary (exit `0`):

```text
 ✓ src/dom-backend.spec.ts (4 tests) 21ms
stdout | src/web.spec.ts > @angnative/web integration > renders an Angular component into the DOM and dispatches a click
Angular is running in development mode.

 ✓ src/web.spec.ts (2 tests) 5736ms

 Test Files  2 passed (2)
      Tests  6 passed (6)
   Start at  00:05:02
   Duration  7.84s (transform 1.15s, setup 0ms, collect 3.37s, tests 5.76s, environment 0ms, prepare 166ms)

webtest_exit=0
```

## 6. Repository-wide tests

Command:

```powershell
pnpm test
```

Observed summary (exit `0`):

```text
 ✓ scripts/check-web-size-budget.spec.ts (9 tests) 42ms
 ✓ packages/cli/src/create.spec.ts (8 tests) 6ms
 ✓ scripts/check-size-budget.spec.ts (8 tests) 15ms
 ✓ packages/cli/src/size-report.spec.ts (6 tests) 5ms
 ✓ packages/contract/src/protocol.spec.ts (6 tests) 6ms
 ✓ packages/contract/src/style.spec.ts (9 tests) 3ms
 ✓ packages/contract/src/elements.spec.ts (8 tests) 3ms
 ✓ packages/cli/src/bundle.spec.ts (4 tests) 83ms
 ✓ packages/cli/src/dev.spec.ts (4 tests) 3ms
 ✓ packages/testing/src/mutation-teeth.spec.ts (16 tests) 4ms
 ✓ packages/testing/src/conformance.spec.ts (133 tests) 61ms
 ✓ packages/web/src/dom-backend.spec.ts (4 tests) 11ms
 ✓ packages/core/src/renderer.spec.ts (22 tests) 14ms
 ✓ packages/platform/src/native-angular.spec.ts (6 tests) 15ms
 ✓ packages/web/src/web.spec.ts (2 tests) 1861ms
 ✓ packages/testing/src/integration.spec.ts (5 tests) 6ms
 ✓ packages/cli/src/doctor.spec.ts (10 tests) 4756ms

 Test Files  17 passed (17)
      Tests  260 passed (260)
   Start at  00:05:11
   Duration  5.69s (transform 3.20s, setup 0ms, collect 10.50s, tests 6.89s, environment 2ms, prepare 2.32s)

roottest_exit=0
```

## 7. Whitespace check and unrelated working-tree changes

Command:

```powershell
git diff --check
git diff --check --cached
```

Observed: no whitespace errors. LF→CRLF warnings were printed for modified
text files.

While preparing the commit, two unrelated modified files were observed in the
working tree:

```text
 M crates/angnative-host-layout/Cargo.toml
 M crates/angnative-runtime-bridge/Cargo.toml
```

Their diffs replace workspace dependencies with an incomplete dependency list
and are **not part of this gate**. No source in this round writes those files,
their origin was not established, and they were left untouched and excluded
from the gate commit. No Rust/Cargo gate was rerun for this web-only change.

---

## Result

| Check | Result |
|---|---|
| Dedicated budget key | ✅ `webDemoGzipBytes = 128000` |
| Real AOT bundle gate | ✅ 111,977 B gzip, 16,023 B headroom, exit 0 |
| Forbidden package detection | ✅ 0 in real bundle; negative test passes |
| Unlinked partial declaration detection | ✅ 0 in real bundle; negative test passes |
| Gate spec | ✅ 9 tests |
| Web package tests | ✅ 2 files / 6 tests |
| Root tests | ✅ 17 files / 260 tests |
| CI YAML parse | ✅ 5 jobs retained, `size` job extended |
| Bundle hash | ✅ unchanged `832465B7…B245B` |

## Scope limits

- No GitHub Actions run was executed; CI wiring is parse-checked only.
- No Rust gate or Cargo build was rerun for this web-only change.
- Gzip size is a payload guard; it does not prove startup latency, frame rate,
  or real-time behavior.
- The two unrelated `Cargo.toml` modifications remain unverified user/worktree
  changes and are excluded.
- No push was performed.
