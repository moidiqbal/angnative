# AngNative — Native Calendar Showcase (Phase 2 ecosystem)

Standalone Android project demonstrating a **genuine native month-view calendar**.
The 7-column grid is a custom `android.view.View` that paints weekday headers,
day numbers, current-day / selected-day highlights and event dots with
`android.graphics.Canvas` + `Paint`. Day selection is native touch hit-testing.
There is no WebView, no browser engine, no DOM and no JavaScript calendar library.

> **Honesty:** FullCalendar is a DOM + JavaScript library. It cannot run
> unchanged on AngNative's native renderer. This showcase is a **native
> reimplementation** of FullCalendar's month-view visual language — not
> unchanged FullCalendar compatibility. No FullCalendar source or npm package is
> bundled. See `app/src/main/java/com/angnative/app/CalendarEvent.kt`.

## What is included

| Feature | Native implementation |
|---|---|
| 7-column month grid | fixed 6-row / 42-cell grid drawn in `MonthGridView.onDraw()` |
| Weekday headers | `Canvas.drawText` from `CalendarData.WEEKDAYS` |
| Current-day highlight | light-blue rounded rectangle + blue number |
| Selected-day highlight | filled indigo circle, white number; blue ring when it is also today |
| Event dots | up to three `drawCircle` dots per day, computed from deterministic events |
| Tap to select | `onTouchEvent` cell hit-testing; adjacent-month days navigate months |
| Prev / next month | `MonthGridView.shiftMonth(±1)` buttons in `MainActivity` |
| Details panel | selected date + that day's events, updated via `onDateSelected` |

Twelve deterministic events per month are generated from seed 42 in
`CalendarEvent.kt` — the same month always yields the same events.

## Project layout

```
showcases/ecosystem-calendar/
├── app/src/main/java/com/angnative/app/
│   ├── MainActivity.kt        # UI shell, Rust host bootstrap, nav + details
│   ├── MonthGridView.kt       # native Canvas/Paint month grid + touch handling
│   ├── CalendarEvent.kt       # deterministic event seed + honesty note
│   ├── JniBridge.kt           # unchanged Rust FFI bridge from template
│   ├── NativeViewFactory.kt   # unchanged PlatformOp -> View factory
│   ├── AngNativeView.kt       # unchanged native container
│   └── AngNativeApp.kt        # unchanged bootstrap
├── app/src/main/cpp/          # unchanged C++ JNI bridge + CMake
└── app/src/main/jniLibs/      # prebuilt Rust host FFI libraries
```

The Rust `PlatformOps` JNI bridge is kept intact: `MainActivity` creates a host,
processes a command batch and destroys the host on teardown. Calendar drawing
does not depend on the bridge.

## Build and run

```powershell
$env:ANDROID_HOME = "C:\Android\Sdk"
$env:JAVA_HOME = "C:\Program Files\Java\jdk-25.0.5.1"
.\gradlew.bat assembleDebug --no-daemon
adb install -r app\build\outputs\apk\debug\app-debug.apk
adb shell am start -n com.angnative.demo.ecosystem.calendar/com.angnative.app.MainActivity
```

Application id: `com.angnative.demo.ecosystem.calendar`; Kotlin/JVM package
stays `com.angnative.app` so the exported JNI symbols still resolve.

## Verification

Staged APK, `source.zip` and `compatibility-report.md` live in
`dist/showcases/ecosystem-calendar/`. Zero-WebView check:

```powershell
adb shell dumpsys meminfo com.angnative.demo.ecosystem.calendar | Select-String WebViews
# WebViews: 0
```
