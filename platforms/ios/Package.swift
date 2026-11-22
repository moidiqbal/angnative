// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "AngNative",
    platforms: [
        .iOS(.v17),
        .macOS(.v14),
    ],
    products: [
        .library(name: "AngNativeHost", targets: ["AngNativeHost"]),
        .library(name: "AngNativeApp", targets: ["AngNativeApp"]),
        .executable(name: "AngNativeDemo", targets: ["AngNativeDemo"]),
    ],
    dependencies: [],
    targets: [
        // Swift wrapper around the Rust C ABI. Link the unified runtime-bridge
        // staticlib: it bundles angnative-host-ffi, host-engine and QuickJS,
        // and exports both the legacy host symbols and runtime bridge symbols.
        // scripts/build-ios-native.sh stages it (see docs/IOS-BUILD.md).
        .target(
            name: "AngNativeHost",
            path: "Sources/AngNativeHost",
            linkerSettings: [
                .linkedLibrary("angnative_runtime_bridge")
            ]
        ),
        // UIKit view factory + demo view controller.
        .target(
            name: "AngNativeApp",
            dependencies: ["AngNativeHost"],
            path: "Sources/AngNativeApp"
        ),
        .executableTarget(
            name: "AngNativeDemo",
            dependencies: ["AngNativeApp"],
            path: "Sources/AngNativeDemo"
        ),
        .testTarget(
            name: "AngNativeTests",
            dependencies: ["AngNativeApp"],
            path: "Tests/AngNativeTests"
        ),
    ]
)
