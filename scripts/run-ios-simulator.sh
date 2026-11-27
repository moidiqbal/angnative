#!/usr/bin/env bash
# run-ios-simulator.sh — Build, install and launch the AngNative iOS demo in
# the iOS Simulator, then save a screenshot.
#
# SwiftPM executable targets build a bare Mach-O, not an .app, so this wraps
# the AngNativeDemo executable in a minimal app bundle (Info.plist + ad-hoc
# signature), which is all the Simulator needs. No Apple Developer account or
# Xcode project is required.
#
# Usage:
#   ./scripts/run-ios-simulator.sh [screenshot.png]
#
# Env:
#   IOS_SIM_DEVICE  simulator name or UDID (default: first available iPhone)
#   SKIP_RUST=1     reuse the already-staged Rust static library

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
IOS_DIR="${REPO_ROOT}/platforms/ios"
DERIVED_DATA="${IOS_DIR}/.build/xcode"
HOST_LIB_DIR="${IOS_DIR}/Sources/AngNativeHost"
APP_DIR="${IOS_DIR}/.build/AngNativeDemo.app"
BUNDLE_ID="dev.angnative.demo"
SCREENSHOT="${1:-${IOS_DIR}/.build/ios-simulator-demo.png}"
# The script cds into platforms/ios below; resolve a relative path first.
[[ "${SCREENSHOT}" = /* ]] || SCREENSHOT="${PWD}/${SCREENSHOT}"
export IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET:-17.0}"

DEVICE="${IOS_SIM_DEVICE:-$(
  xcrun simctl list devices available | sed -n 's/^ *\(iPhone[^(]*\) (.*/\1/p' | head -1 | sed 's/ *$//'
)}"
if [[ -z "${DEVICE}" ]]; then
  echo "ERROR: no available iPhone simulator. Install an iOS runtime in Xcode > Settings > Components."
  exit 1
fi

# 1. Rust static library (staged into the package)
if [[ "${SKIP_RUST:-}" != "1" ]]; then
  "${SCRIPT_DIR}/build-ios-native.sh" simulator
fi

# 2. Demo executable
cd "${IOS_DIR}"
xcodebuild build \
  -scheme AngNativeDemo \
  -destination "platform=iOS Simulator,name=${DEVICE}" \
  -derivedDataPath "${DERIVED_DATA}" \
  LIBRARY_SEARCH_PATHS="${HOST_LIB_DIR}" \
  IPHONEOS_DEPLOYMENT_TARGET="${IPHONEOS_DEPLOYMENT_TARGET}" \
  -quiet

EXECUTABLE="${DERIVED_DATA}/Build/Products/Debug-iphonesimulator/AngNativeDemo"

# 3. Wrap in a minimal .app bundle
rm -rf "${APP_DIR}"
mkdir -p "${APP_DIR}"
cp "${EXECUTABLE}" "${APP_DIR}/AngNativeDemo"
cat > "${APP_DIR}/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleIdentifier</key><string>${BUNDLE_ID}</string>
  <key>CFBundleExecutable</key><string>AngNativeDemo</string>
  <key>CFBundleName</key><string>AngNative</string>
  <key>CFBundleDisplayName</key><string>AngNative</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>0.3.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>CFBundleSupportedPlatforms</key><array><string>iPhoneSimulator</string></array>
  <key>MinimumOSVersion</key><string>${IPHONEOS_DEPLOYMENT_TARGET}</string>
  <key>UIDeviceFamily</key><array><integer>1</integer><integer>2</integer></array>
  <key>UILaunchScreen</key><dict/>
  <key>UIApplicationSupportsMultipleScenes</key><false/>
</dict>
</plist>
PLIST
codesign --force --sign - "${APP_DIR}" >/dev/null

# 4. Boot, install, launch
xcrun simctl boot "${DEVICE}" 2>/dev/null || true   # already booted is fine
xcrun simctl bootstatus "${DEVICE}" -b >/dev/null
open -b com.apple.iphonesimulator 2>/dev/null || true   # GUI is optional; simctl runs headless
xcrun simctl terminate "${DEVICE}" "${BUNDLE_ID}" 2>/dev/null || true
xcrun simctl install "${DEVICE}" "${APP_DIR}"
LOG="${IOS_DIR}/.build/ios-simulator-demo.log"
# --console-pty streams the app's stdout ([AngNative] nodeCount=...) to the log.
xcrun simctl launch --console-pty --terminate-running-process "${DEVICE}" "${BUNDLE_ID}" >"${LOG}" 2>&1 &
LAUNCH_PID=$!
sleep 4

# 5. Evidence
xcrun simctl io "${DEVICE}" screenshot "${SCREENSHOT}" >/dev/null
kill "${LAUNCH_PID}" 2>/dev/null || true

echo ""
echo "  ✓ AngNativeDemo running on '${DEVICE}'"
echo "  ✓ screenshot: ${SCREENSHOT}"
echo "  app log (${LOG}):"
sed 's/^/    /' "${LOG}"
