# AngNative Android Shell (P3.1)

Android platform shell for AngNative. Renders native `android.view.View`
hierarchy via JNI bridge to the Rust host (`angnative-host-ffi` C ABI).

## Project Structure

```
platforms/android/
├── build.gradle.kts              # Root Gradle build
├── settings.gradle.kts           # Settings
├── gradle.properties             # Gradle properties
├── local.properties              # SDK/NDK paths (create locally)
├── gradle/wrapper/               # Gradle wrapper
├── app/                          # Android app module
│   ├── build.gradle.kts
│   ├── src/main/
│   │   ├── AndroidManifest.xml
│   │   ├── java/com/angnative/app/
│   │   │   ├── MainActivity.kt      # Entry point
│   │   │   ├── AngNativeView.kt     # Custom ViewGroup
│   │   │   ├── JniBridge.kt         # JNI → Rust FFI bridge
│   │   │   └── AngNativeApp.kt      # Bootstrap
│   │   ├── cpp/
│   │   │   ├── CMakeLists.txt       # CMake build
│   │   │   └── jni_bridge.cpp       # JNI C++ wrapper
│   │   └── res/
│   │       ├── values/
│   │       │   ├── strings.xml
│   │       │   ├── styles.xml
│   │       │   └── colors.xml
│   │       └── drawable/
│   └── proguard-rules.pro
```

## Element → Android View Mapping

| AngNative Element | Android View |
|-------------------|--------------|
| `na-view` | `ViewGroup` |
| `na-text` | `TextView` |
| `na-image` | `ImageView` |
| `na-scroll-view` | `ScrollView` / `NestedScrollView` |
| `na-list` | `RecyclerView` (recycled) |
| `na-text-input` | `EditText` |
| `na-pressable` | `View` + touch listener |
| `na-switch` | `Switch` |
| `na-activity-indicator` | `ProgressBar` |
| `na-slot` | `ViewGroup` container |

## Build

Requires Android SDK + NDK:

```bash
cd platforms/android

# Set SDK/NDK paths in local.properties
echo "sdk.dir=/path/to/android-sdk" > local.properties
echo "ndk.dir=/path/to/android-ndk" >> local.properties

# Build debug APK
./gradlew assembleDebug

# Install on device/emulator
./gradlew installDebug
```

## JNI Bridge

The Kotlin `JniBridge` loads `libangnative.so` (built by `cargo` for Android
targets) and calls the `angnative-host-ffi` C ABI:

- `angnative_host_create` / `angnative_host_destroy` — host lifecycle
- `angnative_host_process_batch` — command protocol (JSON)
- `angnative_host_run_layout` — Taffy layout
- `angnative_host_last_error` / `angnative_host_snapshot` — diagnostics

## Requirements

- Android SDK API 24+ (minSdk 24)
- Android NDK (for Rust native library)
- `libangnative.so` built for target ABI (arm64-v8a, armeabi-v7a, x86_64)

## Status

✅ Project structure + Gradle config
✅ Kotlin sources (Activity, View, JNI bridge, bootstrap)
✅ C++ JNI wrapper + CMake
✅ Resources (strings, styles, colors)
⏳ Native library build (requires cargo + Android NDK target)
⏳ Device testing (requires Android device/emulator)
