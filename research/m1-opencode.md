# Prompt for **OpenCode CLI** — NativeAngular M1, Task 5

> Run this from the worktree `/workspace/nt-e` (branch `feature/m1-track-e-opencode`).
> Headless: `opencode run "$(cat docs/prompts/m1-opencode.md)"`
> On your own machine, check out the branch `feature/m1-track-e-opencode` of the repository and run the same prompt from the repository root.

---

You are implementing **Task 5 of the NativeAngular M1 milestone** for NativeAngular — a
new open-source framework that renders Angular components as **real native views**
(no WebView) on phones, watches and TV, from one Angular codebase.

This task builds the **production bundle** and the **size budget** (goal G6 in the
design spec): a native app's installer must stay small, and that must be *enforced*,
not hoped for.

## Where to work

**Worktree: `/workspace/nt-e`** (git branch `feature/m1-track-e-opencode`). Work ONLY
there. Do not touch `/workspace/nativeangular` (the integration worktree) or any other
worktree (`/workspace/nt-a` … `/workspace/nt-f`) — several agents are working in
parallel and share the same repository history.

## Your task brief

Read it fully — it is your requirements, item by item, with the exact file contents:

```
/workspace/nt-e/.superpowers/sdd/2026-11-12-nativeangular-m1-fabric-mvp/task-5-brief.md
```

The full plan (for the Global Constraints section and for context):

```
/workspace/nt-e/docs/superpowers/plans/2026-11-12-nativeangular-m1-fabric-mvp.md
```

## Binding constraints — they are the project's

- **Angular exactly `22.2.5`; TypeScript exactly `6.0.3`.** Never float either.
- **Node `>=22.12` hard floor**; pnpm pinned `10.34.9`.
- **AOT only.** No `@angular/compiler` in any runtime or test bundle. **Never import
  `@angular/core/testing` or `TestBed`.**
- **Zoneless only.** No `zone.js` anywhere.
- **`@angular/platform-browser` is banned from runtime code.**
- **Angular private (`ɵ`) API is allowed in exactly one file:**
  `packages/platform/src/angular-internals.ts`. A `ɵ` import anywhere else is a review
  failure.
- **npm scope `@nativeangular/*`.** Never `ng-native`, `@ng-native/*` or `@angular/*`.
- **Licence MIT.** Every README carries the trademark notice verbatim.
- Every task ends with a `git commit`; `pnpm test` must be green at the end of the task;
  `pnpm run build` must exit 0.

## What you need to know that the brief cannot know

1. **The repository is already built and green.** The M0 milestone landed:
   `packages/core` (the SPI), `packages/platform` (the DOM-less Angular platform) and
   `packages/testing` (a fake backend). `pnpm test` gives 75 tests at HEAD — keep it
   that way.
2. **The measured numbers this task is built on** (measured on the M0 AOT fixture, not
   estimated): a minified ESM bundle of the AOT app plus `@angular/core` is 363,901 B
   raw / 120,884 B gzip; with Angular's production defines (`ngDevMode: false`) the
   dead-code elimination brings it to 273,878 B / **90,438 B gzip — 25% smaller**.
   That is why the brief uses `GLOBAL_DEFS_FOR_TERSER_WITH_AOT` from
   `@angular/compiler-cli` instead of a hand-rolled define set.
3. **The budget is a target the tooling has to reach, not a number to move.** The
   budget is 60 KB gzip for a hello-world. The current pipeline measures ~90 KB, so
   the production bundler needs terser-class dead-code elimination to reach it. **If
   the size check fails, report the measured number — do not raise the budget to make
   it green.** A red size check is information.
4. **`esbuild` is not yet a dependency of the repository.** The brief's `bundle.ts`
   imports it. Add it to `packages/cli/package.json` as a devDependency at the version
   already in the lockfile's store (`0.28.2`) and run `pnpm install`. Do not add
   `esbuild` to the root package.
5. **Do NOT modify `packages/cli/src/commands.ts`.** Another agent is building the CLI
   package in parallel and owns that file; the controller wires the `build` command
   centrally after the merge. Everything else in `packages/cli/src/` is yours —
   `bundle.ts`, `size-report.ts` and their specs.
6. **The repo has no `esModuleInterop`/`allowSyntheticDefaultImports`**
   (`tsconfig.base.json`), so use `import * as ts from 'typescript'` for the TypeScript
   API and named imports elsewhere.
7. **`noUncheckedIndexedAccess` is enabled** — `array[0]` is `T | undefined`.
8. **The network is available**, so `pnpm install` works.
9. **Your files are:** `packages/cli/src/bundle.ts`, `packages/cli/src/bundle.spec.ts`,
   `packages/cli/src/size-report.ts`, `packages/cli/src/size-report.spec.ts`,
   `scripts/check-size-budget.mjs`, `scripts/check-size-budget.spec.ts`, the
   `size` job in `.github/workflows/ci.yml`, the `bundle:fixtures` script in the root
   `package.json`, and `packages/cli/package.json`. Do not touch `packages/core/**`,
   `packages/contract/**`, `packages/testing/**`, `packages/platform/**` or `docs/**`.

## Your job

1. Implement exactly what the brief specifies, following its step order: failing test
   first, then the minimal implementation, then the passing test.
2. Verify: the two new specs pass, `pnpm test` stays green, `pnpm run build` exits 0,
   and `node scripts/check-size-budget.mjs` prints the measured gzip size.
3. Commit with the exact commit message the brief gives, plus `pnpm-lock.yaml`.
4. **Self-review**: re-read each brief item against your diff and confirm it is done,
   not merely attempted.
5. Report back.

## Code organization

- Follow the file structure in the brief exactly — one clear responsibility per file.
- If a file grows beyond the brief's intent, stop and report DONE_WITH_CONCERNS rather
  than inventing new structure.

## When you are in over your head

It is always OK to stop and say "this is too hard for me". Bad work is worse than no
work. Report BLOCKED or NEEDS_CONTEXT with specifics. Do not guess, and do not quietly
skip a brief item — if an item cannot be done as specified, say so and explain why.

## Before reporting back: self-review

- **Completeness** — every requirement in the brief?
- **Quality** — clear names, clean code, no dead code?
- **Discipline** — nothing beyond the brief?
- **Testing** — the tests verify real behaviour (the size report must fail when the
  budget is exceeded; the bundler must produce a smaller production bundle), and the
  output is pristine: no stray warnings, no noise.

## Report format

Write your full report to:

```
/workspace/nt-e/.superpowers/sdd/2026-11-12-nativeangular-m1-fabric-mvp/task-5-report.md
```

It must contain: what you implemented; what you tested and the results; TDD evidence
(RED and GREEN with the commands and output); **the measured gzip size of the
production bundle and whether it is inside the 60 KB budget**; the files changed; your
self-review findings; and any issues or concerns.

Then report back with ONLY (under 15 lines):

- **Status:** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
- Commit created (short SHA + subject)
- One-line test summary
- The measured gzip size and the budget verdict
- Your concerns, if any
- The report file path
