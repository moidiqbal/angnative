# @angnative/native-ui-core

Reusable Android library (source distribution) that turns the eight Rust
`PlatformOp` callbacks emitted by `angnative-host-ffi` into real Android Views.

This module is the Phase 3 consolidation of eight near-identical
`NativeViewFactory.kt` copies that were embedded in the showcase apps. It is
**additive**: no showcase has been modified.

## What it does / does not do

- **Does** own the node registry (`nodeId -> View`), view creation, geometry,
  style application, tree mutation (`append`/`insert`/`remove`/`destroy`),
  root attachment and layout-result application.
- **Does** preserve the exact eight JNI callback names and signatures used by
  the existing `JniBridge.kt` / `jni_bridge.cpp` in each showcase.
- **Does** support theme overrides and a pluggable view-creation lambda, so a
  showcase can brand individual widgets. `DesignTokens.MATERIAL_3`, `PRIMENG`,
  `NG_ZORRO`, `TAIGA` and `SPARTAN` are real `NativeUiTheme` presets that imitate
  those libraries' visual languages on native widgets. They are native theme
  values, NOT the third-party libraries themselves.
- **Does NOT** contain a WebView, Capacitor, an Angular/JS runtime, or any
  layout engine of its own. Geometry comes from Rust layout results.
- **Does NOT** contain Rust `.a`/`.so` libraries or JNI glue. The consuming app
  must supply `libangnative.so` (built by the host-ffi package) and its own
  `JniBridge` / C++ shim, and must register the callbacks with the Rust host.

## Package contents

```
package.json                     npm metadata + build/test scripts
scripts/gradle.mjs               cross-platform wrapper launcher
README.md                        this file
android/                         standalone Gradle Android Library build
  settings.gradle.kts, build.gradle.kts, gradle.properties, local.properties
  gradlew, gradlew.bat, gradle/wrapper/*
  native-ui-core/                the library module
    src/main/AndroidManifest.xml
    src/main/java/com/angnative/ui/core/
      NativeViewFactory.kt       all eight callbacks + registry + styling
      NativeUiTheme.kt           overridable default colors/sizes
      DesignTokens.kt            native theme presets: Material 3, PrimeNG, NG-ZORRO, Taiga, Spartan
      ViewTypes.kt               pure view_type -> widget-kind mapping
      StyleCodec.kt              pure-JVM dimension/color/text-size parsers
      PlatformOp.kt              typed ops + optional JSON replay decoder
    src/test/java/.../StyleCodecTest.kt   plain JUnit tests
```

## Consuming from a showcase

### Option A — Gradle project dependency (preferred)

In the showcase's `settings.gradle.kts`:

```kotlin
include(":native-ui-core")
project(":native-ui-core").projectDir =
  file("../../packages/native-ui-core/android/native-ui-core")
```

Then in the app module's `build.gradle.kts`:

```kotlin
dependencies {
  implementation(project(":native-ui-core"))
}
```

The showcase still applies AGP (9.5.2) in its own root build, so the library's
`plugins { id("com.android.library") }` resolves from the same classpath.

### Option B — copy the source

Copy the five Kotlin files from
`android/native-ui-core/src/main/java/com/angnative/ui/core/` into the app's
source set. They have no third-party runtime dependencies beyond Android and
`org.json` (part of the Android platform).

### Wiring the JNI bridge

Keep the existing `JniBridge` (it is what the C++ `GetMethodID` calls resolve
against) and delegate to the library factory. The signatures must stay exactly
as below — `(JLjava/lang/String;)V`, `(JFFFF)V`, `(JJ)V`, `(JJI)V`, `(J)V`:

```kotlin
import com.angnative.ui.core.NativeViewFactory

class JniBridge(private val context: Context) {
  var viewFactory: NativeViewFactory? = null

  fun install(root: ViewGroup, onPress: (Long) -> Unit) {
    viewFactory = NativeViewFactory(
      context = context,
      rootContainer = root,
      onPress = onPress,
      theme = NativeUiTheme.DEFAULT,                 // or your palette
      viewCreator = { id, type ->                    // optional
        if (type == "Button") brandedButton(id)
        else null                                    // null => stock view
      }
    )
    // JniBridge callbacks then forward one-to-one:
    // onNativeCreateView(nodeId, viewType) -> viewFactory?.onNativeCreateView(...)
    // ...and so on for all eight methods.
  }
}
```

`NativeViewFactory` exposes the complete set:

| Rust `PlatformOp` | Kotlin callback | JNI signature |
| --- | --- | --- |
| `Create { node_id, view_type }` | `onNativeCreateView(nodeId, viewType)` | `(JLjava/lang/String;)V` |
| `SetFrame { node_id, x, y, width, height }` | `onNativeSetFrame(nodeId, x, y, width, height)` | `(JFFFF)V` |
| `SetProps { node_id, props }` | `onNativeSetProps(nodeId, propsJson)` | `(JLjava/lang/String;)V` |
| `SetText { node_id, text }` | `onNativeSetText(nodeId, text)` | `(JLjava/lang/String;)V` |
| `Append { parent_id, child_id }` | `onNativeAppendChild(parentId, childId)` | `(JJ)V` |
| `Insert { parent_id, child_id, index }` | `onNativeInsertChild(parentId, childId, index)` | `(JJI)V` |
| `Remove { parent_id, child_id }` | `onNativeRemoveChild(parentId, childId)` | `(JJ)V` |
| `Destroy { node_id }` | `onNativeDestroyView(nodeId)` | `(J)V` |

Also available: `setRootNode`, `attachRoot`, `applyLayoutResult(layoutJson)`
(`{"nodes":[{"node_id","x","y","width","height"}]}`), `clear`, `getView`,
`viewCount`, `applyStyle`, `defaultViewForType`.

For hosts that replay serialized ops, `PlatformOp.fromJson` decodes one serde
externally-tagged op (`{"Create":{"node_id":1,"view_type":"Text"}}`) or a flat
`{"op":"Create",...}` form; `PlatformOp.decodeBatch` decodes a JSON array or an
`{"ops":[...]}` / `{"operations":[...]}` wrapper. `factory.apply(op)` dispatches
it. The live FFI path does not need this: callbacks arrive one by one.

## Props JSON shape (SetProps)

`propsJson` is Rust's serialized `ViewProps`:

```json
{
  "text": "optional text",
  "styles": { "color": "#0F172A", "fontSize": "15px" },
  "properties": { "fontWeight": "bold" }
}
```

`styles` and `properties` are both applied with `applyStyle`; a non-null string
`text` also overwrites a TextView's text. Non-string `properties` values are
stringified by `optString` (the historical behavior).

## Supported style keys

| Key | Effect |
| --- | --- |
| `width`, `height` | LayoutParams size. `px`/bare numbers are device px; `dp` scales with density; `%` resolves against screen width; `auto` -> `WRAP_CONTENT`. |
| `padding` | Uniform padding on all sides. |
| `paddingLeft`, `paddingTop`, `paddingRight`, `paddingBottom` | Per-side padding. |
| `color` | TextView text color. |
| `backgroundColor` | View background color. |
| `fontSize` | TextView sp size. `px` and bare numbers are treated as sp; `dp` converts via density/fontScale. |
| `fontWeight` | `bold`/`700`/`800`/`900` -> bold; otherwise normal. |
| `opacity` | Alpha in [0, 1]. |
| `visibility` | `hidden`/`gone` -> `GONE`; anything else -> `VISIBLE`. |

Colors accept `#RGB`, `#ARGB`, `#RRGGBB`, `#AARRGGBB` and `black`, `white`,
`red`, `green`, `blue`, `gray`/`grey`, `transparent`; malformed values fall
back to black. Layout-engine keys such as `flexDirection` or `margin` are
intentionally ignored here — layout is Rust's job.

Supported `view_type` values: `View`, `Custom`, `ScrollView` -> `FrameLayout`;
`Text` -> `TextView`; `TextInput` -> `EditText`; `Button` -> `Button`;
`Image` -> `ImageView`; `Switch` -> `Switch`; `ActivityIndicator` ->
indeterminate `ProgressBar`; anything else -> plain `View`.

## Building and testing locally

Toolchain used and verified in this workspace: Gradle wrapper **9.9.2**, AGP
**9.5.2** (built-in Kotlin support), JDK **25.0.5.1**, Android SDK
**C:/Android/Sdk**, compileSdk **36**, build-tools **37.0.0**, minSdk **24**.

```powershell
cd packages\native-ui-core\android
$env:JAVA_HOME = "C:\Program Files\Java\jdk-25.0.5.1"
$env:ANDROID_HOME = "C:\Android\Sdk"
.\gradlew.bat assembleRelease   # AAR: native-ui-core/build/outputs/aar/native-ui-core-release.aar
.\gradlew.bat test              # JUnit report: native-ui-core/build/reports/tests/test/index.html
```

From the package root, `npm run build` and `npm test` invoke the same tasks via
`scripts/gradle.mjs`. `local.properties` is included with `sdk.dir=C:/Android/Sdk`
for this workspace; in other checkouts set `ANDROID_HOME` or edit that file.

## Known limitations

- Callbacks must be delivered on the main/UI thread. The registry is not
  synchronized; the C++/JNI bridge is responsible for dispatching there.
- The library does not ship JNI glue, the Rust host, `.a`/`.so` artifacts, an
  event loop to Angular, or a layout engine. The host app supplies all of them.
- No instrumentation/UI tests are included; the JUnit suite covers only the
  pure codec and view-type mapping. Real View behavior is exercised by the
  showcase apps (which this package intentionally does not modify).
- Percentage dimensions resolve against the screen width for both axes, as in
  the original factory code; there is no per-axis percentage style key.
- `SetProps` silently ignores malformed JSON rather than surfacing an error;
  the Rust host's `last_error`/response remains the error channel.
- Node ids are `Long` on the Kotlin side (JNI `jlong`) although Rust uses `u64`;
  ids above `Long.MAX_VALUE` would wrap. No showcase uses such ids.
- Unknown style keys and unknown view types degrade gracefully rather than
  throwing; there is no schema validation at runtime.
