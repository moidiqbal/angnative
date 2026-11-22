# AngNative — Current Status (single source of truth)

**Last verified:** 2026-11-22
**Repo:** `C:\Users\Moid Iqbal\Projects\angnative`
**Branch:** `master`

> This file exists because an older status list claimed that `@angnative/web`,
> the iOS shell and the Windows desktop app "were not created". That list is
> outdated. Below is the verified state with commands anyone can re-run.

---

## 1. `@angnative/web` — EXISTS, TESTED, BROWSER-VERIFIED ✅

**Files** (`packages/web/`): `package.json`, `README.md`, `tsconfig.json`,
`vitest.config.ts`, and `src/{index,bootstrap,renderer,dom-backend,primitives}.ts`
plus `dom-backend.spec.ts` and `web.spec.ts`.

```powershell
Get-ChildItem packages\web\src -File          # shows all files above
pnpm --filter @angnative/web test             # 2 files / 6 tests passed
pnpm test                                     # 16 files / 221 tests passed
```

Browser verification (Playwright/Chromium) — no console errors:
```
BODY: AngNative Demo | Angular running natively — no WebView. | Counter | Count: 0 | - | reset | + | even
NA-TAGS: 11
BODY AFTER CLICK: ... Count: 1 ... odd
```
Screenshots: `docs/assets/web-demo.png`, `docs/assets/web-demo-clicked.png`
Automation: `scripts/verify-web.py` + `scripts/serve-web.mjs` (`pnpm start`)

---

## 2. iOS shell — CODE COMPLETE, MAC TEST PENDING ⚠️

**Files** (`platforms/ios/`): `Package.swift`, `Sources/AngNativeHost/SwiftFFI.swift`,
`Sources/AngNativeApp/{UIViewFactory,DemoViewController}.swift`,
`Sources/AngNativeDemo/main.swift`, `Tests/AngNativeTests/AngNativeTests.swift`.

Implementation: Swift 6 SwiftPM package, UInt64 C-ABI signatures, `free_string`
on every Rust string, 8 UIKit view-factory callbacks, layout + click → Rust batch,
unit tests. `SwiftFFI.swift` now also binds the five unified runtime-bridge C ABI
functions and exposes `NativeRuntimeBridge` (main-thread owner, borrowed host,
bundle load, event dispatch). `Package.swift` and `build-ios-native.sh` link the
unified `libangnative_runtime_bridge.a`, which bundles host-ffi, host-engine and
QuickJS; the old host-only archive is not linked.
`python scripts/check-ffi-declarations.py` statically verifies the 14 Swift
bindings, their runtime-bridge signatures and the build wiring.

**Honest status:** no Mac here → Swift compile, `swift test`, Simulator/device
run and the runtime-bridge smoke test are **NOT performed**. First action on the
Mac: `python scripts/check-ffi-declarations.py`, then
`./scripts/build-ios-native.sh simulator` and
`cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost`.
See `docs/IOS-BUILD.md` for the full runtime-bridge smoke checklist.

---

## 3. Windows desktop app — COMPILES, TESTED, WINDOW OPENS ✅

Cargo IS installed here: `C:\Users\Moid Iqbal\.cargo\bin\cargo.exe` (nightly 2026-11-14).

```powershell
cargo test --manifest-path crates/angnative-desktop/Cargo.toml   # 11 passed
cargo run  --manifest-path crates/angnative-desktop/Cargo.toml   # window opens
```
Live run opens a window titled **"AngNative Desktop"**. A WGSL validation panic
(`positions[vertex_index]` const-array dynamic index) was found and fixed in
`crates/angnative-desktop/src/renderer.rs`.

---

## 4. Android shell — BUILDS + RUNS ON EMULATOR ✅

- Two verified toolchain profiles: `scripts/build-android-stack.ps1 -Stack latest|legacy`
  - latest: Gradle 9.9.2 / AGP 9.5.2 / **JDK 25** → BUILD SUCCESSFUL
  - legacy: Gradle 8.2 / AGP 8.2 / JDK 17 → BUILD SUCCESSFUL
- Emulator AVD `AngNative34` (API 34 x86_64), script `scripts/run-android-emulator.ps1`
- APK runs; UI is built from Rust `PlatformOp`s; click round trip verified
  (`onNativePress → SetText → "Hello World!"`)
- Screenshots: `docs/assets/android-demo.png`, `docs/assets/android-demo-clicked.png`
- Wrong-thread JNI guard: `connectedDebugAndroidTest` passed 2/2 on `emulator-5554` (Android 14, x86_64) and 2/2 on OnePlus Open `b6640c8f` (`CPH2551`, Android 16, arm64-v8a); evidence in `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md` and `docs/evidence/ANDROID-THREAD-AFFINITY-ONEPLUS.md`

---

## Windows `cargo test --workspace` note

`rquickjs-sys` calls `patch -p1 -f` in its build script. The `patch.exe` that
appears first on PATH on this machine (a WinGet link) has a manifest requiring
elevation (error 740). Git's copy does not. Workaround:

```powershell
$env:PATH = "C:\Program Files\Git\usr\bin;$env:PATH"
cargo build --workspace
cargo test --workspace   # 135 tests pass, 10 ignored; includes host-engine and desktop (11)
```

## Verification summary

| Item | Status |
|---|---|
| Rust core (all workspace crates incl. host-engine + runtime-bridge + desktop) | ✅ **135 tests pass** (with Git `patch.exe` on PATH — see note above) |
| JS/TS (16 spec files) | ✅ 221 tests pass |
| Android JNI + emulator/physical run | ✅ verified: wrong-thread JNI instrumentation passed 2/2 on Android 14 emulator and OnePlus Open (Android 16, arm64-v8a) |
| Desktop window | ✅ verified |
| Web adapter (browser) | ✅ verified |
| iOS shell | ⚠️ Swift runtime-bridge bindings + unified staticlib wiring statically verified; Swift compile / Simulator / device run NOT verified (no Mac/Xcode here). Runbook in `docs/IOS-BUILD.md`. |
| Native event bridge + runtime bridge | ✅ `angnative-runtime-bridge` owns Host + QuickJS; single event path; 9 ignored Angular acceptance tests pass (bootstrap + click round trip + forms/ngModel/router/animations load + animations engine/http/router app/i18n) once `dist/aot` bundles exist, plus 1 third-party RxJS spot-check via `pnpm bundle:third-party`; on-device shell loading still pending (`docs/EVENT-BRIDGE.md`) |
| Angular HttpClient DI (`NG0201`) in QuickJS | ✅ verified by ignored acceptance test `angular_http_platform_loads_in_quickjs`: `provideHttpClient()` + `INJECTOR_SCOPE: 'root'` resolve `HttpClient`, and `get('/api/x')` delivers the native fetch body (`hello-from-native|GET`) plus `HttpResponse.status === 200`. Exact root cause and commands in `docs/architecture/ARCHITECTURE-VERIFIED-2026-11-22.md`. |
| Template-driven `ngModel` in QuickJS | ✅ verified by ignored acceptance test `angular_template_driven_forms_round_trip_in_quickjs`: AOT `FormsAppComponent` uses `FormsModule`/`ngModel` on `<input>`; JS model changes emit native `SetProperty`/text mutations, and a Rust-dispatched native `input` event updates the model. `<input>` now maps to the `na-text-input` primitive; `navigator.userAgent` shim added for Angular's DefaultValueAccessor. |
| AOT i18n markers in QuickJS | ✅ verified by ignored acceptance test `angular_aot_i18n_translated_text_in_native_tree`: `tsconfig.aot.i18n.json` compiles `<na-text i18n>Hello {{ name }}!</na-text>` to `$localize` + `ɵɵi18n`/`ɵɵi18nExp`; a registered translation renders `Assalam-o-Alaikum AngNative!` in the native tree. Root causes: `$localize` placeholder self-substitution loop and missing `Node` DOM constants. |
| Angular Animations engine in QuickJS | ✅ verified by ignored acceptance test `angular_animation_engine_interpolates_native_styles`: `pnpm compile:aot:animations && pnpm bundle:framework` builds the AOT `fade` trigger; the real `AnimationEngine` over `AngNativeRendererFactory2` emits a native `SetStyle opacity: 0.5` midpoint between lifecycle `start,done`, ending at native `opacity: 1`. Native player uses three deterministic samples (`0 / 0.5 / 1`) via flat numeric-id registries — real-time easing fidelity is **not verified**. Root causes: null root renderer host, string `nodeType`, missing `element.style` mirror, orphan-element check, and zero-keyframe `animate('100ms')`. |
| Third-party library spot-check (RxJS 7.8.2) | ✅ verified in embedded QuickJS (native JS path) | `pnpm bundle:third-party` + ignored test `third_party_rxjs_runs_unchanged_in_quickjs`: unmodified RxJS `of(1,2,3).pipe(map(x => x*2))` yields `[2,4,6]` with `complete`, no error. Single DOM-free library only — no DOM-heavy UI library or third-party app was run unchanged; see `docs/COMPATIBILITY.md` §2b. |
| Android cross-thread bridge safety | ✅ verified on Android emulator and physical OnePlus Open | Fixed the release `panic = "abort"` defect: C ABI entry points reject wrong-thread calls with a sentinel before any `assert_owner_thread()` panic can occur (`test_c_abi_dispatch_from_non_owner_thread_returns_sentinel_without_panicking` + `test_thread_affinity` pass). Added `RuntimeBridgeThreadAffinityTest.kt`; `connectedDebugAndroidTest` passed 2/2 on `emulator-5554` (Android 14, x86_64) after rebuilding stale `jniLibs`, and 2/2 on OnePlus Open `b6640c8f` (`CPH2551`, Android 16, arm64-v8a). Evidence: `docs/evidence/ANDROID-THREAD-AFFINITY-EMULATOR.md` and `docs/evidence/ANDROID-THREAD-AFFINITY-ONEPLUS.md`. UI-thread marshalling and a native tap into a live Angular bridge remain separate open items. |
| Production AOT (`pnpm run compile:aot`, `angnative build`) | ✅ real Angular `ngc` (not shadowed 0.1.0 binary), Ivy definitions verified in `dist/aot/`, esbuild bundle produced |

Full history: `docs/PROJECT-REVIEW-2026-11-20.md`
Roadmap: `docs/ROADMAP.md`
