# AngNative — Project Review

**Date:** 2026-11-20 (Fri)
**Reviewer:** AI agent review (static analysis + locally executed build/test verification)
**Scope:** Full monorepo — Rust crates, TypeScript packages, native platform shells, docs, scripts, and CI.
**Repo state at review:** `master` at `e86d73c`, clean working tree, in sync with `origin/master`.

---

## Verdict

An ambitious, well-conceived monorepo with some genuinely solid subsystem code (Angular renderer, Taffy layout, mutation protocol, conformance tests). However, the project is **not currently in a buildable, end-to-end functional state**, and the README/docs substantially overstate maturity. The gap is mostly *wiring and honesty*, not a lack of code.

Paths below are relative to the repository root.

---

## 1. Build & test status (reproduced on this host)

| Check | Result |
|---|---|
| `pnpm build` | ✅ Pass |
| `pnpm test` (13 spec files) | ✅ **171 passed** |
| `cargo test -p angnative-host-core` | ✅ 8 passed |
| `cargo test -p angnative-host-layout` | ✅ 18 passed |
| `cargo test -p angnative-host-view --test conformance_spec` | ✅ 14 passed |
| `cargo test -p angnative-host-view --lib` | ❌ **13 passed, 4 failed** |
| `cargo test -p angnative-host-ffi` | ❌ **Fails to compile** (15 errors in `ffi_spec.rs`) |
| `cargo test -p angnative-services-core` | ❌ **Fails to compile** (9 lib errors, 13 test errors) |
| `cargo test --workspace` | ❌ `rquickjs-sys` build script needs elevation for `patch.exe` on Windows (host-specific) |

So **two workspace crates do not compile**, and one compiling crate has real failing tests. The "build-passing" badge in `README.md:13` is a hardcoded static shield; CI never runs Rust builds/tests at all (`.github/workflows/ci.yml` has only `test` and `size` jobs).

---

## 2. Critical: the runtime is not wired end-to-end

These are the findings that matter most — each independently prevents the advertised flow (Angular component → mutations → Rust → native UI) from working.

1. **Renderer factory is never registered in DI.** `packages/platform/src/native-angular.ts:56` injects `AngNativeRendererFactory2`, but no provider exists anywhere (it isn't `@Injectable`, `packages/core/src/renderer.ts:551`). All `na-*` primitives inject that token (`na-view.component.ts:28`, etc.) → `NullInjectorError` at component creation.

2. **No flush path.** `apps/demo-app/src/main.ts:11-13` passes no `nativeConfig`; `renderer.ts:136` sets `flush = null`, and `renderer.ts:528` returns early when no flush callback exists. Mutations accumulate in memory and are never delivered.

3. **Native→Angular events are a stub.** `renderer.ts:457-494` `listen()` only encodes a `SetProperty` with an opaque ID; `dispatchEvent()` is called only by tests. Unlisten functions are written to unused element properties (`na-button.component.ts:52`, `na-text-input.component.ts:57`); `na-text-input.component.ts:60-62` `onInput()` is an explicit no-op. Button presses cannot reach outputs.

4. **Demo uses a non-existent output.** `apps/demo-app/src/counter.component.ts:29` binds `(press)`, but `NaButtonComponent` declares only `(click)` (`na-button.component.ts:38`). The README example repeats this (`README.md:124`).

5. **FFI never updates the shadow tree.** `crates/angnative-host-ffi/src/bridge.rs:287-310` calls only `emitter.process_batch()`, never `engine.run_batch()`. Yet `angnative_host_node_count` / `angnative_host_snapshot` (`ffi.rs:291-310`) read from `host.engine()` — they always report empty. `Host.tree` is dead state.

6. **Android view callbacks are null.** `platforms/android/app/src/main/cpp/jni_bridge.cpp:86-96` registers all 8 native view callbacks as `nullptr`. The Kotlin UI (`AngNativeView.kt:99-380`) is hand-built; the Rust batch is only logged (`:57-97`). Placement uses Java `identityHashCode` as a native pointer (`JniBridge.kt:261-264`), and layout runs on a hardcoded empty node set (`jni_bridge.cpp:133-136`).

7. **QuickJS is disconnected.** `host-engine` is feature-complete and unit-tested, but `host-ffi/Cargo.toml` has no dependency on it; `runtime.rs:298-303` `flush()` is explicitly a no-op. The only consumer is `angnative-desktop`, which is commented out of the workspace (`Cargo.toml:11`).

8. **iOS shell does not exist.** `platforms/` contains only `android`; no SwiftPM project, no Swift code. `README.md:17,211`, `docs/ROADMAP.md:26-29`, and `docs/ARCHITECTURE.md:106-109` claim iOS readiness. `TestApp.ipa` is local but gitignored (`.gitignore:25`). *Note: iOS build verification is planned for 2026-11-21 on a Mac.*

---

## 3. Rust correctness & safety

- **Panic across FFI:** `element_registry.rs:56-67` panics on missing/duplicate IDs with attacker-controlled IDs from JSON (`mutation.rs:145-233`); `MutationError::NodeNotFound` is declared but never constructed. Only `angnative_host_create` uses `catch_unwind` (`ffi.rs:43`); combined with `panic = "abort"` (`Cargo.toml:32`), a malformed batch can abort the host process.
- **Failing tests (reproduced):** four `op_emitter` tests fail in `host-view` — `child_of_flattened_node_attaches_to_grandparent`, `flattened_node_with_children_then_promoted`, `remove_child_releases_to_recycle_pool`, `insert_child_reuses_recycle_slot` (assert at `op_emitter.rs:734/794/850/898`). Flattening/recycling behavior is therefore broken or its tests encode wrong expectations.
- **ABI inconsistencies:** `out_len` means byte length in some functions and op count in others (`ffi.rs:178` vs `:215`); header declares `const char*` while Rust docs say caller must free (`angnative_host_ffi.h:42/80`, `ffi.rs:75-79`); `process_batch_raw` bypasses validation and view factory; header omits `angnative_host_free_string`. No double-free protection on `Box::from_raw` (`ffi.rs:59`).
- **Unsound `Sync`:** `unsafe impl Send/Sync` for raw `*mut c_void` callback state (`bridge.rs:101-107`) justified by "single thread" but `Sync` asserts cross-thread sharing is safe.
- **Fake crypto:** `services-core/src/crypto.rs` is XOR obfuscation (`:155-166`) despite "authenticated encryption" claims (`:5-6`); empty nonce causes divide-by-zero (`:182`). The crate doesn't compile anyway.
- **Other stubs:** `text_measurer.rs:45-89` returns `{0,0}` / `chars()*font_size*0.5` — no real text measurement; `sync.rs` claims durable persistence but uses only in-memory `BTreeMap`; `diagnostics.rs:210-226` `subscribe()` just increments a counter; `quickjs.rs:135-139` `memory_usage()` always `None`.
- **Excluded crates are stale:** `angnative-desktop` has compile errors (`app.rs:265-270` wrong field names, `main.rs:17` missing `env_logger`, borrow-check conflict at `app.rs:65-66`) and renders colored rectangles (`renderer.rs:296-337`). `angnative-core` has no `lib.rs`.

---

## 4. TypeScript / packaging quality

The good: `packages/core/src/renderer.ts` implements a complete `Renderer2` surface with batch encoding and shadow tree (`native-element.ts`); primitives are real Angular components; the protocol matches across TS and Rust (`PROTOCOL_VERSION = 1`, 7 command kinds).

The problems beyond the wiring issues in §2:

- **Conformance suite is partly inert:** of 9 fixtures, 7 use expectation keys the harness doesn't recognize (`conformance.spec.ts:26-34`); their only assertion is `expect(result).toBeDefined()`. Rust conformance runner similarly early-returns on missing keys (`conformance_spec.rs:88-91,171-174,218-221`).
- **Style validation not enforced:** `renderer.ts:27` imports `validateStyleMap` but `setStyle` (`:403-419`) never calls it, contradicting `elements.spec.ts:57-60`.
- **Zoneless claim false:** `bootstrap.ts:49-51` calls `provideZoneChangeDetection` even when `zoneless: true`, while importing `@angular/platform-browser` (`:59`), contradicting `docs/ARCHITECTURE.md:78` and `REQUIREMENTS.md:34`. Angular 22's `provideZonelessChangeDetection` is available but unused.
- **Broken fresh checkout:** `dist/` is gitignored, cross-package imports resolve to `dist/*`, but `pnpm build` only builds the CLI. `compile:fixtures` (`package.json:16`) references a script that doesn't exist, and CI calls it twice. `pnpm start` points to a script that doesn't exist (`demo-app/package.json` has only `start:web/desktop/android` echoes).
- **Low-value tests:** `create.spec.ts:51-84` never calls `createApp`; `dev.spec.ts` never calls `start()`. (One reviewer's inference that `bundle.spec.ts` uses `require()` in ESM is disproven — all 171 tests passed.)
- **Licensing/hygiene:** `README.md:18,243` links a `LICENSE` file that does not exist; ~114 MB of prebuilt `.a`/`.so` binaries are committed under `platforms/android/.../jniLibs`; 6 `tsconfig.tsbuildinfo` caches are tracked while `Cargo.lock` (which pins taffy) is gitignored.

---

## 5. Documentation drift (high confidence, static evidence)

| Claim | Reality |
|---|---|
| Taffy **0.5.2** (`README.md:75,95`, `ARCHITECTURE.md:42,90`, `SPECIFICATIONS.md:32`) | `Cargo.toml:40` pins **0.15.1** (ADR agrees) |
| Angular **19+** (`README.md:14,96`) | Everything pins **22.2.5** |
| Android 17 / API 35 (`README.md:35,210`) | `build.gradle.kts:15` targets SDK **34** |
| "Zero-copy binary protocol" (`README.md:29,70,97`, `SPECIFICATIONS.md:15-26`) | Wire format is `JSON.stringify` (`protocol.ts:115-118`, `bridge.rs:302-309`) |
| NDK docs r25 / scripts r27c / Gradle r27 | Mutually inconsistent (`ANDROID-BUILD.md:11`, `build-android-native.sh:36-45`, `build.gradle.kts:10`) |
| "Instant HMR" enforced (`REQUIREMENTS.md:16,67`) | `cli/src/dev.ts:168-175` is `setTimeout` + "placeholder for ngc call" |
| README API `import { NaView, ... }` (`README.md:108`) | `common/src/index.ts:9-17` exports only `*Component` names |

To their credit, `docs/adr/0001` and `platforms/android/README.md:87-92` are candid about what is unverified/pending — that honesty should be propagated upward.

---

## 6. What is genuinely good

- **Layering is clean and acyclic**: `host-core → host-layout → host-view → host-ffi`; `host-engine` / `services-core` independent. No dependency cycles.
- **Real Taffy integration**: `taffy_adapter.rs:39-145` and `layout_engine.rs` have 18 meaningful tests (flex, grid, RTL, insets, cycles, NaN guarding).
- **Real QuickJS bindings** with 6 tests exercising eval/call/globals/GC (`quickjs.rs:232-288`).
- **Solid mutation core** (`mutation_spec.rs`, 8 tests) and a broad protocol contract shared TS↔Rust.
- **Substantive Android Gradle scaffold** with Kotlin/C++ sources and prebuilt per-ABI libraries.
- **Good test discipline on the TS side** — 171 passing specs across 13 files.

---

## 7. Recommended priorities

1. **Make the workspace compile and CI honest:** fix `services-core` (9 errors: missing `serialize_messagepack`, non-existent `QueuedOperation` fields, `f64: Eq`, moves) and `ffi_spec.rs` (15 errors: missing `c_void` imports, `LayoutRequestJson` not `Serialize`); add Rust build+test to CI; remove or replace the missing `compile:fixtures` step; drop the static build badge.
2. **Fix the 4 failing `host-view` tests** — they point at real flattening/recycling bugs.
3. **Wire the vertical slice:** register the renderer factory, provide a default flush backend, implement `dispatchEvent`, use the `click` output consistently, call `engine.run_batch()` in `process_batch_json`.
4. **Wire Android callbacks** to `ViewFactory` (currently all null) and stop hand-building the Kotlin UI; lock the header/ABI contract with tests.
5. **Harden FFI:** return `Result` instead of panicking on bad IDs, wrap all extern functions in `catch_unwind`, unify `out_len` semantics, remove unsound `Sync`, add explicit free-function ownership.
6. **Reconcile docs with code** (Taffy, Angular, API level, binary protocol, zoneless, iOS/desktop status) and add the missing `LICENSE`.
7. **Repo hygiene:** remove committed 100+ MB native artifacts and `tsbuildinfo` caches (use releases/artifact storage), commit `Cargo.lock`, and add Windows-friendly script wrappers.
8. **iOS (2026-11-21 Mac session):** first verify the shell actually exists (it currently does not in this repo) before attempting a build; use the checklist in the session notes to drive that verification.

---

**Bottom line:** treat this as a **prototype with strong components but no integrated product path yet**. The highest-value work is integration and truthful status reporting, not more feature scaffolding.

---

## Appendix — Remediation progress (2026-11-21)

The following items from this review have been addressed in the working tree:

- **Rust compile/test health:** `angnative-services-core` and `angnative-host-ffi` test targets now compile; all their tests pass. The four failing `host-view` `op_emitter` tests were fixed at the implementation level (flattened nodes now keep a parent link for descendant resolution; recycled views are genuinely released and reused), plus one bad test expectation corrected.
- **FFI correctness:** `last_error` is no longer consumed on read; layout request structs are serializable; FFI tests no longer leak `CString`s.
- **Services:** conflict resolution is now persisted and observable (`resolved_conflicts()`); integration tests assert the real capability↔diagnostics boundary; `crypto` feature compiles and rejects empty nonces; docs now state it is obfuscation, not authenticated encryption.
- **CI:** `.github/workflows/ci.yml` now has real `test`, `rust`, and `size` jobs, uses `pnpm/action-setup`, and no longer calls a nonexistent script. A `compile-fixtures.mjs` generator was added for the size gate. The README build badge is now a live workflow badge.
- **Size gate:** `check-rust-size.mjs` finds hashed rlib artifacts, supports `--host`, and measures 3 host crates well under the 1 MB budget. `host-engine` still cannot be built on this Windows host (rquickjs-sys needs elevated `patch.exe`); CI on Ubuntu is expected to build it.
- **Vertical slice (TS):** `AngNativeRendererFactory2` is now provided in DI; a default flush handler queues batches and calls an optional host dispatcher; zoneless bootstrap now uses `provideZonelessChangeDetection()`; the demo uses `(click)`; native events can be routed by `(nodeId, eventName)` via `dispatchNativeEvent()`.
- **Web adapter:** new `@angnative/web` package with a DOM backend, renderer, bootstrap, primitives and passing tests. `pnpm start` now builds and serves a real browser demo (verified HTTP 200).
- **Desktop shell:** `angnative-desktop` now compiles (wgpu 0.20 / winit 0.29), has a real `DemoApp::new()`, per-rectangle instance rendering, a retained local view tree that consumes ops, and 11 passing tests. It is now a workspace member.
- **Fixtures/T4–T5:** all fixtures migrated to the supported expectation keys, harness asserts them, 9/9 fixtures now have actionable assertions, and `FIXTURES.md` documents the format.
- **Docs/hygiene:** Taffy 0.15.1 / Angular 22+ / API 34 corrected across docs; `LICENSE` added; zoneless and zero-copy claims reconciled with the code.
- **Verification after remediation:** `pnpm test` → **16 files / 208 tests passed**; Rust (host-core/layout/view/ffi/services/desktop) → **100 tests passed**; `pnpm start` web demo serves HTTP 200.

Still open (not addressed here): iOS shell (Mac verification scheduled 2026-11-21), FFI panic safety/`Send`+`Sync` unsoundness, zero-copy transport, binary protocol, and the committed native binary artifacts.

### Update 2026-11-21 (late): Android JNI bridge + shadow tree

- **Android JNI (was "40%") is now wired end-to-end at source level:** `jni_bridge.cpp` includes the real C header, registers all 8 `PlatformOp` callbacks (no `nullptr`), holds a real JNI global ref in a heap `ViewFactoryContext`, frees every Rust-returned string with `angnative_host_free_string`, and builds the layout request from `angnative_host_snapshot()` instead of an empty literal. New `NativeViewFactory.kt` materializes Rust ops into real `View`/`TextView`/`Button` hierarchy; `JniBridge.onNativePress` dispatches a Rust batch; `AngNativeView.kt` no longer hand-builds UI content. CMake header/lib paths corrected (6-level repo-root walk). Gradle/device verification was **not possible** on this Windows host (no NDK/SDK); JNI signatures were matched by inspection only.
- **FFI shadow tree fix:** `Host::process_batch_json` now also calls `engine.run_batch()`, so `angnative_host_node_count` / `angnative_host_snapshot` reflect the processed batch (regression test added). Android's node-count log is now meaningful.
- **AOT regression repaired:** teammate's `renderer.ts` rewrite had broken 10 JS tests and typechecking; legacy `createElement(parent, tag)`, JSON-safe `listenerId`, `dispatchEvent`/`dispatchNativeEvent`, public `data`, and Angular-22-safe imports were restored while keeping the AOT path. `maybeFlush()` is now a no-op so batches are flushed by `RendererFactory2.end()`.
- **Verified after these changes:** `pnpm test` → 16 files / **208 tests passed**; `cargo test --workspace --exclude angnative-host-engine` → **101 tests passed**; `cargo check/test --manifest-path crates/angnative-desktop/Cargo.toml` → 11 passed; `pnpm start` web demo HTTP 200.

### Update 2026-11-21 (evening): Android SDK installed, APK built

- Installed Android command-line tools, SDK platforms 34/36, build-tools 34/37, NDK 27.2.12479018, CMake 3.22.1 under `C:\Android\Sdk`, plus Temurin JDK 17 (Gradle 8.2 + AGP 8.2 cannot run on JDK 25 — Java 25 requires Gradle 9.1+). Added `platforms/android/local.properties`, `scripts/build-android-local.ps1`, `scripts/build-android-native.ps1`, and aligned `compileSdk` to 34 / build-tools 34.0.0.
- Rebuilt `angnative-host-ffi` release libs for `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android` and staged them into `jniLibs/<abi>/`.
- `./gradlew assembleDebug` → **BUILD SUCCESSFUL** (39 tasks, Kotlin + CMake C++ for all three ABIs). APK: `platforms/android/app/build/outputs/apk/debug/app-debug.apk`, 21.88 MB, containing `libangnative.so` and `libangnative_host_ffi.so` for all 3 ABIs.
- Remaining: on-device/emulator click verification (no emulator image installed here), and the JS-runtime wiring needed for a true Angular listener round trip.

### Update 2026-11-21 (night): Android app RUNNING and click round trip verified

- Installed emulator 37.3.10 + `system-images;android-34;google_apis;x86_64`. `avdmanager` failed with `Range [0, 0 + -1) out of bounds for length 256`, so the AVD (`AngNative34`) was created manually with a `config.ini` pointing at the API 34 image. `scripts/run-android-emulator.ps1` boots it, installs the APK, launches `MainActivity`, and captures screenshot + logcat.
- First launch exposed two real bugs, both fixed: (1) `NativeViewFactory.onNativeSetProps` used `optString("text")`, which returns the literal `"null"` for JSON null and overwrote `SetText` values — now guarded with `isNull`; (2) the demo batch had no explicit `height` styles, and intrinsic text measurement in Rust returns 0, so every node collapsed to 15×15 px — explicit heights were added to the demo batch.
- Verified on emulator: Rust host created, 8 nodes in shadow tree, all `PlatformOp`s delivered through JNI into real Android views, Taffy layout applied as frames, and tapping "Click Me!" produced `onNativePress(5) → SetText(node 6, "Hello World!") → visible UI update`. Screenshots: `docs/assets/android-demo.png`, `docs/assets/android-demo-clicked.png`.
- The emulator is left running. Full Angular listener round trip still requires wiring the embedded JS runtime on Android.

### Update 2026-11-21 (final): Desktop and Web runtime verification

- **Desktop:** first live `cargo run --manifest-path crates/angnative-desktop/Cargo.toml` panicked in wgpu because the WGSL vertex shader indexed a const array with `vertex_index` ("expression may only be indexed by a constant"). Replaced with arithmetic corner derivation (`(i & 1)*2-1`, `((i>>1)&1)*2-1`); the app now opens and holds a window titled "AngNative Desktop" with no panic.
- **Web:** Playwright/Chromium verification initially found two real bugs: `WebRenderer.createElement` rejected Angular component selectors such as `<demo-counter>` (fixed by registering unknown tags as `Custom` NativeElements in the DOM backend), and kebab-case `[style.flex-direction]` was rejected by the camelCase style table (fixed by normalizing in `WebRenderer.setStyle`). A stale `serve-web.mjs` process on port 4173 also caused an old bundle to be served once; it was killed. Final run: zero console errors, 11 `na-*` nodes rendered, `+` click changed `Count: 0 → 1` and `even → odd`. Screenshots: `docs/assets/web-demo.png`, `docs/assets/web-demo-clicked.png`.
- **Regression status after all fixes:** `pnpm test` → 16 files / 208 tests passed; `cargo test --workspace --exclude angnative-host-engine` → 101 tests passed; Android APK builds with both toolchain profiles and runs on the emulator; desktop window opens; web demo runs in a real browser.
