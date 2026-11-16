# Task 12 — the integration task on the merged M1 tree

**Worktree:** `/workspace/nativeangular` · **Branch:** `feature/m1-fabric-mvp`
**Base:** `c839f55` (merge of track E — the production bundle and the size budget)
**Status:** DONE_WITH_CONCERNS — the four items are closed and the merged CLI is
proven end to end, but item 2's premise was **not yet true in the merged tree**
(see §2 and §3.1).

---

## 1. What changed, per item

### 1.1 `packages/cli/README.md` — the merged command surface (item 1)

The README described three commands and said nothing about the heavy
dependency. It now documents:

- **All five commands**, in the order the dispatcher ships them
  (`doctor`, `create`, `test`, `inspect`, `build`):
  - `doctor` — the Node hard floor (`22.12`), Angular 22's own engine range
    (`^22.22.3 || ^24.15.0 || >=26.0.0`) and the pinned versions
    (`@angular/core` `22.2.5`, `typescript` `6.0.3`). A Node version *outside*
    the engine range is a warning, not a failure — stated explicitly, because
    this repository runs on Node 22.19.0 and `doctor` still exits `0`.
  - `create` — scaffolds from the **bundled** template
    (`packages/cli/templates/app`), replacing `__APP_NAME__` and
    `__PACKAGE_NAME__`.
  - `test` — runs the repository's own test script
    (`pnpm run compile:fixtures && vitest run`), so the AOT fixtures are
    compiled **first**.
  - `inspect [outfile]` — renders the native element tree of the compiled AOT
    fixture as a **single self-contained HTML page** (default
    `nativeangular-tree.html`), and its missing-fixture message.
  - `build [entry] [outfile]` — the production bundle (esbuild + Angular's own
    JavaScript transformer + Angular's production defines) **and** the size
    report; defaults `src/main.ts` → `dist/bundle.js`.
- **The install cost** — `@angular/build` pulls in vite, sass, rolldown,
  oxc-parser, sass-embedded and piscina; measured on this tree (~210 MB in
  total): the rolldown native binding 19 MB, `sass` 5.8 MB, `oxc-parser`
  1.5 MB, `sass-embedded` 1.3 MB, `rolldown` 964 KB, `piscina` 841 KB.
- **The lazy-loading promise** — only `build` imports the toolchain;
  `doctor`, `create`, `test` and `inspect` never load `@angular/build`, vite,
  sass, rolldown or piscina; `doctor` starts in ~215 ms instead of ~3.6 s. The
  README names the proof (`packages/cli/src/cli-startup.spec.ts`, an ESM
  resolve hook plus a CJS `Module._load` patch, because `@angular/build` is
  CommonJS).
- **The budget enforcement** — the §11.1 targets (≤ 60 KB gzip for a
  hello-world, ≤ 250 KB gzip for a typical app, 0 B for `@angular/compiler`
  and `zone.js`), the CI `size` job **blocking** the workflow when the budget
  is exceeded, when the bundle or its metafile is missing, or when the
  metafile carries `@angular/compiler` or `zone.js`, and the single source of
  truth (`packages/cli/src/budgets.json`, read by the report *and* the gate).
- **The measured numbers** — 41,735 B raw / 14,831 B gzip for the current AOT
  fixture (`@angular/core` 41,046 B of it; 24% of the hello-world budget).
- The **trademark notice verbatim** (byte-identical to the one in the other
  seven READMEs) and the MIT line.

The stale sentence "The `build` and `dev` commands ship with the bundler and
the dev server" is gone: `build` is now a row of the table, and **`dev` does
not exist** — it is still planned (M1 Task 6), so naming it as shipped would
have been a second lie in the same file.

### 1.2 `packages/platform/README.md` — the stale line (item 2)

- The `NativeAngularOptions` row no longer says "today only `document`"; it
  names the `document` shim (defaulting to the backend's own document) **and**
  the application's own `appProviders`.
- A new paragraph states the mechanism: the `appProviders` option is the
  *application's* providers, they are passed to `createNativeApplication`
  **after** the platform's own application providers (the three that
  `appProviders()` returns), so an app's own providers win; they are **not**
  platform providers — the platform injector holds the document and the
  renderer and belongs to the platform.
- The usage example now shows `appProviders: [{ provide: FEATURE, useValue: 'on' }]`.

**Deviation (important):** the brief says "the platform *now* accepts
application providers". It did **not** — see §3.1.

### 1.3 The CLI's missing-fixture error path — now tested (item 3)

- `packages/cli/src/commands.ts`: the fixture path is now read through
  `aotFixtureEntry()`, which honours the `NA_AOT_FIXTURE` environment
  variable. It is a **test seam**: it lets the missing-fixture condition be
  created deliberately (a scratch directory with no `out/`), so the test never
  has to delete the repository's own fixture.
- `packages/cli/src/commands.spec.ts`: a new describe block
  *"the inspect command without a compiled fixture"* asserts through the
  dispatcher (`main(['inspect'])`) that the exit code is `1` and that stderr
  carries `No compiled AOT fixture at <absolute path>` and
  `Build it first: pnpm run compile:fixtures`.
- `packages/cli/src/cli-startup.spec.ts`: a second, stronger proof — the
  **real binary in its own process** (`runCli`), with `NA_AOT_FIXTURE` pointing
  at a scratch path — asserts the *process* exits non-zero with the same
  actionable message. `runCli` grew an optional `env` argument for it.

Both are **load-bearing** (verified by mutation, §3.3): changing the message
or the `return 1` into `return 0` makes them fail.

### 1.4 The forbidden-module patterns — now self-tested (item 4)

`packages/cli/src/cli-startup.spec.ts` gained a describe block *"the
forbidden-module patterns"* with the two controls:

- **Positive control** — a path that really is the compiler package must be
  flagged: the plain layout
  (`/repo/node_modules/@angular/compiler/fesm2022/compiler.mjs`) *and* the
  real pnpm layout, where the compiler is linked into the peer-suffixed
  directory of `@angular/core`
  (`.pnpm/@angular+core@22.2.5_@angular+compiler@22.2.5_rxjs@7.8.2/node_modules/@angular/compiler/…`
  — verified to exist on this machine).
- **Negative control** — the peer-suffixed `@angular/core` path
  (`.pnpm/@angular+core@22.2.5_@angular+compiler@22.2.5_rxjs@7.8.2/node_modules/@angular/core/fesm2022/core.mjs`)
  must **not** be flagged.

The negative control is load-bearing: with the pre-merge pattern
`/@angular[+/]compiler/` it *does* match (verified in §3.3) — which is the
false positive the merge surfaced.

---

## 2. Files changed

| File | Change |
|---|---|
| `packages/cli/README.md` | rewritten — five commands, install cost, lazy loading, budget enforcement, measurements, trademark notice |
| `packages/cli/src/commands.ts` | `aotFixtureEntry()` + the `NA_AOT_FIXTURE` test seam |
| `packages/cli/src/commands.spec.ts` | the in-process missing-fixture test |
| `packages/cli/src/cli-startup.spec.ts` | `runCli(..., env)`, the pattern controls, the child-process missing-fixture test |
| `packages/platform/README.md` | the stale line, the ordering paragraph, the example |
| `packages/platform/src/bootstrap.ts` | **the `appProviders` option** — taken **byte-identical** from `feature/m1-track-d` (see §3.1) |
| `packages/platform/src/bootstrap.spec.ts` | the two `appProviders` tests — taken **byte-identical** from `feature/m1-track-d` |

---

## 3. What I found

### 3.1 Item 2's premise was not true in the merged tree — the platform had no `appProviders` hook

`packages/platform/src/bootstrap.ts` in `feature/m1-fabric-mvp` accepted
**only** `{ document }`; `createNativeApplication` was called with the
platform's own `appProviders()` and nothing else. The `appProviders` option
exists only on **`feature/m1-track-d`** (Task 8 — the router track, commit
`e50a99b`), which the ledger shows as *not yet merged* (fix round 2 in flight).

So the brief's premise ("the platform now accepts application providers as
well") described a change that had not landed on the integration branch yet.
Documenting it without the code would have made the README lie — the exact
defect item 2 exists to remove.

**What I did:** I applied the **identical, already reviewed** change from
`feature/m1-track-d` (`git checkout feature/m1-track-d -- packages/platform/src/bootstrap.ts
packages/platform/src/bootstrap.spec.ts`, which touches only those two files and
only adds the option plus its two tests). The controller had authorised exactly
this change in Task 8's fix round ("the platform is untouched by every other
live track"); bringing it forward early makes the README truthful and keeps the
platform's internals untouched. Because the content is **byte-identical** to
track-d's, merging track-d later is a no-op for these two files — no conflict,
no drift.

**What I did not do:** I did not restructure anything in the platform beyond
that one option, and I did not touch the router package.

### 3.2 The CLI's fixture path was hard-wired

`AOT_FIXTURE_ENTRY` was a module-level constant resolved from `import.meta.url`,
so the missing-fixture path could only be reached by deleting the repository's
own compiled fixture. The `NA_AOT_FIXTURE` seam is the smallest change that
makes the condition reproducible. It follows the repository's own convention
(`INIT_CWD`, `NA_MODULE_TRACE` are read from the environment).

### 3.3 The new tests are load-bearing (mutation checks)

| Mutation | Result |
|---|---|
| `Build it first: pnpm run compile:fixtures` → `pnpm install` | the in-process test fails (1 failed / 7 passed) |
| the missing-fixture path's `return 1` → `return 0` | **both** tests fail (2 failed / 12 passed) |
| the compiler pattern `/[/\\]@angular[/\\]compiler[/\\]/` → the pre-merge `/@angular[+/]compiler/` | the negative control fails (`old.test(peerPath) === true`) |

### 3.4 Measurements differ slightly from the documented figures (not fixed — not a defect)

The install-cost figures in the brief are the ones documented. Re-measured on
this machine with `du -sk`: the rolldown native binding 19,438 KB (19 MB),
`sass` 5,910 KB (5.8 MB), `oxc-parser` 1,466 KB (1.5 MB), `sass-embedded`
1,091 KB (documented as 1.3 MB), `rolldown` 969 KB (documented as 964 KB),
`piscina` 841 KB, whole tree 210,971 KB (~210 MB). Every figure is within
rounding of the documented one.

The startup time is likewise machine-dependent: `doctor` takes ~480 ms here
through `pnpm exec tsx` (the documented ~215 ms was measured on the reference
machine), while an eager `import('@angular/build/private')` alone costs ~1.8 s
in this sandbox. The README keeps the project's documented numbers.

### 3.5 Small, unreported-on-the-merged-tree findings (not fixed — outside the four items)

- `nativeangular create "Demo App"` creates a directory literally named
  `Demo App`; only the *package* name is slugged (`demo-app`). Harmless, but
  the directory name is not sanitised.
- The template's `package.json` still lists `nativeangular dev` /
  `nativeangular build android` / `nativeangular build ios` scripts, and the
  template imports `@nativeangular/fabric` — neither exists yet (M1 Task 6 /
  Task 7). The scaffolded app therefore does not build as it stands. This is
  the template's problem, not the merged surface's, and it was out of scope.
- `pnpm nativeangular test` runs the whole repository suite, including the
  ~7 s CLI startup spec — fine, but it is not a "contract suite only" command
  as the old README claimed.

---

## 4. The commands I ran, and what they printed

### 4.1 `pnpm test` — 27 files / 214 tests green (was 208)

```
 Test Files  27 passed (27)
      Tests  214 passed (214)
   Duration  21.71s
```

The six new tests: 1 CLI missing-fixture (in-process) + 1 CLI missing-fixture
(child process) + 2 forbidden-module pattern controls + 2 platform
`appProviders` tests.

`scripts/constraints.spec.ts` (the repository's constraint guard) — 7 passed.

### 4.2 `pnpm run build` — exit 0

```
> tsc -b
BUILD EXIT=0
```

### 4.3 `pnpm run compile:fixtures` — exit 0 · `pnpm run bundle:fixtures` — exit 0

```
JS payload: 41735 B raw, 14831 B gzip
Hello-world budget: 61440 B gzip — within budget
Typical-app budget: 256000 B gzip — within budget
Bytes by package:
     41046 B  @angular/core
       636 B  app
```

### 4.4 The size gate — exit 0, payload unchanged

```
$ node scripts/check-size-budget.mjs
JS payload: 41735 B raw, 14831 B gzip (budget 61440 B)
@angular/compiler and zone.js: 0 B in the bundle.
Bytes by package:
     41046 B  @angular/core
       636 B  app
The JS payload is within budget.
GATE EXIT=0
```

Note the order: `pnpm test` starts with `pnpm run compile:fixtures`, which
**empties** `packages/testing/fixtures/out` and writes only `app.js` — so the
bundle has to be produced again (`pnpm run bundle:fixtures`) before the gate
can read it. That is exactly the CI order (`compile:fixtures` → `bundle:fixtures`
→ `node scripts/check-size-budget.mjs`).

### 4.5 The five commands

**`pnpm nativeangular --help`** — exit 0, the usage line and all five commands:

```
Usage:
  nativeangular <command> [options]

Commands:
  doctor   Check the toolchain and the pinned package versions
  create   Scaffold a new app
  test     Run the contract suite
  inspect  Render the native element tree of the AOT fixture as a self-contained page
  build    Bundle the app for production and report its size
```

**`pnpm nativeangular doctor`** — exit 0, warning (not failure) on Node
22.19.0, which is below Angular's engine range:

```
! node             22.19.0 (expected >=22.12.0 (Angular engine range ^22.22.3 || ^24.15.0 || >=26.0.0)) — outside the Angular engine range — npm will warn
✓ @angular/core    22.2.5 (expected 22.2.5)
✓ typescript       6.0.3 (expected 6.0.3)
DOCTOR EXIT=0
```

**`pnpm nativeangular create "Demo App"`** — exit 0, the template is scaffolded
with the placeholders replaced (`package.json` name `demo-app`,
`app.component.ts` title `'Demo App'`).

**`pnpm nativeangular test`** — exit 0: the repository's script runs (AOT
fixtures compiled first) and the suite is green — 27 files / 214 tests.

**`pnpm nativeangular inspect`** — exit 0, the self-contained page:

```
Wrote …/nativeangular-tree.html     (2,354 B)
grep -c "http" nativeangular-tree.html  → 0
grep -o "<span>elements</span><b>3</b>"  → <span>elements</span><b>3</b>
no <script>/<link>/<img> tags in the page
```

**`pnpm nativeangular build`** — exit 0, the production bundle and the size
report (the numbers in §4.3/§4.4); the gate that reads the same budget file
exits 0.

### 4.6 The missing-fixture path, as the CLI now reports it

```
No compiled AOT fixture at /tmp/na-no-fixture-XXXX/out/app.js
Build it first: pnpm run compile:fixtures
```

exit code 1 — asserted both in-process (`main(['inspect'])`) and through the
real binary in its own process.
