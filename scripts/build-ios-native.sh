#!/usr/bin/env bash
# build-ios-native.sh — Build the AngNative iOS shell on macOS.
#
# Prerequisites (Mac only):
#   - Xcode (not just the command line tools) with an iOS Simulator runtime
#   - Rust iOS targets: rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#     (added automatically below if missing)
#
# Usage:
#   ./scripts/build-ios-native.sh [simulator|device] [--test]
#
# What it does:
#   1. Builds angnative-runtime-bridge as a static lib for the selected iOS
#      target. This archive bundles host-ffi, host-engine and QuickJS and
#      exports both legacy host and unified runtime-bridge C ABI symbols.
#      rquickjs-sys has no pregenerated iOS bindings, so bindgen runs against
#      the iOS SDK (see crates/angnative-host-engine/Cargo.toml).
#   2. Copies the .a + C header into the SwiftPM package.
#   3. Builds the package with xcodebuild for an iOS destination. Plain
#      `swift build` / `swift test` compile for macOS, where UIKit does not
#      exist, so they cannot build this package.
#   4. With --test (simulator only), runs the XCTest suite in the Simulator.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
IOS_DIR="${REPO_ROOT}/platforms/ios"

BUILD_TARGET="${1:-simulator}"
RUN_TESTS="${2:-}"
HOST_LIB_DIR="${IOS_DIR}/Sources/AngNativeHost"
DERIVED_DATA="${IOS_DIR}/.build/xcode"

# Must match `platforms: [.iOS(.v17)]` in platforms/ios/Package.swift, so the
# QuickJS C objects are not built for a newer iOS than the app links against.
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "ERROR: iOS builds require macOS/Xcode. Run this on the Mac."
  exit 1
fi

if [[ "${BUILD_TARGET}" == "simulator" ]]; then
  case "$(uname -m)" in
    arm64) IOS_TARGET="aarch64-apple-ios-sim" ;;
    *)     IOS_TARGET="x86_64-apple-ios" ;;
  esac
  SDK_NAME="iphonesimulator"
  CONFIGURATION="Debug"
  # Any available iPhone simulator works; override with IOS_SIM_DESTINATION.
  DESTINATION="${IOS_SIM_DESTINATION:-platform=iOS Simulator,name=$(
    xcrun simctl list devices available | sed -n 's/^ *\(iPhone[^(]*\) (.*/\1/p' | head -1 | sed 's/ *$//'
  )}"
elif [[ "${BUILD_TARGET}" == "device" ]]; then
  IOS_TARGET="aarch64-apple-ios"
  SDK_NAME="iphoneos"
  CONFIGURATION="Release"
  DESTINATION="generic/platform=iOS"
else
  echo "usage: $0 [simulator|device] [--test]"
  exit 2
fi

SDK_PATH="$(xcrun --sdk "${SDK_NAME}" --show-sdk-path)"

echo "=============================================="
echo "  AngNative iOS Build"
echo "=============================================="
echo "  Rust target : ${IOS_TARGET}"
echo "  Build mode  : ${BUILD_TARGET} (${CONFIGURATION})"
echo "  Destination : ${DESTINATION}"
echo "  Min iOS     : ${IPHONEOS_DEPLOYMENT_TARGET}"
echo ""

# 1. Rust static library
if ! rustup target list --installed | grep -q "^${IOS_TARGET}$"; then
  rustup target add "${IOS_TARGET}"
fi

# bindgen (rquickjs-sys) needs the iOS SDK headers, not the macOS ones.
export BINDGEN_EXTRA_CLANG_ARGS="${BINDGEN_EXTRA_CLANG_ARGS:-} -isysroot ${SDK_PATH}"

cargo build --release --target "${IOS_TARGET}" --package angnative-runtime-bridge

RUST_LIB="${REPO_ROOT}/target/${IOS_TARGET}/release/libangnative_runtime_bridge.a"
if [[ ! -f "${RUST_LIB}" ]]; then
  echo "ERROR: Rust static library not found: ${RUST_LIB}"
  exit 1
fi

# 2. Stage library + header into the SwiftPM package
mkdir -p "${HOST_LIB_DIR}"
cp "${RUST_LIB}" "${HOST_LIB_DIR}/libangnative_runtime_bridge.a"
# Avoid accidentally linking a stale host-only archive left by older builds.
rm -f "${HOST_LIB_DIR}/libangnative_host_ffi.a"
mkdir -p "${HOST_LIB_DIR}/include"
cp "${REPO_ROOT}/crates/angnative-host-ffi/include/angnative_host_ffi.h" \
   "${HOST_LIB_DIR}/include/angnative_host_ffi.h"

echo "  ✓ staged Rust library and header"

# 3. Build the Swift package for iOS
cd "${IOS_DIR}"
XCODEBUILD_ARGS=(
  -scheme AngNative-Package
  -destination "${DESTINATION}"
  -configuration "${CONFIGURATION}"
  -derivedDataPath "${DERIVED_DATA}"
  LIBRARY_SEARCH_PATHS="${HOST_LIB_DIR}"
  IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET}"
)
if [[ "${BUILD_TARGET}" == "device" ]]; then
  XCODEBUILD_ARGS+=(CODE_SIGNING_ALLOWED=NO)
  xcodebuild build "${XCODEBUILD_ARGS[@]}"
else
  xcodebuild build-for-testing "${XCODEBUILD_ARGS[@]}"
fi

echo ""
echo "  ✓ iOS shell build complete"

# 4. Optional XCTest run in the Simulator
if [[ "${RUN_TESTS}" == "--test" ]]; then
  if [[ "${BUILD_TARGET}" != "simulator" ]]; then
    echo "ERROR: --test is only supported for simulator builds."
    exit 2
  fi
  xcodebuild test-without-building "${XCODEBUILD_ARGS[@]}"
fi

echo ""
echo "Run tests in the Simulator:"
echo "  ./scripts/build-ios-native.sh simulator --test"
echo "Install and launch the demo app in the Simulator:"
echo "  ./scripts/run-ios-simulator.sh"
