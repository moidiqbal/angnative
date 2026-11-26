# Evidence — Web adapter AOT migration (JIT-free browser demo)

**Date:** 2026-11-26
**Repo:** `C:\Users\Moid Iqbal\Projects\angnative`
**Base:** local tag `v0.3.0` at `9f909408d29a242fbbe3ef4a8af37fd4747d09be`
**Scope:** priority #1 after the socket-only MarketPulse release. Lifts the
previous "web browser demo still uses JIT" deferral only for the browser demo;
SSR, HMR and native QuickJS parity remain unchanged.

## Claim under test

The `@angnative/web` browser demo now bootstraps from real Angular `ngc` AOT
output. The shipped browser bundle contains no `@angular/compiler` or
`@angular/platform-browser-dynamic` input, and Angular packages that ship
partial declarations are processed by the Angular Linker rather than falling
back to JIT. The same mutation/DOM backend path still renders and handles a
click.

---

## 1. Real ngc AOT compile for the web entry

Command:

```powershell
pnpm compile:aot:web
```

Observed output (exit `0`):

```text
> angnative-monorepo@0.1.0 compile:aot:web C:\Users\Moid Iqbal\Projects\angnative
> node scripts/compile-aot.mjs tsconfig.aot.web.json

[angnative aot] Angular compiler-cli: C:\Users\Moid Iqbal\Projects\angnative\node_modules\.pnpm\@angular+compiler-cli@22.2._0580495e46e9f4bb5e3ba1a71e3a1f1f\node_modules\@angular\compiler-cli\bundles\src\bin\ngc.js
[angnative aot] compiling with tsconfig.aot.web.json...
[angnative aot] AOT compilation complete (Ivy definitions verified).
compile_exit=0
```

## 2. AOT bundle + JIT metafile guard

Command:

```powershell
pnpm bundle:aot:web
```

Observed output (exit `0`; ANSI colour codes omitted):

```text
> angnative-monorepo@0.1.0 bundle:aot:web C:\Users\Moid Iqbal\Projects\angnative
> node scripts/bundle-web-aot.mjs

  dist\web-demo\app.js      345.6kb
  dist\web-demo\app.js.map    3.9mb

⚡ Done in 2120ms
[angnative web] AOT bundle written to C:\Users\Moid Iqbal\Projects\angnative\dist\web-demo\app.js
[angnative web] JIT compiler packages absent from metafile inputs
bundle_exit=0
```

`scripts/bundle-web-aot.mjs` also checks that `dist/aot/web/app.component.js`
and `dist/aot/web/counter.component.js` contain `ɵɵdefineComponent` before
bundling, then fails if the esbuild metafile inputs contain either JIT package.

## 3. Web package test — no `@angular/compiler` import

Command:

```powershell
pnpm --filter @angnative/web test
```

Observed summary (exit `0`):

```text
> @angnative/web@0.1.0 test C:\Users\Moid Iqbal\Projects\angnative\packages\web
> vitest run

 ✓ src/dom-backend.spec.ts (4 tests) 27ms
stdout | src/web.spec.ts > @angnative/web integration > renders an Angular component into the DOM and dispatches a click
Angular is running in development mode.

 ✓ src/web.spec.ts (2 tests) 3282ms

 Test Files  2 passed (2)
      Tests  6 passed (6)
   Start at  23:11:22
   Duration  4.69s (transform 1.53s, setup 0ms, collect 1.44s, tests 3.31s, environment 1ms, prepare 459ms)

package_test_exit=0
```

The integration spec imports the checked-in real ngc fixture
`packages/testing/fixtures/web-aot/compiled/web-aot-fixture.component.js`. It
does not import `@angular/compiler` and does not declare an inline JIT
`@Component`.

### Blocker found and fixed during migration

The first linker-less run failed with:

```text
Error: The injectable 'PlatformLocation' needs to be compiled using the JIT
compiler, but '@angular/compiler' is not available.

The injectable is part of a library that has been partially compiled.
However, the Angular Linker has not processed the library such that JIT
compilation is used as fallback.
```

Cause: Angular packages in `node_modules` ship partial declarations such as
`ɵɵngDeclareFactory`. Linking only the checked-in fixture was not enough; the
Angular package modules in the esbuild/Vitest graph also needed linking.

Fix: added `scripts/angular-linker-plugin.mjs`, which uses
`@angular/compiler-cli/linker/babel` with `@babel/core` resolved from
`node_modules/.pnpm/node_modules`. It exposes:

- `angularLinkerEsbuildPlugin()` for `scripts/bundle-web-aot.mjs`;
- `angularLinkerVitePlugin()` for `vitest.config.ts` and
  `packages/web/vitest.config.ts`.

The plugin only transforms files containing `ɵɵngDeclare`. It does **not**
import `@angular/compiler`. Vitest hook/test timeouts were raised to 60 s
because linker transform work made the first `beforeAll` exceed the 10 s
default; the passing run above took 4.69 s after warm-up.

## 4. Repository-wide tests

Command:

```powershell
pnpm test
```

Observed summary (exit `0`):

```text
 ✓ packages/contract/src/style.spec.ts (9 tests) 18ms
 ✓ packages/contract/src/elements.spec.ts (8 tests) 12ms
 ✓ packages/cli/src/create.spec.ts (8 tests) 22ms
 ✓ packages/cli/src/size-report.spec.ts (6 tests) 10ms
 ✓ packages/contract/src/protocol.spec.ts (6 tests) 10ms
 ✓ scripts/check-size-budget.spec.ts (8 tests) 55ms
 ✓ packages/cli/src/bundle.spec.ts (4 tests) 136ms
 ✓ packages/cli/src/dev.spec.ts (4 tests) 9ms
 ✓ packages/testing/src/mutation-teeth.spec.ts (16 tests) 14ms
 ✓ packages/web/src/dom-backend.spec.ts (4 tests) 40ms
 ✓ packages/testing/src/conformance.spec.ts (133 tests) 79ms
 ✓ packages/core/src/renderer.spec.ts (22 tests) 20ms
 ✓ packages/platform/src/native-angular.spec.ts (6 tests) 19ms
 ✓ packages/web/src/web.spec.ts (2 tests) 3277ms
 ✓ packages/testing/src/integration.spec.ts (5 tests) 11ms
 ✓ packages/cli/src/doctor.spec.ts (10 tests) 4643ms

 Test Files  16 passed (16)
      Tests  251 passed (251)
   Start at  23:11:29
   Duration  6.33s (transform 6.46s, setup 0ms, collect 18.99s, tests 8.38s, environment 5ms, prepare 3.70s)

root_test_exit=0
```

## 5. Existing native CLI AOT build still works

Command:

```powershell
node packages/cli/dist/cli.js build
```

Observed output (exit `0`):

```text
[angnative build] running real Angular AOT compile (scripts/compile-aot.mjs)...
[angnative aot] Angular compiler-cli: C:\Users\Moid Iqbal\Projects\angnative\node_modules\.pnpm\@angular+compiler-cli@22.2._0580495e46e9f4bb5e3ba1a71e3a1f1f\node_modules\@angular\compiler-cli\bundles\src\bin\ngc.js
[angnative aot] compiling with tsconfig.aot.json...
[angnative aot] AOT compilation complete (Ivy definitions verified).
[angnative build] bundling C:\Users\Moid Iqbal\Projects\angnative\dist\aot\main.js -> C:\Users\Moid Iqbal\Projects\angnative\dist\aot\bundle.js
[angnative build] bundle written to C:\Users\Moid Iqbal\Projects\angnative\dist\aot\bundle.js
cli_build_exit=0
```

## 6. Browser smoke test (Playwright/Chromium)

Command (uses the webapp-testing helper to manage the local server):

```powershell
python "C:\Users\Moid Iqbal\.claude\skills\webapp-testing\scripts\with_server.py" --server "node scripts/serve-web.mjs" --port 4173 -- python scripts/verify-web-aot.py
```

Observed output (exit `0`):

```json
{
  "title": "AngNative Web Demo",
  "html_lang": "en",
  "script_fetchpriority": "high",
  "script_type": "module",
  "counter_nodes": 1,
  "na_view_nodes": 3,
  "na_text_nodes": 5,
  "na_button_nodes": 3,
  "plus_button_count": 1,
  "text_before": "AngNative Demo\nAngular running natively \u2014 no WebView.\nCounter\nCount: 0\n\u2212\nreset\n+\neven",
  "text_after": "AngNative Demo\nAngular running natively \u2014 no WebView.\nCounter\nCount: 1\n\u2212\nreset\n+\nodd",
  "requests": [
    "http://localhost:4173/",
    "http://localhost:4173/app.js"
  ],
  "console": [],
  "page_errors": [],
  "screenshot": "docs/assets/web-aot-demo.png"
}
```

Server wrapper output:

```text
Starting server 1/1: node scripts/serve-web.mjs
Waiting for server on port 4173...
Server ready on port 4173

All 1 server(s) ready
Running: python scripts/verify-web-aot.py

Stopping 1 server(s)...
Server 1 stopped
All servers stopped
playwright_exit=0
```

Observed: `Count: 0 → Count: 1` and `even → odd` after clicking the `+`
`na-button`; only `/` and `/app.js` were requested; no console errors and no
page errors. The persisted screenshot is `docs/assets/web-aot-demo.png`.

## 7. Artifact size, hashes and forbidden-string scan

Commands:

```powershell
Get-FileHash -Algorithm SHA256 dist\web-demo\app.js,docs\assets\web-aot-demo.png | Format-Table -AutoSize
Get-Item dist\web-demo\app.js,docs\assets\web-aot-demo.png | Select-Object FullName,Length | Format-Table -AutoSize
$matches = Select-String -Path dist\web-demo\app.js -Pattern '@angular/compiler','platform-browser-dynamic','ɵɵngDeclare','JIT compilation failed' -SimpleMatch
"forbidden_string_matches=$($matches.Count)"
```

Observed:

```text
SHA256    832465B703BDE83C563A5D1DFCE5FA29523AF006D8627C373851D9C42C2B245B  dist\web-demo\app.js
SHA256    F631441729078E8BB0B3E1106E7F13177BFBE551196FB729D45A1AD999E8C0BD  docs\assets\web-aot-demo.png

dist\web-demo\app.js          353854 bytes
docs\assets\web-aot-demo.png   12199 bytes

forbidden_string_matches=0
```

The `ɵɵngDeclare` check is included deliberately: a zero count confirms the
shipped bundle has been linked past Angular's partial-declaration markers.

## 8. Unrelated gate checks after the edits

Command:

```powershell
python scripts/check-ffi-declarations.py
```

Observed output:

```text
FFI declaration check OK: 36 JNI methods across 2 Android shells, 22 Swift bindings (13 runtime bridge), 29 header declarations; iOS unified staticlib wiring OK
```

Commands:

```powershell
git diff --check
git diff --check --cached
```

Observed: no whitespace errors. Six `LF will be replaced by CRLF` warnings were
printed for modified tracked text files (`apps/demo-app/src/main.web.ts`,
`package.json`, `packages/web/README.md`, `packages/web/src/web.spec.ts`,
`packages/web/vitest.config.ts`, `scripts/serve-web.mjs`). None is a diff error.

---

## Result

| Check | Result |
|---|---|
| Real `ngc` output for browser entry | ✅ `tsconfig.aot.web.json`, Ivy definitions verified |
| Browser bundle JIT inputs | ✅ `@angular/compiler` and `@angular/platform-browser-dynamic` absent from metafile; forbidden strings absent from `app.js` |
| Angular partial declarations | ✅ linked by Babel/Linker plugin before bundle/test evaluation |
| Web adapter tests | ✅ 2 files / 6 tests |
| Repository tests | ✅ 16 files / 251 tests |
| Native CLI AOT build | ✅ unchanged and passing |
| Browser render + click | ✅ `Count: 0 → Count: 1`, no errors |
| Browser requests | ✅ only `/` and `/app.js` |
| FFI declaration check | ✅ unchanged |
| `git diff --check` | ✅ no errors (CRLF warnings only) |

## Scope limits

- This lifts only the **browser demo JIT** deferral. It does not add SSR,
  hydration, HMR, a web size-budget gate, Hermes/V8, or native real-time
  easing.
- The production native AOT path and socket-only MarketPulse behavior are
  unchanged by this migration.
- No claim is made about non-`na-*` DOM libraries in the browser demo.
- No push was performed; the migration is intended as a local commit on top of
  `v0.3.0`.
