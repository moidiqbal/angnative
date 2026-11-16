# Prompt for **OpenCode CLI** — AngNative Phase 1 (bundle/size/dev server)

> Run from the repo root `angnative/`.
> Headless: `opencode run "$(cat doc/angnative-opencode.md)"`
> Ensure Node >= 22.12 and pnpm 10.34.9 are installed.

---

You are implementing selected tasks of the **AngNative** Rust-first build plan
(`research/TaskList.txt`), Phase 1. AngNative renders Angular components as real
native views (no WebView) on phones, watches, TV and desktop from one Angular
codebase, with a **Rust host** as the only native backend.

## Canonical identity (do not violate)
- Product: **AngNative**; npm scope `@angnative/*`; element prefix `na-`;
  licence **MIT** (see `docs/identity.md`).
- Angular exactly `22.2.5`; TypeScript exactly `6.0.3`; Node `>= 22.12`;
  pnpm `10.34.9`.
- AOT only — no `@angular/compiler` in any runtime/test bundle.
- Zoneless only — zero `zone.js`.
- Angular private (`ɵ`) API allowed in exactly one file:
  `packages/platform/src/angular-internals.ts`.
- Unknown property writes must throw.
- Size budget: <= 60 KB gzip hello-world, <= 250 KB typical app, 0 B
  `@angular/compiler` and `zone.js`. Rust core <= 1 MB per platform (measured).

## Your assigned tasks

### T7 — Size budget CI
- `packages/cli/src/bundle.ts`, `size-report.ts` + specs; `scripts/check-size-budget.mjs`
  + spec; the `size` job in `.github/workflows/ci.yml`; `bundle:fixtures` script
  in root `package.json`; esbuild `0.28.2` as devDependency in
  `packages/cli/package.json` only.
- Terser-class DCE to reach <= 60 KB gzip (measured baseline today: 90,438 B
  gzip prod, `@angular/core` = 96.3% of payload).
- Gate blocks when budget exceeded, metafile missing, or bundle contains
  `@angular/compiler` / `zone.js`. Single source of truth:
  `packages/cli/src/budgets.json`.
- Do NOT modify `packages/cli/src/commands.ts` if another track owns it; otherwise
  wire the `build` command.

### T8 — Dev server + incremental AOT
- `nativeangular dev` (or `angnative dev`): incremental AOT, <= 200 ms per
  changed component, <= 500 ms inner loop end to end. Reuse the existing
  esbuild + oxc-transform pipeline — do NOT build a second bundler. Dev bundles
  stay out of the production size gate.
- Fast refresh: component edits keep state; framework/package edits restart.

### D2 — `angnative doctor`
- Checks SDKs, Node, Rust, Android SDK, Xcode, NDK. Correct exit 0/1 on
  Windows AND Linux (the M0 review found the Windows path guard never matches).

## Where to work
- `packages/cli/**`, `scripts/**`, `.github/workflows/ci.yml`, root
  `package.json` (scripts only).
- Do NOT touch `packages/core/**`, `packages/contract/**`,
  `packages/testing/**`, `packages/platform/**`, `crates/**`.

## Verification (run before committing)
1. `pnpm run check:toolchain`
2. `pnpm run compile:fixtures`
3. `pnpm run build` (tsc -b exit 0)
4. `pnpm test` (all green)
5. `node scripts/check-size-budget.mjs` (prints measured gzip size, exits 0)
6. mutation check on the size gate.

## Report
Write your report to `doc/angnative-opencode-report.md` with: what you implemented,
TDD evidence (RED/GREEN with commands + output), measured gzip size + budget
verdict, files changed, self-review, concerns. Then report back under 15 lines.
