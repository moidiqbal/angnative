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
        // Swift wrapper around the Rust host C ABI. The native library is
        // supplied at build time via the XCFramework produced by
        // scripts/build-ios-native.sh (see docs/IOS-BUILD.md).
        .target(
            name: "AngNativeHost",
            path: "Sources/AngNativeHost",
            linkerSettings: [
                .linkedLibrary("angnative_host_ffi")
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
