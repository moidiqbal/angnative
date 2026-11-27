#!/usr/bin/env bash
# package-ios-release.sh — Build the iOS release assets on a Mac.
#
# Usage:
#   ./scripts/package-ios-release.sh [version]     # default 0.4.0
#
# Produces dist/ios/:
#   AngNativeDemo-iOS-Simulator.zip      Release demo .app for the iOS Simulator
#                                        (Apple silicon). Install with:
#                                          unzip AngNativeDemo-iOS-Simulator.zip
#                                          xcrun simctl install booted AngNativeDemo.app
#                                          xcrun simctl launch booted dev.angnative.demo
#   AngNativeRuntime.xcframework.zip     Rust runtime bridge staticlib (host-ffi,
#                                        Taffy layout, QuickJS) for iOS device
#                                        (arm64) and Simulator (arm64), plus the
#                                        C header angnative_host_ffi.h
#   SHA256SUMS.txt
#
# Not produced: a device .ipa. Running on an iPhone needs signing and has not
# been verified yet (see docs/evidence/IOS-SIMULATOR.md).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
IOS_DIR="${REPO_ROOT}/platforms/ios"
OUT_DIR="${REPO_ROOT}/dist/ios"
VERSION="${1:-0.4.0}"
STAGE="${IOS_DIR}/.build/release-stage"

if [[ "$(uname -m)" != "arm64" ]]; then
  echo "ERROR: package on an Apple silicon Mac (Simulator assets are arm64)."
  exit 1
fi

rm -rf "${OUT_DIR}" "${STAGE}"
mkdir -p "${OUT_DIR}" "${STAGE}"

# 1. Device build: Rust aarch64-apple-ios + Swift package (unsigned, Release).
"${SCRIPT_DIR}/build-ios-native.sh" device

# 2. Simulator build + full XCTest run (stages the simulator archive last, so
#    the package is left ready for simulator development).
"${SCRIPT_DIR}/build-ios-native.sh" simulator --test

# 3. XCFramework from the two Rust archives.
mkdir -p "${STAGE}/headers"
cp "${REPO_ROOT}/crates/angnative-host-ffi/include/angnative_host_ffi.h" "${STAGE}/headers/"
xcodebuild -create-xcframework \
  -library "${REPO_ROOT}/target/aarch64-apple-ios/release/libangnative_runtime_bridge.a" \
  -headers "${STAGE}/headers" \
  -library "${REPO_ROOT}/target/aarch64-apple-ios-sim/release/libangnative_runtime_bridge.a" \
  -headers "${STAGE}/headers" \
  -output "${STAGE}/AngNativeRuntime.xcframework"
(cd "${STAGE}" && ditto -c -k --norsrc --noextattr --noacl --keepParent AngNativeRuntime.xcframework \
  "${OUT_DIR}/AngNativeRuntime.xcframework.zip")

# 4. Release demo .app for the Simulator.
SKIP_RUST=1 NO_LAUNCH=1 CONFIGURATION=Release APP_VERSION="${VERSION}" \
  "${SCRIPT_DIR}/run-ios-simulator.sh"
(cd "${IOS_DIR}/.build" && ditto -c -k --norsrc --noextattr --noacl --keepParent AngNativeDemo.app \
  "${OUT_DIR}/AngNativeDemo-iOS-Simulator.zip")

# 5. Checksums.
(cd "${OUT_DIR}" && shasum -a 256 *.zip > SHA256SUMS.txt)

echo ""
echo "  ✓ iOS release assets for v${VERSION} in ${OUT_DIR}:"
ls -la "${OUT_DIR}" | sed 's/^/    /'
cat "${OUT_DIR}/SHA256SUMS.txt" | sed 's/^/    /'
