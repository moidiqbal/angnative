# AngNative — Native Charts Showcase (Phase 2 ecosystem)

Standalone Android project demonstrating an **ECharts-style chart suite drawn
entirely with native Android `Canvas`**. There is no WebView, no Chromium, no
DOM and no JavaScript chart library in this app.

> **Honesty:** Apache ECharts and ng2-charts are DOM / browser-canvas
> JavaScript libraries. They cannot execute unchanged on AngNative's native
> renderer. This showcase is a **native reimplementation** of common chart
> types (bar, line, donut), inspired by that ecosystem's visual language.

## What is included

| Chart | Native drawing primitives |
|---|---|
| Bar | `drawRoundRect` with `LinearGradient`, `drawLine` gridlines, `drawText` axis labels |
| Line | Two `Path`s (trend line + gradient area fill), `drawCircle` data points |
| Donut | Annular sector `Path`s (`addArc` + `arcTo`), `drawRoundRect` legend, centre summary |

All three support **tap interaction**: tapping a bar, point, slice or legend row
opens a value tooltip drawn as a native rounded rectangle.

## Project layout

```
showcases/ecosystem-charts/
├── app/src/main/java/com/angnative/app/
│   ├── MainActivity.kt        # UI shell + chart tabs, initializes Rust host
│   ├── ChartData.kt           # Kotlin-generated data + honesty note
│   ├── NativeChartView.kt     # shared View: paints, grid, tooltip, touch
│   ├── BarChartView.kt        # native bar chart
│   ├── LineChartView.kt       # native line/area chart
│   ├── PieChartView.kt        # native donut chart + legend
│   ├── JniBridge.kt           # unchanged Rust FFI bridge from template
│   ├── NativeViewFactory.kt   # unchanged PlatformOp -> View factory
│   ├── AngNativeView.kt       # unchanged native container
│   └── AngNativeApp.kt        # unchanged bootstrap
├── app/src/main/cpp/          # unchanged C++ JNI bridge + CMake
└── app/src/main/jniLibs/      # prebuilt Rust host FFI libraries
```

The Rust `PlatformOps` JNI bridge is kept intact: `MainActivity` still creates
a host and processes a command batch through it. The chart pixels are painted
by the custom Views above — they do not depend on the bridge.

## Build and run

```powershell
$env:ANDROID_HOME = "C:\Android\Sdk"
$env:JAVA_HOME = "C:\Program Files\Java\jdk-25.0.5.1"
.\gradlew.bat assembleDebug --no-daemon
adb install -r app\build\outputs\apk\debug\app-debug.apk
adb shell monkey -p com.angnative.demo.ecosystem.charts 1
```

Application id: `com.angnative.demo.ecosystem.charts`.

## Verification

The compatibility report and staged APK live in
`dist/showcases/ecosystem-charts/`. Zero-WebView check:

```powershell
adb shell dumpsys meminfo com.angnative.demo.ecosystem.charts | Select-String WebViews
# WebViews: 0
```
