# Build-Time Tool Validation

**Status: 2026-11-28.** This document describes `scripts/validate-build-tools.mjs`,
a read-only suite that verifies the repository's build-time toolchain facts and
fails loudly when something concrete is broken.

## How to run

```powershell
node scripts/validate-build-tools.mjs
```

No arguments, no installs, no builds. The script only reads files and runs two
harmless commands (`node --version`, `pnpm --version`). It prints a
**PASS / FAIL / INFO** table and exits `1` if any row is `FAIL`, otherwise `0`.

- `PASS` — a concrete fact was verified against a real file or command.
- `FAIL` — a concrete inconsistency (missing required file, duplicate name,
  unresolved entry point, command failure). Causes exit 1.
- `INFO` — an honest finding about optional tooling that is intentionally
  absent. Does **not** fail the run.

## What it checks

1. **Runtime versions** — `node --version` and `pnpm --version` actually run.
2. **Declared versions** (root `package.json`) — `@angular/core`,
   `@angular/compiler-cli`, `typescript`, `vitest`; plus that core and
   compiler-cli versions match. Reports `@angular/cli` as declared or absent.
3. **TypeScript config** — `tsconfig.json`, `tsconfig.base.json`,
   `tsconfig.aot.json`, and a `tsconfig.json` in every workspace package.
4. **Workspace config** — presence of `pnpm-workspace.yaml`, `nx.json`,
   `angular.json` (absent Nx / Angular CLI configs are reported as INFO).
5. **Test tooling** — `vitest.config.*` in packages, Playwright usage in
   `scripts/verify-web.py`, and any Jest config or dependency.
6. **pnpm workspace membership** — parses the `packages:` globs out of
   `pnpm-workspace.yaml` and verifies every directory containing a
   `package.json` under `packages/` and `apps/` is covered.
7. **Unique package names** — across all workspace `package.json` files.
8. **Entry points** — every `main`, `types`, `exports`, and `bin` target of every
   workspace package must exist under `dist/` **or** have a source equivalent
   under `src/` (`.js` / `.d.ts` → `.ts`). Missing targets are FAIL rows; the
   script never throws on a missing file.
9. **CI / root script files** — every `scripts/*.{mjs,js,py,sh,ps1}` referenced
   by `.github/workflows/ci.yml` or by a root `package.json` script must exist;
   every `pnpm run <name>` target in CI must be declared as a root script.

## Current results (actual run, this host)

Environment: **Node v26.13.1**, **pnpm 10.34.9**, Windows/PowerShell.

```
SUMMARY: 24 PASS, 1 FAIL, 7 INFO
RESULT: FAIL        (exit code 1)
```

The single FAIL is pre-existing and unrelated to this build fix:
`packages/native-ui-core` has no `tsconfig.json` (it is excluded from the
TypeScript package build via `build:packages`).

Selected verified rows:

| Check | Result |
|---|---|
| `node --version` / `pnpm --version` | PASS — v26.13.1 / 10.34.9 |
| `@angular/core` / `@angular/compiler-cli` | PASS — both `22.3.0` (aligned) |
| `typescript` / `vitest` | PASS — `6.0.3` (resolved by Angular ngc) + `typescript7` alias `7.0.2` (workspace `tsc`) / `5.1.2` |
| Root + per-package tsconfigs | FAIL — 3 root configs, 7 of 8 packages covered; `packages/native-ui-core` has no `tsconfig.json` (pre-existing) |
| `pnpm-workspace.yaml` | PASS — all 8 packages matched by `[packages/*, apps/*]` |
| Unique package names | PASS — 8 names across 8 packages |
| Vitest configs | PASS — 6 packages (`cli` and `demo-app` delegate to the root runner) |
| Playwright usage | PASS — `scripts/verify-web.py` imports `playwright.sync_api` |
| Entry points | PASS — all 7 publishable packages resolved (on this host via built `dist/`; the `src/` fallback keeps this green on a clean checkout where `dist/` is absent) |
| CI / root-referenced script files | PASS — all 15 exist |
| CI `pnpm run` targets | PASS — all resolve to declared root scripts, including `build:packages` |

## Known gaps (reported as INFO, not failures)

- **Nx is not installed.** There is no `nx.json`, no `project.json`, and no `nx`
  dependency. The workspace is plain pnpm + TypeScript project references
  (`tsconfig.json`) — see `tsconfig.json` and `pnpm-workspace.yaml`.
- **No Angular CLI workspace.** `@angular/cli` is not a dependency and there is
  no `angular.json`. Builds use the deterministic `pnpm run build:packages`
  entrypoint (pnpm topological order; `@angnative/common` via real ngc partial
  compilation).
- **TypeScript 7 + Angular 22.3.0.** `@angular/compiler-cli@22.3.0` pins
  `typescript >=6.0 <6.1` and calls TS APIs removed in 7, so the root
  `typescript` dependency stays at `6.0.3` for ngc. `typescript7` is installed
  as an alias, and `scripts/tsc7.mjs` invokes its declared `tsc` bin explicitly
  for all non-Angular package builds. Both majors are declared; ngc and
  workspace `tsc` resolve different versions by design.
- **Jest is not present.** No `jest.config.*` and no `jest` dependency were
  found. JS/TS tests use Vitest; Rust tests use `cargo test`.
- **Playwright is present but only as a Python tool.** `scripts/verify-web.py`
  imports `playwright.sync_api` (verified importable on this host); it is not a
  Node dependency and has no `playwright.config.*`.
- **`packages/cli` and `apps/demo-app` have no local `vitest.config.ts`**; their
  `test` scripts rely on the root `vitest run`.
- **`apps/showcase` has no `package.json`**, so it is not a pnpm workspace
  member and is not covered by the entry-point check.
- **This validator is not yet wired into CI.** Add a `pnpm run validate:build-tools`
  step to `.github/workflows/ci.yml` if you want it as a gate (that edit is out
  of scope for this change).
- **AOT shadowed binary (fixed).** `npx ngc` in this workspace resolves to a
  shadowing 0.1.0 binary that emits plain JS with no Ivy component definitions.
  `scripts/compile-aot.mjs` now resolves the genuine `@angular/compiler-cli` `ngc`
  via its `package.json` `bin.ngc` field and verifies Ivy output (`defineComponent`)
  after compilation. `node packages/cli/dist/cli.js build` runs this real AOT path
  and bundles `dist/aot/main.js` with esbuild. The CLI's ESM relative imports were
  given `.js` extensions so the compiled binary is runnable.
- **Native shells still have no embedded JS runtime that can execute an Angular
  bundle.** QuickJS scaffolding exists but is not connected; see
  [EVENT-BRIDGE.md](EVENT-BRIDGE.md). This matters for the library matrix in
  [ECOSYSTEM-COMPATIBILITY.md](ECOSYSTEM-COMPATIBILITY.md).
