# Task 5 report — AOT fixture pipeline and the end-to-end proof

**Status:** DONE
**Commit:** `ca1c789` — `test: AOT fixture pipeline and end-to-end proof on the fake backend`
**Branch:** `feature/m0-spike` (base `2472397`)

---

## 1. What I implemented

Exactly the seven artefacts the brief lists, nothing more:

| File | Kind | Purpose |
|---|---|---|
| `packages/testing/src/aot.e2e.spec.ts` | new | The end-to-end proof: a real `ngc`-compiled Angular component rendered through the DOM-less platform onto the fake backend. |
| `packages/testing/src/teardown.ts` | new | `disposeNativeApplication(appRef)` — destroys the application **and** the platform that owns it (public `PlatformRef` only). |
| `packages/testing/src/index.ts` | modified | One added line: `export * from './teardown';` |
| `packages/testing/fixtures/src/app.ts` | new | The fixture components `AnRootComponent` (signal `label`, binds it to the child) and `AnViewComponent` (signal **input** `label`, static `class="label"` span). |
| `packages/testing/fixtures/tsconfig.json` | new | The standalone `ngc` project (`module: preserve`, `moduleResolution: bundler`, `strictTemplates`, `compilationMode: full`). |
| `scripts/compile-fixtures.mjs` | new | Runs `node_modules/@angular/compiler-cli/bundles/src/bin/ngc.js -p packages/testing/fixtures/tsconfig.json`. |
| `package.json` | modified | `"test": "pnpm run compile:fixtures && vitest run"` — the suite can never run against a stale artefact. |

`compile:fixtures` already existed in `package.json` from Task 1; it was **not** duplicated.
The compiled artefact `packages/testing/fixtures/out/` is covered by the existing `.gitignore` (`out/`) and is **not** committed — `git check-ignore -v` confirms `.gitignore:3:out/`.

The fixture is deliberately outside the `tsc -b` solution: `packages/testing/tsconfig.json` includes only `src/**/*.ts`, and the root `tsconfig.json` references were not touched. `pnpm run build` therefore still compiles the fixture exactly once — via `ngc`.

---

## 2. TDD evidence

### RED — before the implementation

Command:

```
$ pnpm vitest run packages/testing/src/aot.e2e.spec.ts
```

Output (abridged):

```
 ❯ packages/testing/src/aot.e2e.spec.ts (3 tests | 3 skipped) 7ms
   ↓ NativeAngular — AOT component on the fake backend > renders the tree and binds a signal input to the child component
   ↓ NativeAngular — AOT component on the fake backend > re-renders when the signal changes (zoneless)
   ↓ NativeAngular — AOT component on the fake backend > renders with nothing declared — proof that no property write ever happens

⎯⎯⎯⎯ Failed Suites 1 ⎯⎯⎯⎯

 FAIL  packages/testing/src/aot.e2e.spec.ts [ packages/testing/src/aot.e2e.spec.ts ]
Error: AOT fixtures are missing. Run `pnpm run compile:fixtures` before the tests.
 ❯ packages/testing/src/aot.e2e.spec.ts:29:11
     27| beforeAll(async () => {
     28|   if (!existsSync(fileURLToPath(fixtureUrl))) {
     29|     throw new Error(
          |       ^
     30|       'AOT fixtures are missing. Run `pnpm run compile:fixtures` befor…

 Test Files  1 failed (1)
      Tests  3 skipped (3)
```

The type-check failed in the same state, proving the second missing piece:

```
$ pnpm run build
packages/testing/src/aot.e2e.spec.ts(6,29): error TS2305: Module '"./index"' has no exported member 'disposeNativeApplication'.
 ELIFECYCLE  Command failed with exit code 1.
```

**Why this failure was expected:** the spec was written first, against two artefacts that did not exist yet — the `ngc`-compiled fixture (`packages/testing/fixtures/out/app.js`) and the `disposeNativeApplication` export. Both failures are "the thing under test does not exist yet", not a broken assertion.

### GREEN — after the implementation

```
$ pnpm vitest run packages/testing/src/aot.e2e.spec.ts
 ✓ packages/testing/src/aot.e2e.spec.ts (3 tests) 41ms

 Test Files  1 passed (1)
      Tests  3 passed (3)
```

Full suite and build, from a clean `out/` (deleted first, so the pipeline is exercised end to end):

```
$ pnpm test
> nativeangular@0.0.0 test /workspace/nativeangular
> pnpm run compile:fixtures && vitest run

> nativeangular@0.0.0 compile:fixtures /workspace/nativeangular
> node scripts/compile-fixtures.mjs
AOT fixtures compiled to packages/testing/fixtures/out

 ✓ packages/testing/src/aot.e2e.spec.ts (3 tests) 62ms
 ✓ packages/platform/src/bootstrap.spec.ts (3 tests) 27ms
 ✓ packages/testing/src/fake-renderer.spec.ts (17 tests) 10ms
 ✓ scripts/check-toolchain.spec.ts (9 tests) 7ms
 ✓ packages/testing/src/fake-backend.spec.ts (6 tests) 7ms
 ✓ packages/core/src/registry/element-registry.spec.ts (3 tests) 4ms

 Test Files  6 passed (6)
      Tests  41 passed (41)
```

```
$ pnpm run build
> nativeangular@0.0.0 build /workspace/nativeangular
> tsc -b
(exit 0)
```

---

## 3. The AOT proof

### 3.1 The signal input marker

The brief's grep is `grep -o 'inputs:{[^}]*}'`. The compiler emits a **space** after the colon, so that pattern matches nothing; the working pattern is `grep -o 'inputs: {[^}]*}'`:

```
$ grep -o 'inputs: {[^}]*}' packages/testing/fixtures/out/app.js
inputs: { label: [1, "label"] }
```

**This is present.** The marker sits inside the AOT component definition of the child:

```js
static { this.ɵcmp = /*@__PURE__*/ i0.ɵɵdefineComponent({
    type: AnViewComponent,
    selectors: [["an-view"]],
    inputs: { label: [1, "label"] },
    decls: 2, vars: 1, consts: [[1, "label"]],
    ...
```

Flag `1` is `InputFlags.SignalBased`, confirmed against the pinned Angular 22.2.5 runtime
(`node_modules/@angular/core/fesm2022/_debug_node-chunk.mjs:5452`):

```js
InputFlags[InputFlags["SignalBased"] = 1] = "SignalBased";
```

and against the public API docs in `node_modules/@angular/core/types/core.d.ts:1438`:

```ts
*   signalInput: [InputFlags.SignalBased, "signalInput"],
```

**The signal input was compiled. The end-to-end test is trustworthy.**

### 3.2 The static attribute in the `consts` table

The brief offers two ways to confirm this — the literal string `class="root"`, or the `consts` array. The AOT compiler does **not** emit the literal string: a static class attribute is encoded as the `AttributeMarker.Classes` marker (`1`) followed by the class name. So the literal grep finds nothing, and the `consts` inspection shows the truth:

```
$ grep -o 'consts: \[\[[^]]*\]' packages/testing/fixtures/out/app.js
consts: [[1, "label"]
consts: [[1, "root", 3, "label"]
```

The root component's table is `[[1, "root", 3, "label"]]`:

* `1` = `AttributeMarker.Classes`, `"root"` → the static `class="root"`;
* `3` = `AttributeMarker.Template`, `"label"` → the `[label]="label()"` property binding.

(The child's `[[1, "label"]]` is the static `class="label"` of the inner `<span>`.)

Full root definition for the record:

```js
static { this.ɵcmp = /*@__PURE__*/ i0.ɵɵdefineComponent({
    type: AnRootComponent,
    selectors: [["an-root"]],
    decls: 1, vars: 1,
    consts: [[1, "root", 3, "label"]],
    template: function AnRootComponent_Template(rf, ctx) {
        if (rf & 1) { i0.ɵɵelement(0, "an-view", 0); }
        if (rf & 2) { i0.ɵɵadvance(); i0.ɵɵproperty("label", ctx.label()); }
    },
    dependencies: [AnViewComponent],
    encapsulation: 2 });
}
```

### 3.3 One Angular instance (verified, not assumed)

The platform, the testing package and the compiled fixture all resolve `@angular/core` to the **same realpath**:

```
$ readlink -f packages/platform/node_modules/@angular/core
…/node_modules/.pnpm/@angular+core@22.2.5_@angular+compiler@22.2.5_rxjs@7.8.2/node_modules/@angular/core
$ readlink -f packages/testing/node_modules/@angular/core
…/node_modules/.pnpm/@angular+core@22.2.5_@angular+compiler@22.2.5_rxjs@7.8.2/node_modules/@angular/core
$ node -e "console.log(require.resolve('@angular/core',{paths:['packages/testing/fixtures/out']}))"
…/node_modules/.pnpm/@angular+core@22.2.5_.../node_modules/@angular/core/fesm2022/core.mjs
```

The signal write path is the decisive probe: `ɵINPUT_SIGNAL_BRAND_WRITE_TYPE` is a plain
`Symbol()` (`core.mjs:24`), i.e. unique per module instance. The fixture's signal is
written by the platform's Angular and the interpolation renders `hello` — so the brand
matches and the instances are identical. `resolve: { dedupe: ['@angular/core'] }` was
**not** added to `vitest.config.ts`; it is not needed.

---

## 4. The negative case (Step 8) and mutation testing

The brief asks that the test **fail if the AOT proof is broken**. I verified this by
mutating the compiled artefact and the fixture source, then restoring both (the
restoration was checked: `git diff --stat` on the fixture source is empty).

### Mutation A — remove `inputs: { label: [1, "label"] }` from the compiled artefact

Simulates "the signal input was not registered" (the JIT trap).

```
 ❯ packages/testing/src/aot.e2e.spec.ts (3 tests | 3 failed) 198ms
   × … renders the tree and binds a signal input to the child component
     → expected [ 'label' ] to deeply equal []
   × … re-renders when the signal changes (zoneless)
     → expected '' to be 'updated' // Object.is equality
   × … renders with nothing declared — proof that no property write ever happens
     → expected '' to be 'hello' // Object.is equality
```

and on stderr the strict Task 4 rule fired exactly as designed:

```
ERROR Error: Unknown property "label" on <an-view>. Known properties: (none declared)
    at FakeRenderer.setProperty (packages/testing/src/fake-renderer.ts:108:13)
    at setDomProperty (…/packages/core/src/render3/instructions/shared.ts:318:14)
    at setPropertyAndInputs (…/packages/core/src/render3/instructions/shared.ts:274:3)
    at Module.ɵɵproperty (…/packages/core/src/render3/instructions/property.ts:48:5)
    at AnRootComponent_Template (packages/testing/fixtures/out/app.js:36:16)
```

All three assertions fail. The proof is genuinely load-bearing.

**One precise nuance worth recording:** the `setProperty` throw does **not** propagate
out of the test. It is raised inside Angular's change detection, so Angular's default
`ErrorHandler` catches it and logs it to stderr; the view is left half-rendered and the
test fails on the *assertion* (`expected '' to be 'hello'`) rather than on the thrown
error. The brief's wording ("a single `setProperty` call would throw") is accurate about
the renderer, but the failure surfaces through Angular's error handler. The assertion still
catches it — which is what matters.

### Mutation B — remove `class="root"` from the fixture template

```
consts: [[1, "label"]
consts: [[3, "label"]
```

```
   × … renders the tree and binds a signal input to the child component
     → expected null to be 'root' // Object.is equality
 Test Files  1 failed (1)
      Tests  1 failed | 2 passed (3)
```

The `consts` table is genuinely what drives the static attribute.

### Mutation C — the teardown helper really destroys the platform

A throwaway spec (since deleted, never committed) asserted the helper's actual contract:

```
expect(platformRef).toBeDefined();       // PlatformRef resolves through the app injector
expect(platformRef!.destroyed).toBe(false);
disposeNativeApplication(appRef);
expect(appRef.destroyed).toBe(true);
expect(platformRef!.destroyed).toBe(true);   // ✓ passed
```

So the helper really destroys the platform — it is not a no-op that only kills the app.

---

## 5. Files changed

```
 M package.json                                (1 line: the `test` script)
 M packages/testing/src/index.ts                (1 line: export the teardown helper)
 A packages/testing/fixtures/src/app.ts
 A packages/testing/fixtures/tsconfig.json
 A packages/testing/src/aot.e2e.spec.ts
 A packages/testing/src/teardown.ts
 A scripts/compile-fixtures.mjs
```

7 files, 180 insertions, 1 deletion. No existing package behaviour was changed:
`packages/core` and `packages/platform` are untouched.

---

## 6. Constraint check

| Constraint | Result |
|---|---|
| Angular exactly `22.2.5`, TypeScript exactly `6.0.3` | ✅ unchanged — no dependency was added. |
| AOT only — no `@angular/compiler` in any runtime/test bundle | ✅ grep over the repo finds `@angular/compiler` only in `@angular/compiler-cli` (the `ngc` binary path in the build script) and in comments. `@angular/core/testing` / `TestBed` are never imported. |
| Zoneless only — no `zone.js` | ✅ grep finds no `zone.js` anywhere. |
| `ɵ` API only in `packages/platform/src/angular-internals.ts` | ✅ grep over `packages/**/*.ts` finds `ɵ` only in that file (plus a comment in `providers.ts` that names `ɵINJECTOR_SCOPE`). The `ɵ` in `fixtures/out/app.js` is *compiler output*, not source — it is gitignored, and every AOT Angular component contains it by construction. |
| `@angular/platform-browser` banned from runtime code | ✅ only mentioned in comments explaining why it is banned. |
| Scope `@nativeangular/*`, licence MIT | ✅ no new package published; the fixture is an internal directory of `@nativeangular/testing`. |
| `pnpm test` green, `pnpm run build` exit 0 | ✅ verified above. |

---

## 7. Self-review findings

Things I checked and fixed / confirmed:

1. **The teardown helper was verified to be real** (mutation C above) — `appRef.injector.get(PlatformRef, undefined)` resolves through the app injector's parent to the platform injector; `PlatformRef` is provided there by Angular's `createPlatform` (`injector.get(PlatformRef)`), and it is public API — no `ɵPlatformRef`.
2. **`flush()`'s extra macrotask is a timing guard, not dead code — but it is not currently load-bearing *in this environment*.** I removed the `setTimeout` and the suite still passed 3/3. The guard is still correct to keep: Angular 22's `ApplicationRef.isStable` is derived from the pending-task queue alone (`isStable() { return this.internalPendingTask.hasPendingTasksObservable.pipe(map(pending => !pending)); }`, `_debug_node-chunk.mjs:13232`), while `ChangeDetectionSchedulerImpl` adds and removes its *own* pending task around the tick (`_debug_node-chunk.mjs:19503+`) — so `whenStable()` resolving is not a guarantee that the scheduled tick already ran. The comment explaining exactly this stays, as the brief requires.
3. **Output noise.** `Angular is running in development mode.` appears on stdout in the new spec. It is Angular core's own banner (`_debug_node-chunk.mjs:13298`) and it is **pre-existing** — the Task 3 platform spec prints the identical line on every bootstrap. It is not new noise introduced by this task and suppressing it would mean changing Angular's behaviour, which is out of scope.
4. **`git check-ignore`** confirms `packages/testing/fixtures/out/` is ignored, so no build artefact is committed.
5. **The fixture is compiled exactly once.** `packages/testing/tsconfig.json` includes only `src/**/*.ts`; the root `tsconfig.json` references were not extended. `pnpm run build` does not touch the fixture.
6. **No `dedupe` was added to `vitest.config.ts`** — the single-instance property was proven by realpath inspection plus the signal-brand behaviour, so the knob would have been cargo-culted.
7. **Scope discipline:** nothing outside the brief's file list was touched.

---

## 8. Issues / concerns

1. **The brief's two Step 6 greps do not match Angular's actual output.**
   * `grep -o 'inputs:{[^}]*}'` → no match (the compiler emits `inputs: { … }` with a space). The working pattern is `grep -o 'inputs: {[^}]*}'`, and the *content* the brief expects — `label: [1, "label"]`, flag `1` = `InputFlags.SignalBased` — is exactly what is there.
   * `grep -o 'class="root"'` → no match. The AOT compiler encodes a static class attribute as `AttributeMarker.Classes` (`1`) + the class name, so the string `class="root"` never appears. The brief's alternative ("or inspect the `consts` array") holds: `consts: [[1, "root", 3, "label"]]` — the string `root` **is** present in the component's `consts`, as required.
   No code change was needed; this is a documentation-level imprecision in the brief.

2. **The negative case fails through Angular's error handler, not through the test.** As described in §4: the strict `setProperty` rule throws inside change detection, Angular's default `ErrorHandler` logs it to stderr, and the assertion fails on the resulting empty render. The test still fails — but a future reader expecting a thrown `Unknown property` error to fail the test directly should know the error is swallowed by Angular first.

3. **Environment note (not a code issue):** the *first* `pnpm run compile:fixtures` invocation failed with `TS500: Error: ENOENT: no such file or directory, lstat '…/packages/testing/fixtures/tsconfig.json'` even though the file existed (`ls` and a direct `fs.lstatSync` both confirmed it). The identical command succeeded on the next run, and every run since. It looks like a write-visibility race in this sandbox's shared mount between the file write and the immediately following shell command, not a defect in the script — the script resolves the project path from `import.meta.url` and `spawnSync`s `ngc` with the absolute path, which is correct.

4. **`flush()`'s extra macrotask is currently redundant** (see §7.2). Kept, with its explanatory comment, as the brief mandates and as protection against a real (but currently not-reproducible-here) ordering race.

None of these blocks the milestone: the AOT proof is present, the end-to-end test is green, the negative case is live, and the whole suite plus the build pass.
