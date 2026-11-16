# Prompt for **Codex CLI** — NativeAngular M1, Task 8

> Run this from the worktree `/workspace/nt-d` (branch `feature/m1-track-d-codex`).
> Headless: `codex exec "$(cat docs/prompts/m1-codex.md)"`
> On your own machine, check out the branch `feature/m1-track-d-codex` of the repository and run the same prompt from the repository root.

---

You are implementing **Task 8 of the NativeAngular M1 milestone** for NativeAngular — a
new open-source framework that renders Angular components as **real native views**
(no WebView) on phones, watches and TV, from one Angular codebase.

This task builds `@nativeangular/router`: the bridge that makes Angular's router drive
the **native navigation stack** instead of a browser history — push/pop, the hardware
back button, Apple's interactive swipe-back and deep links.

## Where to work

**Worktree: `/workspace/nt-d`** (git branch `feature/m1-track-d-codex`). Work ONLY
there. Do not touch `/workspace/nativeangular` (the integration worktree) or any other
worktree (`/workspace/nt-a` … `/workspace/nt-f`) — several agents are working in
parallel and share the same repository history.

## Your task brief

Read it fully — it is your requirements, item by item, with the exact file contents:

```
/workspace/nt-d/.superpowers/sdd/2026-11-12-nativeangular-m1-fabric-mvp/task-8-brief.md
```

The full plan (for the Global Constraints section and for context):

```
/workspace/nt-d/docs/superpowers/plans/2026-11-12-nativeangular-m1-fabric-mvp.md
```

## Binding constraints — they are the project's

- **Angular exactly `22.2.5`; TypeScript exactly `6.0.3`.** Never float either.
- **Node `>=22.12` hard floor**; pnpm pinned `10.34.9`.
- **AOT only.** No `@angular/compiler` in any runtime or test bundle. **Never import
  `@angular/core/testing` or `TestBed`.**
- **Zoneless only.** No `zone.js` anywhere.
- **`@angular/platform-browser` is banned from runtime code.**
- **Angular private (`ɵ`) API is allowed in exactly one file:**
  `packages/platform/src/angular-internals.ts`. A `ɵ` import anywhere else — including
  in this task — is a review failure.
- **npm scope `@nativeangular/*`.** Never `ng-native`, `@ng-native/*` or `@angular/*`.
- **Licence MIT.** Every README carries the trademark notice verbatim:
  *"NativeAngular is an independent community project, not affiliated with or endorsed by
  Google LLC, the Angular team, Expo or Meta. 'Angular' is a trademark of Google LLC and
  is used here only to describe compatibility."*
- Every task ends with a `git commit`; `pnpm test` must be green at the end of the task;
  `pnpm run build` must exit 0.

## What you need to know that the brief cannot know

1. **The repository is already built and green.** The M0 milestone landed:
   `packages/core` (the SPI), `packages/platform` (the DOM-less Angular platform) and
   `packages/testing` (a fake backend). `pnpm test` gives 75 tests at HEAD — keep it
   that way.
2. **The network is available**, so the dependency step in the brief works:
   `pnpm --filter @nativeangular/router add @angular/router@22.2.5 @angular/common@22.2.5`.
   Pin the versions exactly — floating them is a review failure.
3. **Verify the `LocationStrategy` import path against the installed package** before
   you write the code. The brief imports it from `@angular/common`; confirm that in
   `node_modules/@angular/common` before relying on it, and check the abstract members
   `LocationStrategy` actually declares (`path`, `onPopState`, `getBaseHref`,
   `prepareExternalUrl`) against the installed `.d.ts`. If the installed API differs
   from the brief, adapt the code to the installed package — the brief's *contract*
   (the strategy drives the native stack; the router uses public API only) does not
   change.
4. **The repo has no `esModuleInterop`/`allowSyntheticDefaultImports`**
   (`tsconfig.base.json`), so use `import * as ts from 'typescript'` for the TypeScript
   API and named imports elsewhere.
5. **`noUncheckedIndexedAccess` is enabled** — `array[0]` is `T | undefined`.
6. **Your files are `packages/router/**` plus one line in the root `tsconfig.json`**
   (the project reference). Do not touch `packages/core/**`, `packages/contract/**`,
   `packages/cli/**`, `packages/testing/**`, `packages/platform/**` or `docs/**` — other
   agents own those in parallel. If the root `tsconfig.json` edit conflicts when you
   merge, the controller resolves it.
7. **Do not create a package that already exists** and do not add packages to the
   workspace that the brief does not name.

## Your job

1. Implement exactly what the brief specifies, following its step order: failing test
   first, then the minimal implementation, then the passing test.
2. Verify: `pnpm vitest run packages/router/src/router.spec.ts` must pass, `pnpm test`
   must stay green, and `pnpm run build` must exit 0.
3. Commit with the exact commit message the brief gives, plus the root `tsconfig.json`
   reference and `pnpm-lock.yaml`.
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
- **Testing** — the tests verify real behaviour (the strategy must actually produce the
  native navigation commands), and the output is pristine: no stray warnings, no noise.

## Report format

Write your full report to:

```
/workspace/nt-d/.superpowers/sdd/2026-11-12-nativeangular-m1-fabric-mvp/task-8-report.md
```

It must contain: what you implemented; what you tested and the results; TDD evidence
(RED — the command and the relevant failing output before the implementation, and why
that failure was expected; GREEN — the command and the passing output after); the exact
`LocationStrategy` API you verified in the installed package and how your code matches
it; the files changed; your self-review findings; and any issues or concerns.

Then report back with ONLY (under 15 lines):

- **Status:** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
- Commit created (short SHA + subject)
- One-line test summary
- Your concerns, if any
- The report file path
