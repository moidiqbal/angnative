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

- macOS with Xcode 15+ installed
- Rust toolchain with iOS target: `rustup target add aarch64-apple-ios`
- `Xcode_27.2.zip` (or newer) downloaded from Apple Developer portal

### Build Steps

#### 1. Install Rust iOS target

```bash
rustup target add aarch64-apple-ios
```

#### 2. Build the unified Rust runtime library

```bash
cargo build --release --target aarch64-apple-ios -p angnative-runtime-bridge
```

This produces
`target/aarch64-apple-ios/release/libangnative_runtime_bridge.a`. Prefer
`./scripts/build-ios-native.sh device`, which also stages the archive and the
C header into `platforms/ios/Sources/AngNativeHost/`. Do **not** link the old
`libangnative_host_ffi.a` alone: it does not contain the
`angnative_runtime_bridge_*` symbols used by `NativeRuntimeBridge`. The
unified staticlib bundles host-ffi, host-engine and QuickJS, so iOS links one
archive (configured in `platforms/ios/Package.swift`).

#### 3. Build iOS app

```bash
cd platforms/ios

# Simulator build
./../../scripts/build-ios-native.sh simulator

# Device build
./../../scripts/build-ios-native.sh device
```

Or use `swift build` directly:

```bash
swift build
swift test
```

## Running on Device

### Option 1: SwiftPM / xcodebuild (requires Apple Developer ID)

```bash
# Build and run on connected device
swift run --device <device-udid>

# Or via xcodebuild
xcodebuild -scheme AngNativeDemo -destination 'platform=iOS,name=<device-name>' -configuration Release
```

### Option 2: Sideloadly (free Apple ID)

1. Download [Sideloadly](https://sideloadly.io/) on Windows/macOS
2. Connect iPhone via USB
3. Drag the built `.ipa` into Sideloadly
4. Enter Apple ID and click Start
5. Trust the developer certificate on iPhone: Settings → General → VPN & Device Management

## Runtime bridge smoke test on macOS (NOT executed here)

`SwiftFFI.swift` now binds the unified runtime bridge
(`angnative_runtime_bridge_create`, `get_host`, `load_script`,
`dispatch_event`, `destroy`) and wraps it in `NativeRuntimeBridge`. Bindings
and iOS build wiring are verified **statically** on Windows/macOS without Xcode:

```bash
python scripts/check-ffi-declarations.py
# FFI declaration check OK: 11 JNI methods, 14 Swift bindings (5 runtime bridge),
# 20 header declarations; iOS unified staticlib wiring OK
```

That check proves the Swift symbols/signatures exist in the shared C header and
that `Package.swift` / `build-ios-native.sh` link the unified staticlib. It does
**not** prove `swift build`, `swift test`, or a Simulator/device run passes.

On the Mac, after `./scripts/build-ios-native.sh simulator`, run the existing
Swift tests, then use this manual smoke checklist on the **main thread**:

```bash
# Generate a self-contained bundle first (from the repo root):
pnpm --filter @angnative/core build && pnpm --filter @angnative/testing build &&
pnpm --filter @angnative/platform build
pnpm compile:aot && pnpm bundle:quickjs

cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost
```

In a Swift debug snippet or a temporary XCTest (main thread only):

```swift
let bridge = try NativeRuntimeBridge()
let host = try bridge.hostHandle()          // borrowed; never call host.destroy()
let source = try String(
    contentsOfFile: "/absolute/path/to/dist/aot/quickjs-bundle.js",
    encoding: .utf8
)
assert(try bridge.loadScript(source) == 0)
assert(angnative_host_node_count(host) > 0)
let result = bridge.dispatchEvent(
    nodeId: <node-id-with-a-press-listener>,
    eventName: "press",
    payloadJson: "{\"source\":\"native\"}"
)
assert(!result.rejectedForWrongThread)
bridge.destroy()                            // same (main/owner) thread
```

Also verify on-device: zero `WebViews`, live Angular listener after bundle
load, and a UIKit tap that updates a native node through `SetTextContent`. Do
not dispatch from a background queue: the C ABI now rejects wrong-thread calls
with `UInt32.max`, but iOS has no UI-thread marshalling layer.

## Status (2026-11-22)

| Feature | Status |
|---------|--------|
| Swift wrapper around C ABI (`SwiftFFI.swift`, UInt64 node ids, free_string, 5 runtime bridge bindings) | ✅ Implemented; static FFI check passes; Swift compile pending on Mac |
| UIKit view factory (8 PlatformOp callbacks, real `user_data`) | ✅ Implemented |
| Demo command batch + Taffy layout + click → Rust batch | ✅ Implemented |
| SwiftPM package (Swift 6 tools version, links `libangnative_runtime_bridge.a`) | ✅ Implemented; static wiring check passes |
| Mac build / Simulator run (including Swift compile and runtime bridge smoke test) | ⚠️ **Not verified** — no Mac in this environment |
| wgpu GPU rendering | ❌ Not applicable (UIKit used instead) |
| Hot reload / App Store distribution | ❌ Not yet |

### Mac verification checklist (do this first on the Mac)

```bash
# 0. Static binding/wiring check (runs anywhere)
python scripts/check-ffi-declarations.py

# 1. Build + stage the unified Rust library and run Swift build/tests
./scripts/build-ios-native.sh simulator
cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost

# 2. Runtime bridge smoke test: see the section above; generate a bundle and
#    assert load_script == 0, nodeCount > 0, dispatchEvent not rejected.

# 3. What to verify in Simulator
#    - "ANGNATIVE • IOS SHELL" badge, "Hello World Demo" title, subtitle
#    - "Click Me!" and "Reset" buttons visible
#    - Tapping a button updates the output text via a Rust SetTextContent op
#    - Xcode console shows nodeCount > 0 and non-empty ops JSON
```

Known fixes applied during preparation: duplicate `swift-tools-version` lines
removed, ABI node ids corrected from `UInt32` to `UInt64`, `UIViewFactory`
type-name mismatch fixed to `UIKitViewFactory`, `UIColor(hex:)` initializer
corrected, duplicate `layoutJson` binding removed, and the demo now uses
explicit node heights (Rust intrinsic text measurement is still a stub).

## Notes

- iOS Simulator is **not** available on Linux/Windows — a physical iPhone or macOS is required
- Free Apple ID allows 7-day signing; paid Apple ID ($99/yr) allows 1-year signing
- Developer Mode must be enabled on iOS: Settings → Privacy & Security → Developer Mode
