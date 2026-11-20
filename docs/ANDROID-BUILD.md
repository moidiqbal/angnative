# AngNative Android Native — Build & Verification Checklist

This checklist ensures 100% confidence that the Android native library builds
and runs correctly. Execute each step in order.

## Prerequisites

| Tool | Version | Install |
|------|---------|---------|
| Rust (rustc/cargo) | nightly-2026-11-14 | https://rustup.rs/ |
| Android NDK | r25+ | Android Studio → SDK Manager |
| Android SDK | API 34 | Android Studio → SDK Manager |
| Gradle | 8.2 | Included (wrapper) |

## Step 1: Install Rust Toolchain

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
source "$HOME/.cargo/env"

# Install nightly toolchain
rustup toolchain install nightly-2026-11-14
rustup default nightly-2026-11-14

# Add Android targets
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64

# Verify
rustc --version
cargo --version
rustup target list --installed
```

Expected output:
```
rustc 1.84.0 (nightly-2026-11-14)
cargo 1.84.0 (nightly-2026-11-14)
aarch64-linux-android (installed)
armv7-linux-androideabi (installed)
x86_64 (installed)
```

## Step 2: Set Up Android NDK

```bash
# Option A: Via Android Studio
#   SDK Manager → SDK Tools → NDK (side by side) → Install

# Option B: Via command line
sdkmanager "ndk;25.1.8937393"

# Set environment variable
export ANDROID_NDK_HOME=$HOME/Android/Sdk/ndk/25.1.8937393

# Verify
ls $ANDROID_NDK_HOME/toolchains/llvm/prebuilt/
```

## Step 3: Build Native Libraries

```bash
cd angnative

# Build for all Android ABIs
bash scripts/build-android-native.sh all

# Or build for a specific ABI:
bash scripts/build-android-native.sh arm64-v8a   # ARM 64-bit (most devices)
bash scripts/build-android-native.sh armeabi-v7a # ARM 32-bit
bash scripts/build-android-native.sh x86_64      # Emulator
```

Expected output:
```
==============================================
  AngNative Android Native Library Build
==============================================
  NDK: /home/user/Android/Sdk/ndk/25.1.8937393
  Rust: rustc 1.84.0 (nightly-2026-11-14)
  Cargo: cargo 1.84.0 (nightly-2026-11-14)

  Targets: aarch64-linux-android armv7-linux-androideabi x86_64

----------------------------------------------
  Building aarch64-linux-android (ABI: arm64-v8a)
----------------------------------------------
  Building angnative-host-ffi for aarch64-linux-android...
     Compiling angnative-host-core v0.1.0
     Compiling angnative-host-layout v0.1.0
     Compiling angnative-host-view v0.1.0
     Compiling angnative-host-ffi v0.1.0
     Finished release [optimized] target(s) in 45.2s
  ✓ aarch64-linux-android: 847 KB -> platforms/android/app/src/main/jniLibs/arm64-v8a/
----------------------------------------------
  ...
==============================================
  Build Complete!
==============================================
```

Verify the static libraries were created:
```bash
ls -la platforms/android/app/src/main/jniLibs/*/
```

Expected:
```
arm64-v8a/libangnative_host_ffi.a
armeabi-v7a/libangnative_host_ffi.a
x86_64/libangnative_host_ffi.a
```

## Step 4: Verify Size Budget (≤1 MB per platform)

```bash
cd angnative
pnpm run check:rust-size
```

Expected output:
```
AngNative Rust core size budget gate (Task R8 / identity D-8)
Budget: 1.0 MB per platform
Targets: x86_64-unknown-linux-gnu, aarch64-apple-darwin, ...

=== Target: aarch64-linux-android ===
  ✓ angnative-host-core: 123 KB (budget: 1.0 MB)
  ✓ angnative-host-layout: 245 KB (budget: 1.0 MB)
  ✓ angnative-host-view: 189 KB (budget: 1.0 MB)
  ✓ angnative-host-engine: 312 KB (budget: 1.0 MB)

✅ SIZE BUDGET MET — Rust core is within 1 MB per platform.
```

## Step 5: Build Android APK

```bash
cd platforms/android

# Set SDK/NDK paths (if not already set)
echo "sdk.dir=$HOME/Android/Sdk" > local.properties
echo "ndk.dir=$ANDROID_NDK_HOME" >> local.properties

# Build debug APK
./gradlew assembleDebug
```

Expected output:
```
BUILD SUCCESSFUL in 2m 34s
21 actionable tasks: 21 executed
```

APK location:
```
platforms/android/app/build/outputs/apk/debug/app-debug.apk
```

## Step 6: Install and Run on Device/Emulator

```bash
# List connected devices
adb devices

# Install APK
./gradlew installDebug

# Or manually:
adb install app/build/outputs/apk/debug/app-debug.apk

# Launch app
adb shell am start -n com.angnative.app/.MainActivity
```

Expected: App launches and shows "AngNative Demo" with counter buttons.

## Step 7: Verify End-to-End

| Check | Command | Expected |
|-------|---------|----------|
| App installs | `adb install app-debug.apk` | Success |
| App launches | `adb shell am start ...` | "AngNative Demo" visible |
| Counter works | Tap "+" button | "Count: 1" displayed |
| No crashes | Logcat | No `Fatal exception` |

View logs:
```bash
adb logcat -s AngNative
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `cargo: command not found` | Install Rust: https://rustup.rs/ |
| `ANDROID_NDK_HOME not set` | Set `export ANDROID_NDK_HOME=/path/to/ndk` |
| Linker not found | Check NDK path includes `toolchains/llvm/prebuilt/` |
| Build fails on `angnative-host-engine` | rquickjs may not support Android yet — use `--no-default-features` |
| APK install fails | Enable USB debugging on device, check `adb devices` |
| App crashes on launch | Check logcat: `adb logcat -s AngNative AndroidRuntime` |

## Complete One-Command Setup

```bash
# Everything in one go (requires Rust + NDK pre-installed)
export ANDROID_NDK_HOME=/path/to/android-ndk
bash scripts/setup-android-native.sh
```

## Files Created/Modified for P3.1

| File | Purpose |
|------|---------|
| `rust-toolchain.toml` | Pinned nightly toolchain + Android targets |
| `scripts/build-android-native.sh` | Build static libs for all Android ABIs |
| `scripts/setup-android-native.sh` | One-command full setup |
| `crates/angnative-host-ffi/Cargo.toml` | cdylib crate type for Android |
| `crates/angnative-host-ffi/include/angnative_host_ffi.h` | C ABI header |
| `crates/angnative-host-ffi/build.rs` | Copy header to include dir |
| `platforms/android/` | Complete Android shell project |
| `docs/R8-SIZE-BUDGET.md` | Size budget measurement procedure |

## Next Steps After Build

1. Run `pnpm run check:rust-size` to verify ≤1 MB budget
2. Install APK on device/emulator
3. Verify counter app renders and responds to input
4. Check logcat for any FFI errors
5. Once Android is verified, proceed to P3.2 (iOS shell)

---

## Two verified toolchain profiles (Windows, 2026-11-21)

Both stacks build `app-debug.apk` successfully on this host. SDK/NDK live in
`C:\Android\Sdk` (NDK 27.2.12479018, CMake 3.22.1, API 34/36, build-tools 34/37).

| Profile | Gradle | AGP | Kotlin | JDK | compileSdk | Build script |
|---|---|---|---|---|---|---|
| `latest` (default) | 9.9.2 | 9.5.2 | built-in (AGP) | 25 | 36 | `pwsh -File scripts/build-android-stack.ps1 -Stack latest` |
| `legacy` | 8.2 | 8.2.0 | KGP 1.9.20 | 17 | 34 | `pwsh -File scripts/build-android-stack.ps1 -Stack legacy` |

The script copies the selected files from `platforms/android/profiles/<stack>/`
into the live Gradle files, sets `JAVA_HOME`, and runs `assembleDebug`.

Notes for the `latest` profile:
- AGP 9+ has built-in Kotlin: `org.jetbrains.kotlin.android` must NOT be applied.
- `kotlinOptions { jvmTarget }` is removed; Kotlin follows `compileOptions`.
- Java 25 requires Gradle 9.1+; Gradle 8.2 cannot run on JDK 25 (use legacy).

Rust host libraries for all three ABIs are rebuilt with
`pwsh -File scripts/build-android-native.ps1` before the Gradle build.
