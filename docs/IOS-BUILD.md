# AngNative iOS Platform (P3.2)

iOS platform shell for AngNative. Renders native `UIView` hierarchy via
the Rust host (`angnative-host-ffi` C ABI) bridged through Swift.

## Project Structure

```
platforms/ios/
├── Package.swift                     # SwiftPM package manifest
├── Sources/
│   ├── AngNativeHost/
│   │   └── SwiftFFI.swift            # Swift wrapper around legacy host + runtime bridge C ABI
│   ├── AngNativeApp/
│   │   ├── UIViewFactory.swift       # UIKit view factory (UIButton, UILabel, etc.)
│   │   └── DemoViewController.swift  # Demo counter component
│   └── AngNativeDemo/
│       └── main.swift                # App entry point (SwiftUI wrapper)
├── Tests/
│   └── AngNativeTests/
│       └── AngNativeTests.swift      # Unit tests
└── Resources/
```

## Element → iOS View Mapping

| AngNative Element | iOS View |
|-------------------|----------|
| `na-view` | `UIView` |
| `na-text` | `UILabel` |
| `na-image` | `UIImageView` |
| `na-scroll-view` | `UIScrollView` |
| `na-list` | `UIScrollView` container |
| `na-text-input` | `UITextField` |
| `na-pressable` | `UIButton` |
| `na-switch` | `UISwitch` |
| `na-activity-indicator` | `UIActivityIndicatorView` |
| `na-slot` | `UIView` |

## Build

### Prerequisites

- macOS with **Xcode** (not just the command line tools) and an iOS Simulator runtime
- The pinned Rust nightly from `rust-toolchain.toml`; the script adds the iOS
  target (`aarch64-apple-ios-sim`, `aarch64-apple-ios` or `x86_64-apple-ios`)

> `swift build` / `swift test` do **not** work for this package: they compile
> for macOS, where UIKit does not exist. Build through `xcodebuild` with an iOS
> destination; the scripts below do that.

### Build and test (Simulator)

```bash
./scripts/build-ios-native.sh simulator --test
```

This:

1. builds `angnative-runtime-bridge` as a staticlib for the simulator. The
   archive bundles host-ffi, host-engine and QuickJS. `rquickjs-sys` ships no
   pregenerated iOS bindings, so bindgen runs against the iOS SDK using Xcode's
   libclang (`crates/angnative-host-engine/Cargo.toml`, `cfg(target_os = "ios")`);
2. stages `libangnative_runtime_bridge.a` and the C header into
   `platforms/ios/Sources/AngNativeHost/` (gitignored);
3. runs `xcodebuild build-for-testing` for the first available iPhone simulator
   (override with `IOS_SIM_DESTINATION='platform=iOS Simulator,name=…'`) with
   `IPHONEOS_DEPLOYMENT_TARGET=17.0`, matching `Package.swift`;
4. with `--test`, runs the XCTest suite in the Simulator.

### Run the demo app in the Simulator

```bash
./scripts/run-ios-simulator.sh [screenshot.png]
```

SwiftPM executables are bare Mach-O files, so the script wraps
`AngNativeDemo` in a minimal `.app` (Info.plist + ad-hoc signature), installs
and launches it with `simctl`, prints the app log and saves a screenshot. No
Xcode project or Apple Developer account is needed.

### Device build

```bash
./scripts/build-ios-native.sh device
```

Builds the `aarch64-apple-ios` archive and an unsigned Release build of the
package for `generic/platform=iOS`. Installing on a phone still needs signing
(see below).

## Running on Device

### Option 1: xcodebuild (requires Apple Developer ID)

```bash
xcodebuild -scheme AngNativeDemo -destination 'platform=iOS,name=<device-name>' -configuration Release
```

### Option 2: Sideloadly (free Apple ID)

1. Download [Sideloadly](https://sideloadly.io/) on Windows/macOS
2. Connect iPhone via USB
3. Drag the built `.ipa` into Sideloadly
4. Enter Apple ID and click Start
5. Trust the developer certificate on iPhone: Settings → General → VPN & Device Management

## Threading model

The Rust host calls the view-factory callbacks **synchronously** inside
`angnative_host_process_batch` / `run_layout`, on the calling thread. The host
and runtime bridge are thread-affine and must be driven from the main thread,
so `UIKitViewFactory` is `@MainActor` and every callback asserts that with
`MainActor.assumeIsolated` (a call from another thread traps instead of racing
UIKit). The UIKit tree therefore exists as soon as `processBatch` returns.
Dispatching from a background queue is rejected by the C ABI with
`UInt32.max`; iOS has no UI-thread marshalling layer.

## Status (2026-11-28) — verified on a Mac

Evidence: [`docs/evidence/IOS-SIMULATOR.md`](evidence/IOS-SIMULATOR.md).

| Feature | Status |
|---------|--------|
| Rust runtime bridge staticlib for `aarch64-apple-ios-sim` and `aarch64-apple-ios` | ✅ built (bindgen against the iOS SDK) |
| Swift package compile + link for iOS Simulator and iOS device (unsigned) | ✅ `xcodebuild` BUILD SUCCEEDED |
| XCTest suite in the iOS Simulator (8 tests) | ✅ 8 passed, 0 failures |
| Real Angular AOT bundle → QuickJS → Rust → UIKit, native click → Angular listener → `UILabel` | ✅ XCTest `testRuntimeBridgeBootsAngularBundleIntoUIKit` |
| Demo app installed and launched in the Simulator | ✅ screenshot `docs/assets/ios-simulator-demo.png`, `nodeCount=7` |
| Physical iPhone run | ⚠️ not verified (needs signing / a device) |
| Angular bundle as the demo app's UI (instead of the hand-built Rust batch) | ❌ not yet; covered only by XCTest |
| wgpu GPU rendering | ❌ Not applicable (UIKit used instead) |
| Hot reload / App Store distribution | ❌ Not yet |

## Notes

- iOS Simulator is **not** available on Linux/Windows — a physical iPhone or macOS is required
- Free Apple ID allows 7-day signing; paid Apple ID ($99/yr) allows 1-year signing
- Developer Mode must be enabled on iOS: Settings → Privacy & Security → Developer Mode
