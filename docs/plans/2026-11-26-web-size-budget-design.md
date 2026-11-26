# Design — Web demo size-budget gate

**Date:** 2026-11-26
**Priority:** #2 after the local `v0.3.0` release and the web AOT migration.
**Status:** validated with the user; not yet implemented.

## Objective

Add an enforceable size/JIT regression gate for the AOT browser demo. The gate
must protect the JIT-free web bundle produced by the priority #1 migration
without changing the fixture JS gate, Rust gate, or native runtime pipeline.

## Decisions

- Budget: dedicated `webDemoGzipBytes = 128000` in
  `packages/cli/src/budgets.json`. The current AOT bundle measures 113,224 B
  gzip (353,854 B raw), leaving ~13% headroom. Reusing the 250 KB typical-app
  budget would be too loose; the 60 KB hello-world budget would fail.
- Enforcement: standalone `scripts/check-web-size-budget.mjs`, exposed as
  `pnpm check:web-size`, plus a step in the existing CI `size` job.
- Checks: gzip size, artifact presence, forbidden packages from
  `budgets.json`, and absence of `ɵɵngDeclare` markers in the shipped bundle.
- HTML attributes (`type="module"`, `fetchpriority="high"`) stay in the existing
  Playwright smoke test; the size gate does not duplicate browser checks.

## Architecture

Inputs are artifacts produced by:

```powershell
pnpm compile:aot:web
pnpm bundle:aot:web
```

Specifically:

- `dist/web-demo/app.js`
- `dist/web-demo/app.meta.json`

The script loads `packages/cli/src/budgets.json`, requires both artifacts,
gzips `app.js` with `zlib.createGzip()`, and compares against
`webDemoGzipBytes`. It reuses the forbidden-package list
(`@angular/compiler`, `zone.js`) against metafile inputs and scans the shipped
bundle text for `ɵɵngDeclare`. A marker means Angular partial declarations
reached the browser unlinked, so the build must fail.

Report shape:

```text
raw bytes, gzip bytes, budget, headroom,
forbidden matches, partial-declaration matches,
WEB GATE EXIT=0|1
```

CI's `size` job will compile and bundle the web demo, then run
`pnpm check:web-size`. The fixture JS gate and Rust core gate remain unchanged.

## Error handling

Core logic is a pure exported function:

```ts
runWebSizeGate(options): {
  ok: boolean;
  rawBytes: number;
  gzipBytes: number;
  budgetBytes: number;
  forbiddenFound: string[];
  partialDeclarationsFound: string[];
  errors: Array<{ code: string; message: string; detail?: string }>;
}
```

The CLI wrapper prints the report and sets the process exit code. `process.exit()`
is not called inside the core function.

Failure codes:

- `MISSING_ARTIFACT` — print `Run: pnpm compile:aot:web && pnpm bundle:aot:web`
- `BUDGET_EXCEEDED` — measured gzip, budget, overage bytes
- `FORBIDDEN_PACKAGE` — exact metafile input paths
- `UNLINKED_PARTIAL_DECLARATION` — Angular Linker did not process partial declarations

## Tests

New `scripts/check-web-size-budget.spec.ts` with temporary fixture files:

1. `budgets.webDemoGzipBytes === 128000` and forbidden list intact.
2. Tiny bundle + metafile → gate passes.
3. Oversized gzip content → `BUDGET_EXCEEDED`.
4. `@angular/compiler` metafile input → `FORBIDDEN_PACKAGE`.
5. `ɵɵngDeclare` in bundle → `UNLINKED_PARTIAL_DECLARATION`.
6. Missing artifact → `MISSING_ARTIFACT` with regeneration hint.

## Evidence-first delivery

After implementation, record exact commands and observed outputs in
`docs/evidence/WEB-SIZE-BUDGET.md`, including:

- `pnpm check:web-size` on the real AOT bundle
- `pnpm test` summary
- CLI diff in `.github/workflows/ci.yml`
- deliberate negative-case outputs from the spec run

Only after that evidence, update `STATUS.md`,
`docs/UNSUPPORTED-FEATURES.md`, and
`docs/architecture/issue-register.md`. Commit locally; do not push.

## Scope limits

- No budget increase for the existing fixture or Rust gates.
- No SSR, HMR, DOM-library parity, or native runtime changes.
- No claim that gzip size guarantees startup latency or frame rate.
