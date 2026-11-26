# Working notes — Web adapter AOT migration

Status: **complete and verified** (priority #1 after `v0.3.0` local release).
Evidence: `docs/evidence/WEB-AOT-MIGRATION.md`.

## Objective

Remove JIT from the AngNative browser demo. The web demo must consume real
Angular `ngc` AOT output, while keeping the same mutation protocol/DOM backend.
Do not introduce `@angular/compiler` back into the demo bundle.

## Completed edits

- `apps/demo-app/src/main.web.ts`
  - Removed `import '@angular/compiler'`.
  - Entry is now compiled by ngc via `tsconfig.aot.web.json`.
- Added `tsconfig.aot.web.json`
  - ngc output: `dist/aot/web` from `apps/demo-app/src/main.web.ts`.
- Added `tsconfig.aot.web-fixture.json`
  - Regenerates the checked-in AOT fixture used by web tests.
- Added `packages/testing/fixtures/web-aot/web-aot-fixture.component.ts`
  - Real decorator source; compiled output checked in at
    `packages/testing/fixtures/web-aot/compiled/web-aot-fixture.component.js`
    and `.d.ts`.
  - Compiled output has `ɵɵdefineComponent`; tests need no JIT compiler.
- Added `scripts/bundle-web-aot.mjs`
  - Bundles `dist/aot/web/main.web.js` → `dist/web-demo/app.js`.
  - Aliases `@angnative/web` to `packages/web/dist/index.js` because the
    workspace link lives under `apps/demo-app/node_modules`.
  - Fails if Ivy definitions are missing or if metafile inputs contain
    `@angular/compiler` / `@angular/platform-browser-dynamic`.
- Updated `scripts/serve-web.mjs`
  - No longer esbuild-bundles JIT source.
  - Serves the AOT bundle and uses
    `<script type="module" src="/app.js" fetchpriority="high">`.
- Updated root `package.json`
  - Added `compile:aot:web`, `compile:aot:web-fixture`, `bundle:aot:web`.
  - `pnpm start` now builds the web package, compiles AOT, bundles, serves.
- Updated `packages/web/README.md`.
- Updated `packages/web/src/web.spec.ts`
  - Removed JIT `await import('@angular/compiler')` and inline `@Component`.
  - Uses the checked-in real ngc fixture `WebAotFixtureComponent`.

## Verified commands/outputs so far

```text
pnpm compile:aot:web
[angnative aot] Angular compiler-cli: ...@angular\compiler-cli\bundles\src\bin\ngc.js
[angnative aot] compiling with tsconfig.aot.web.json...
[angnative aot] AOT compilation complete (Ivy definitions verified).
```

```text
pnpm bundle:aot:web
dist\web-demo\app.js      345.6kb
dist\web-demo\app.js.map    3.9mb
[angnative web] AOT bundle written to ...\dist\web-demo\app.js
[angnative web] JIT compiler packages absent from metafile inputs
```

## Original blocker (resolved)

```text
pnpm --filter @angnative/web test
Error: The injectable 'PlatformLocation' needs to be compiled using the JIT
compiler, but '@angular/compiler' is not available.

The injectable is part of a library that has been partially compiled.
However, the Angular Linker has not processed the library such that JIT
compilation is used as fallback.
```

Test result: 1 failed suite, 4 passed tests, 2 skipped.

Cause: Angular fesm2022 packages under `node_modules` ship partial compilation
declarations (`ɵɵngDeclareFactory`). The AOT fixture itself is linked, but
`@angular/common`/`platform-browser` are not processed by the Angular Linker in
this esbuild/vitest setup. The old spec masked this by importing
`@angular/compiler`.

Do **not** fix this by re-adding `@angular/compiler` to the demo/test.

## Resolution

Added `scripts/angular-linker-plugin.mjs`, using
`@angular/compiler-cli/linker/babel` with `@babel/core` resolved from
`node_modules/.pnpm/node_modules`. It exposes an esbuild `onLoad` plugin for
`scripts/bundle-web-aot.mjs` and a Vite/Vitest `transform` plugin for both
Vitest configs. Only files containing `ɵɵngDeclare` are transformed; no
`@angular/compiler` import was reintroduced. Vitest hook/test timeouts were
raised to 60 s to cover linker transform work.

Rerun and recorded in `docs/evidence/WEB-AOT-MIGRATION.md`:

- `pnpm compile:aot:web`
- `pnpm bundle:aot:web`
- `pnpm --filter @angnative/web test` → 2 files / 6 tests
- `pnpm test` → 16 files / 251 tests
- `node packages/cli/dist/cli.js build`
- Playwright AOT demo (`Count: 0 → Count: 1`, no console/page errors)
- `python scripts/check-ffi-declarations.py`
- `git diff --check`

Commit locally only. No push.

## Guide disclosure to include in final delivery

Modern web guidance was consulted before changing browser code. Applied guides:

- `optimize-script-priority`: “Elevate critical async scripts: For critical
  scripts loaded with `async` or `defer`, add the `fetchpriority="high"`
  attribute to ensure they are prioritized during the discovery phase.”
- `performance`: “`type="module"` is preferred for modern JavaScript and is
  deferred by default so no need to have an explicit `defer` attribute”.
