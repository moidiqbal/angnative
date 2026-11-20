#!/usr/bin/env bash
# build-ios-native.sh — Build the AngNative iOS shell on macOS.
#
# Prerequisites (Mac only):
#   - Xcode command line tools + Swift 6
#   - Rust iOS targets: rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
#
# Usage:
#   ./scripts/build-ios-native.sh [simulator|device]
#
# What it does:
#   1. Builds angnative-host-ffi as a static lib for the selected iOS target.
#   2. Copies the .a + C header into the SwiftPM package.
#   3. Runs `swift build` (simulator) or `swift build -c release -Xswiftc -target ...` (device).
#   4. Prints the exact commands to launch the demo in Simulator / on a device.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
IOS_DIR="${REPO_ROOT}/platforms/ios"

BUILD_TARGET="${1:-simulator}"
HOST_LIB_DIR="${IOS_DIR}/Sources/AngNativeHost"

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "ERROR: iOS builds require macOS/Xcode. Run this on the Mac."
  exit 1
fi

if [[ "${BUILD_TARGET}" == "simulator" ]]; then
  case "$(uname -m)" in
    arm64) IOS_TARGET="aarch64-apple-ios-sim" ;;
    *)     IOS_TARGET="x86_64-apple-ios" ;;
  esac
  SWIFT_FLAGS=""
else
  IOS_TARGET="aarch64-apple-ios"
  SWIFT_FLAGS="-c release"
fi

echo "=============================================="
echo "  AngNative iOS Build"
echo "=============================================="
echo "  Rust target : ${IOS_TARGET}"
echo "  Build mode  : ${BUILD_TARGET}"
echo ""

# 1. Rust static library
if ! rustup target list --installed | grep -q "^${IOS_TARGET}$"; then
  rustup target add "${IOS_TARGET}"
fi

cargo build --release --target "${IOS_TARGET}" --package angnative-host-ffi

RUST_LIB="${REPO_ROOT}/target/${IOS_TARGET}/release/libangnative_host_ffi.a"
if [[ ! -f "${RUST_LIB}" ]]; then
  echo "ERROR: Rust static library not found: ${RUST_LIB}"
  exit 1
fi

# 2. Stage library + header into the SwiftPM package
mkdir -p "${HOST_LIB_DIR}"
cp "${RUST_LIB}" "${HOST_LIB_DIR}/libangnative_host_ffi.a"
mkdir -p "${HOST_LIB_DIR}/include"
cp "${REPO_ROOT}/crates/angnative-host-ffi/include/angnative_host_ffi.h" \
   "${HOST_LIB_DIR}/include/angnative_host_ffi.h"

echo "  ✓ staged Rust library and header"

# 3. Swift build
cd "${IOS_DIR}"
swift build ${SWIFT_FLAGS} \
  -Xlinker -L"${HOST_LIB_DIR}"

echo ""
echo "=============================================="
echo "  iOS shell build complete"
echo "=============================================="
echo "Run tests:"
echo "  cd ${IOS_DIR} && swift test -Xlinker -L${HOST_LIB_DIR}"
echo ""
echo "Open in Xcode (simulator):"
echo "  open -a Simulator"
echo "  xcode-select -p   # ensure Xcode is selected"
echo "  swift run AngNativeDemo   # or run the .app from DerivedData"
