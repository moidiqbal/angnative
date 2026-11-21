# AngNative Ecosystem Showcase — Native Virtualized Data Grid

**Native AG Grid–compatible equivalent for AngNative.** Zero WebView.

## ⚠️ Honesty / verdict

**Native reimplementation required — this is NOT unchanged AG Grid compatibility.**

[AG Grid](https://www.ag-grid.com/) (`ag-grid-community` / `ag-grid-angular`) is a
DOM + CSS + JavaScript library. It needs an HTML DOM, browser layout engine and
a JS runtime, none of which exist in AngNative's native Android renderer. The
AG Grid npm packages are therefore **not bundled** and **not executed** in this
showcase. What you see is a purpose-built **native equivalent** that reproduces
the essential behaviour of a data grid: 10,000 rows, fixed row height, a
virtualized viewport, ViewHolder recycling, stable row IDs and aligned column
headers.

There is no WebView, no Capacitor, no embedded browser and no DOM anywhere in
this project. The only visible UI is `android.view.View` / `ViewGroup` and a
real `androidx.recyclerview.widget.RecyclerView`.

## What is implemented

| AG Grid concept | Native equivalent here |
|---|---|
| Row data | 10,000 generated `Employee` rows (id, name, department, salary) |
| Row virtualization | `RecyclerView` + `LinearLayoutManager` + `Adapter` + `ViewHolder` |
| `getRowId()` | `setHasStableIds(true)` / `getItemId()` = employee id |
| Column definitions | Shared `GridRowView` column weights for header + rows |
| Row model recycling | `onViewRecycled()` increments a live on-screen counter |
| Virtual viewport position | `OnScrollListener` → "TOP POSITION" stat tile |
| Row height | Fixed 56 dp → `setHasFixedSize(true)` |
| Sorting / filtering / editing | **not implemented** (see limitations) |

The header shows a live **ROWS / VIEWS RECYCLED / TOP POSITION** strip. After
scrolling a few hundred rows the recycle counter climbs into the hundreds while
only ~10–15 row views exist — making virtualization directly observable.

## Rust host bridge (kept intact)

All bridge code from the verified `showcases/material/` template is unchanged:

- `JniBridge.kt` / `jni_bridge.cpp` — JNI → `angnative-host-ffi` C ABI
- `NativeViewFactory.kt` — Rust `PlatformOp` → native `View` materialization
- `AngNativeView.kt`, `AngNativeApp.kt`, `CMakeLists.txt`
- prebuilt `libangnative_host_ffi.{a,so}` in `app/src/main/jniLibs/{abi}/`

`MainActivity` still initializes the host and dispatches a small command batch
through `angnative_host_process_batch`; the resulting live status ("Rust host
bridge: ONLINE • shadow nodes: 2 • WebViews: 0") is shown in the header.

**Package naming note:** the installed application id is
`com.angnative.demo.ecosystem.grid`, while the Kotlin/JVM package remains
`com.angnative.app`. That is deliberate: the exported JNI symbol names in
`jni_bridge.cpp` are `Java_com_angnative_app_JniBridge_*`, so renaming the JVM
package would break the Rust bridge. `applicationId` is the package id visible
to Android/`adb`.

## Build

```powershell
# SDK + toolchain
$env:JAVA_HOME = "C:\Program Files\Java\jdk-25.0.5.1"
$env:ANDROID_HOME = "C:\Android\Sdk"
# local.properties: sdk.dir=C:/Android/Sdk

.\gradlew.bat assembleDebug --no-daemon
```

Toolchain: Gradle 9.9.2, AGP 9.5.2, JDK 25, NDK 27.2.12479018,
compileSdk 36, targetSdk 34, minSdk 24.

APK: `app/build/outputs/apk/debug/app-debug.apk`
Published copy: `dist/showcases/ecosystem-grid/app-debug.apk`

## Verification

See `dist/showcases/ecosystem-grid/compatibility-report.md` for the exact
commands, outputs, APK size, WebView count and screenshot path.

## Limitations

- Native equivalent, not AG Grid source compatibility; no npm/AG Grid runtime.
- No sorting, filtering, column resizing, cell editing, grouping or pinning.
- Data is generated in Kotlin, not bound from an Angular/AG Grid data source.
- No performance benchmark suite yet (visual verification only).
