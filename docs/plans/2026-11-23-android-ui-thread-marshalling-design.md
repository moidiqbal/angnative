# Android UI-thread marshalling and live-tap re-verification — design

**Date:** 2026-11-23
**Status:** Validated with user; ready for implementation

## Problem

Two honest gaps were recorded after the wrong-thread JNI verification:

1. Rust→Kotlin ViewFactory callbacks use JNI `AttachCurrentThread` but do not
   guarantee Android's main looper. Today's production call sites all originate
   on the UI thread, so the guarantee is incidental, not enforced.
2. Native tap → live Angular listener was implemented and emulator-proven in
   commit `98a4f4e`, but several canonical docs still called it open, and no
   physical OnePlus evidence existed for the tap path.

## Decisions

- **Scope:** implement UI-thread marshalling, re-verify the tap on emulator and
  OnePlus Open, retain artifacts, and fix stale docs/CI in the same work.
- **Approach:** marshal once at native entry points in `JniBridge`, not per
  callback. Per-callback latches were rejected in architect review because a
  timeout would let Rust/host state diverge silently from the native view tree.
- **Test seam:** extract a `NativeViewSink` interface from `NativeViewFactory`;
  `JniBridge.viewFactory` becomes `NativeViewSink?`. No open subclass.
- **No C++/Rust/ABI changes.** Global refs, `ScopedEnv`, and owner-thread guards
  are already correct.

## Architecture

`JniBridge` holds a cached `Handler(Looper.getMainLooper())` and one
`runOnMainForNative(...)` helper:

- if `Looper.myLooper() === Looper.getMainLooper()`, run inline;
- otherwise post to the main handler and wait on a single `CountDownLatch`;
- on timeout or captured `Throwable`, throw `IllegalStateException` so callers
  see failure instead of a false success;
- `destroyHost()` calls `removeCallbacksAndMessages(null)`.

Wrap all native entry points that touch the bridge/host: `initHost`,
`loadScript`, `processBatch`, `runLayout`, `dispatchNativeEvent`, `nodeCount`,
`snapshot`, `lastError`, `destroyHost`. Because Rust→Kotlin view callbacks fire
synchronously inside these calls, the eight `onNative*` callbacks and
`buildLayoutRequest` then execute on main. The inline fast path preserves
`EcosystemCoreActivity`'s immediate `findButtonIdByText("+")` after
`loadScript`, and making `initHost()` main-resident keeps Rust's
`owner_thread` equal to main so later UI taps are not rejected.

`buildLayoutRequest` is pure JSON; include it in the audit but do not wrap it
separately. Wording for the event bridge remains **one delivery per event
(canonical JS thread-local path + legacy FFI fallback)**, not “single path”.

## Tests and artifacts

Add to `RuntimeBridgeThreadAffinityTest.kt` (or a focused sibling):

1. `viewFactoryCallbacksFromWorkerEntryAreMarshalledToMainLooper` — create/init
   bridge with `runOnMainSync`, install `RecordingViewSink`, call
   `loadScript(bundle)` from a worker thread. The sink records
   `Looper.myLooper()`, thread name, method name and args. Assert all eight
   callbacks ran on `Looper.getMainLooper()`, no worker name appears, and
   `nodeCount() > 0`.
2. `dispatchNativeEventFromWorkerRunsOnMainAndReachesAngularListener` — call
   `dispatchNativeEvent(nodeId, "click")` from a worker thread; assert
   `invoked > 0` and sink callbacks on main. This also proves the owner-thread
   guard is satisfied because the native call is posted to main.

Tap re-verification on `emulator-5554` and OnePlus `b6640c8f`:

```text
adb devices -l
getprop ro.product.model / ro.build.version.release / ro.product.cpu.abi
install + launch com.angnative.demo.ecosystem.core
uiautomator dump before/after
python scripts\android-tap-coords.py <before.xml>
adb logcat -c; adb shell input tap <coords>
logcat AngNative:I contains "Angular tap node=... -> [1-9]"
UI contains "Count: 1"
screenshot
```

Retain `docs/evidence/android-viewfactory-mainthread-{emulator,oneplus}.html`,
`docs/evidence/ANDROID-VIEWFACTORY-MAINTHREAD.md`,
`docs/assets/angular-real-tap-oneplus-count1.png`, plus logcat and XML dumps.

## Docs/CI corrections

- `docs/EVENT-BRIDGE.md`: live tap is implemented/emulator-proven; update
  shipping-shell, bridge-lifetime, and “not runs natively” paragraphs.
- `docs/UNSUPPORTED-FEATURES.md`: row 18, cross-thread row tail, General row.
- `STATUS.md`: §4, rows 106/110/116.
- `docs/architecture/ARCHITECTURE-VERIFIED-2026-11-22.md`: §3.1, §6, §7, §8
  gap 2, and `nodes=... rc=... plusNode=...` label format.
- `docs/CODE-REVIEW-2026-11-22.md`: resolve line 183 vs 174 contradiction.
- `docs/evidence/ANDROID-THREAD-AFFINITY-ONEPLUS.md`: tap now verified on
  OnePlus by the new evidence, not an open item.
- `.github/workflows/ci.yml` line 190: replace stale regex with a match for
  `Angular in Rust: nodes=[0-9]+ rc=0 plusNode=[0-9]+`.

## Full verification gate

```text
cargo test --workspace
cargo test -p angnative-runtime-bridge -- --ignored
pnpm test
python scripts/check-ffi-declarations.py
connectedDebugAndroidTest on emulator-5554
connectedDebugAndroidTest on b6640c8f
```

Commit and push after the gate passes. No claim of iOS verification.
