# AngNative iOS Platform (P3.2)

iOS platform shell for AngNative. Renders native `UIView` hierarchy via
the Rust host (`angnative-host-ffi` C ABI) bridged through Swift.

## Project Structure

```
platforms/ios/
├── Package.swift                     # SwiftPM package manifest
├── Sources/
│   ├── AngNativeHost/
│   │   └── SwiftFFI.swift            # Swift wrapper around C ABI
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

#### 2. Build Rust host library

```bash
cargo build --release --target aarch64-apple-ios -p angnative-host-ffi
```

This produces `target/aarch64-apple-ios/release/libangnative_host_ffi.a`.

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

## Status (2026-11-21)

| Feature | Status |
|---------|--------|
| Swift wrapper around C ABI (`SwiftFFI.swift`, UInt64 node ids, free_string) | ✅ Implemented |
| UIKit view factory (8 PlatformOp callbacks, real `user_data`) | ✅ Implemented |
| Demo command batch + Taffy layout + click → Rust batch | ✅ Implemented |
| SwiftPM package (Swift 6 tools version) | ✅ Implemented |
| Mac build / Simulator run | ⚠️ **Not verified** — no Mac in this environment |
| wgpu GPU rendering | ❌ Not applicable (UIKit used instead) |
| Hot reload / App Store distribution | ❌ Not yet |

### Mac verification checklist (do this first on the Mac)

```bash
# 1. Build + stage the Rust library and run Swift build/tests
./scripts/build-ios-native.sh simulator
cd platforms/ios && swift test -Xlinker -L../../platforms/ios/Sources/AngNativeHost

# 2. What to verify in Simulator
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
