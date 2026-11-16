# Prompt for **Kilo Code CLI** — NativeAngular M1, Task 11

> Run this from the worktree `/workspace/nt-f` (branch `feature/m1-track-f-kilo`).
> Headless: `kilo run "$(cat docs/prompts/m1-kilo.md)"`
> On your own machine, check out the branch `feature/m1-track-f-kilo` of the repository and run the same prompt from the repository root.

---

You are implementing **Task 11 of the NativeAngular M1 milestone** for NativeAngular — a
new open-source framework that renders Angular components as **real native views**
(no WebView) on phones, watches and TV, from one Angular codebase.

This task builds the **native tree inspector**: it renders the native element tree of a
real `ngc`-compiled Angular app as a **single self-contained HTML page**. It is the
project's visible proof — every other task produces something a reviewer must *read*;
this one produces something a person can *see*.

## Where to work

**Worktree: `/workspace/nt-f`** (git branch `feature/m1-track-f-kilo`). Work ONLY
there. Do not touch `/workspace/nativeangular` (the integration worktree) or any other
worktree (`/workspace/nt-a` … `/workspace/nt-f`) — several agents are working in
parallel and share the same repository history.

## Your task brief

Read it fully — it is your requirements, item by item, with the exact file contents:

```
/workspace/nt-f/.superpowers/sdd/2026-11-12-nativeangular-m1-fabric-mvp/task-11-brief.md
```

The full plan (for the Global Constraints section and for context):

```
/workspace/nt-f/docs/superpowers/plans/2026-11-12-nativeangular-m1-fabric-mvp.md
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
2. **The fake backend's node model is the tree source.** `FakeNode`, `FakeText` and
   `FakeComment` are exported from `@nativeangular/testing` and expose `childNodes`,
   `attributes`, `styles`, `properties` and `nodeValue`. The inspector reads that
   model — it does not re-render anything.
3. **`@nativeangular/contract` does not exist yet.** Another agent is building it in
   parallel. **Do not import it and do not add it as a dependency** — your package
   must build against what exists today. If the inspector would like element metadata
   (the `na-` tag names, the style subset), derive it from the node names; the
   controller will wire the contract in later.
4. **Do NOT modify `packages/cli/src/commands.ts`.** Another agent owns the CLI package
   in parallel; the controller wires the `inspect` command centrally after the merge.
   Build `packages/inspector/**` — including `src/inspect.ts`, which is the function
   the CLI command will call — and leave the command itself alone.
5. **The repo has no `esModuleInterop`/`allowSyntheticDefaultImports`**
   (`tsconfig.base.json`), so use `import * as ts from 'typescript'` for the TypeScript
   API and named imports elsewhere.
6. **`noUncheckedIndexedAccess` is enabled** — `array[0]` is `T | undefined`. The
   brief's code already handles it; keep that discipline.
7. **The HTML must be self-contained.** No CDN link, no external script, no external
   stylesheet, no web font — one file that opens offline, attaches to a pull request or
   gets published. The spec for this is explicit; a test enforces it.
8. **The network is available**, so `pnpm install` works.
9. **Your files are** `packages/inspector/**` plus one line in the root `tsconfig.json`
   (the project reference). Do not touch `packages/core/**`, `packages/contract/**`,
   `packages/cli/**`, `packages/testing/**`, `packages/platform/**` or `docs/**` — other
   agents own those in parallel. If the root `tsconfig.json` edit conflicts when you
   merge, the controller resolves it.

## Your job

1. Implement exactly what the brief specifies, following its step order: failing test
   first, then the minimal implementation, then the passing test.
2. Verify: the two new specs pass, `pnpm test` stays green, and `pnpm run build` exits 0.
3. **Generate the page and look at it.** Run
   `pnpm run compile:fixtures` and then call `inspectApp` with the fixture entry; open
   the produced HTML and confirm it shows the `an-root` host, the `an-view` child with
   its `class="root"` attribute, and the text `hello` — and that the file makes no
   external requests. Report what you saw.
4. Commit with the exact commit message the brief gives, plus the root `tsconfig.json`
   reference and `pnpm-lock.yaml`.
5. **Self-review**: re-read each brief item against your diff and confirm it is done,
   not merely attempted.
6. Report back.

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
- **Testing** — the tests verify real behaviour (the page must be self-contained; the
  tree must carry the styles and the text), and the output is pristine: no stray
  warnings, no noise.

## Report format

Write your full report to:

```
/workspace/nt-f/.superpowers/sdd/2026-11-12-nativeangular-m1-fabric-mvp/task-11-report.md
```

It must contain: what you implemented; what you tested and the results; TDD evidence
(RED and GREEN with the commands and output); **what the generated page looked like
when you opened it**; the files changed; your self-review findings; and any issues or
concerns.

Then report back with ONLY (under 15 lines):

- **Status:** DONE | DONE_WITH_CONCERNS | BLOCKED | NEEDS_CONTEXT
- Commit created (short SHA + subject)
- One-line test summary
- One line describing what the generated page shows
- Your concerns, if any
- The report file path
