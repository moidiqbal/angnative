#!/usr/bin/env bash
# setup-android-native.sh — One-command setup for AngNative Android native build
#
# This script:
#   1. Installs Rust Android targets
#   2. Verifies / auto-detects ANDROID_NDK_HOME
#   3. Builds libangnative_host_ffi.a/.so for all Android ABIs
#   4. Copies libs to jniLibs
#   5. Builds the Android APK (if Gradle / gradlew is available)
#
# Usage: ./scripts/setup-android-native.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

echo "=============================================="
echo "  AngNative Android Native Setup"
echo "=============================================="
echo ""

# --- Step 1: Check Rust ---
if ! command -v cargo &> /dev/null; then
  echo "Installing Rust..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
  source "$HOME/.cargo/env"
fi

echo "Rust: $(rustc --version)"
echo "Cargo: $(cargo --version)"

# --- Step 2: Add Android targets ---
echo ""
echo "Adding Rust Android targets..."
rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android 2>&1 | tail -5

# --- Step 3: Check / Auto-detect NDK ---
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  if [[ -d "C:/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c" ]]; then
    export ANDROID_NDK_HOME="C:/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c"
  elif [[ -d "/c/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c" ]]; then
    export ANDROID_NDK_HOME="/c/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c"
  fi
fi

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo ""
  echo "WARNING: ANDROID_NDK_HOME is not set."
  echo "  Please set it and re-run:"
  echo "    export ANDROID_NDK_HOME=/path/to/android-ndk"
  echo "    ./scripts/setup-android-native.sh"
  echo ""
  exit 0
fi

echo "NDK: ${ANDROID_NDK_HOME}"

# --- Step 4: Build native libs ---
echo ""
echo "Building native libraries..."
bash "${SCRIPT_DIR}/build-android-native.sh" all

# --- Step 5: Build APK ---
if [[ -d "C:/Users/Moid Iqbal/cmake-3.22.1/bin" ]]; then
  export PATH="C:/Users/Moid Iqbal/cmake-3.22.1/bin:${PATH}"
elif [[ -d "/c/Users/Moid Iqbal/cmake-3.22.1/bin" ]]; then
  export PATH="/c/Users/Moid Iqbal/cmake-3.22.1/bin:${PATH}"
fi

if [[ -f "${REPO_ROOT}/platforms/android/gradlew" ]]; then
  echo ""
  echo "Building Android APK..."
  cd "${REPO_ROOT}/platforms/android"
  chmod +x ./gradlew 2>/dev/null || true
  ./gradlew assembleDebug
elif command -v gradle &> /dev/null; then
  echo ""
  echo "Building Android APK..."
  cd "${REPO_ROOT}/platforms/android"
  gradle assembleDebug
else
  echo ""
  echo "Gradle not found. Build APK manually:"
  echo "  cd platforms/android"
  echo "  ./gradlew assembleDebug"
fi

echo ""
echo "=============================================="
echo "  Setup Complete!"
echo "=============================================="
