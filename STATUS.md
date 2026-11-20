# AngNative — Current Status (single source of truth)

**Last verified:** 2026-11-21
**Repo:** `C:\Users\Moid Iqbal\Projects\angnative`
**Branch:** `master` (working tree has uncommitted changes — see `git status`)

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
pnpm test                                     # 16 files / 208 tests passed
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
unit tests. `scripts/build-ios-native.sh` now builds the Rust static lib for the
correct simulator/device triple and runs `swift build` with the linker path.

**Honest status:** no Mac here → `swift build` / Simulator run NOT performed.
First action on the Mac: `./scripts/build-ios-native.sh simulator` then
`cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost`.
See `docs/IOS-BUILD.md` for the full checklist.

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

---

## Windows `cargo test --workspace` note

`rquickjs-sys` calls `patch -p1 -f` in its build script. The `patch.exe` that
appears first on PATH on this machine (a WinGet link) has a manifest requiring
elevation (error 740). Git's copy does not. Workaround:

```powershell
$env:PATH = "C:\Program Files\Git\usr\bin;$env:PATH"
cargo build --workspace
cargo test --workspace   # 108 tests pass, including host-engine (6) and desktop (11)
```

## Verification summary

| Item | Status |
|---|---|
| Rust core (all workspace crates incl. host-engine + desktop) | ✅ **108 tests pass** (with Git `patch.exe` on PATH — see note above) |
| JS/TS (16 spec files) | ✅ 208 tests pass |
| Android JNI + emulator run | ✅ verified |
| Desktop window | ✅ verified |
| Web adapter (browser) | ✅ verified |
| iOS shell | ⚠️ code complete, Mac build pending |
| Native event bridge (FFI + QuickJS API) | ✅ scaffold + tests; full Angular-in-native pending (`docs/EVENT-BRIDGE.md`) |

Full history: `docs/PROJECT-REVIEW-2026-11-20.md`
Roadmap: `docs/ROADMAP.md`
