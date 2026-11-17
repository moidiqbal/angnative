#!/usr/bin/env bash
# build-android-native.sh — Build libangnative for Android via cargo + NDK
#
# Prerequisites:
#   1. Rust toolchain installed
#   2. Android NDK (r25+) installed
#   3. ANDROID_NDK_HOME environment variable set (or auto-detected)
#   4. Android targets added: rustup target add aarch64-linux-android armv7-linux-androideabi x86_64-linux-android
#
# Usage:
#   ./scripts/build-android-native.sh [arm64-v8a|armeabi-v7a|x86_64|all]
#
# Output:
#   platforms/android/app/src/main/jniLibs/<abi>/libangnative_host_ffi.a
#   platforms/android/app/src/main/jniLibs/<abi>/libangnative_host_ffi.so

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# --- Configuration ---
ANDROID_API_LEVEL=24
DEFAULT_TARGETS="aarch64-linux-android armv7-linux-androideabi x86_64-linux-android"
ABI_MAP=(
  "aarch64-linux-android:arm64-v8a"
  "armv7-linux-androideabi:armeabi-v7a"
  "x86_64-linux-android:x86_64"
)

BUILD_TARGET="${1:-all}"
BUILD_TYPE="release"

# --- Validate & auto-detect environment ---
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  if [[ -d "C:/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c" ]]; then
    export ANDROID_NDK_HOME="C:/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c"
  elif [[ -d "/c/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c" ]]; then
    export ANDROID_NDK_HOME="/c/Program Files (x86)/Android/AndroidNDK/android-ndk-r27c"
  elif [[ -d "$HOME/AppData/Local/Android/Sdk/ndk" ]]; then
    LATEST_NDK=$(ls -1d "$HOME/AppData/Local/Android/Sdk/ndk/"* 2>/dev/null | tail -1)
    if [[ -n "${LATEST_NDK}" ]]; then
      export ANDROID_NDK_HOME="${LATEST_NDK}"
    fi
  fi
fi

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "ERROR: ANDROID_NDK_HOME is not set."
  echo "  Set it to your Android NDK path, e.g.:"
  echo "    export ANDROID_NDK_HOME=\$HOME/Android/Sdk/ndk/25.1.8937393"
  exit 1
fi

if ! command -v cargo &> /dev/null; then
  echo "ERROR: cargo not found. Install Rust: https://rustup.rs/"
  exit 1
fi

if ! command -v rustc &> /dev/null; then
  echo "ERROR: rustc not found."
  exit 1
fi

echo "=============================================="
echo "  AngNative Android Native Library Build"
echo "=============================================="
echo "  NDK: ${ANDROID_NDK_HOME}"
echo "  Rust: $(rustc --version)"
echo "  Cargo: $(cargo --version)"
echo ""

# --- Determine targets to build ---
if [[ "${BUILD_TARGET}" == "all" ]]; then
  TARGETS=(${DEFAULT_TARGETS})
else
  # Map ABI name to rust target
  case "${BUILD_TARGET}" in
    arm64-v8a) TARGETS=("aarch64-linux-android") ;;
    armeabi-v7a) TARGETS=("armv7-linux-androideabi") ;;
    x86_64) TARGETS=("x86_64-linux-android") ;;
    *)
      echo "ERROR: Unknown target '${BUILD_TARGET}'. Use: arm64-v8a, armeabi-v7a, x86_64, or all."
      exit 1
      ;;
  esac
fi

echo "  Targets: ${TARGETS[*]}"
echo ""

# --- Find NDK clang linker ---
find_ndk_toolchain() {
  local target="$1"
  local ndk="$ANDROID_NDK_HOME"

  # Find the clang compiler in the NDK toolchains
  local toolchain_dir="${ndk}/toolchains/llvm/prebuilt/linux-x86_64"
  if [[ ! -d "${toolchain_dir}" ]]; then
    # Try macOS
    toolchain_dir="${ndk}/toolchains/llvm/prebuilt/darwin-x86_64"
  fi
  if [[ ! -d "${toolchain_dir}" ]]; then
    # Try Windows
    toolchain_dir="${ndk}/toolchains/llvm/prebuilt/windows-x86_64"
  fi

  local ext=""
  if [[ -f "${toolchain_dir}/bin/aarch64-linux-android${ANDROID_API_LEVEL}-clang.cmd" ]]; then
    ext=".cmd"
  fi

  case "${target}" in
    aarch64-linux-android) echo "${toolchain_dir}/bin/aarch64-linux-android${ANDROID_API_LEVEL}-clang${ext}" ;;
    armv7-linux-androideabi) echo "${toolchain_dir}/bin/armv7a-linux-androideabi${ANDROID_API_LEVEL}-clang${ext}" ;;
    x86_64-linux-android) echo "${toolchain_dir}/bin/x86_64-linux-android${ANDROID_API_LEVEL}-clang${ext}" ;;
    *) echo "" ;;
  esac
}

# Helper: format bytes
format_size() {
  local bytes=$1
  if [[ ${bytes} -lt 1024 ]]; then
    echo "${bytes} B"
  elif [[ ${bytes} -lt 1048576 ]]; then
    echo "$((bytes / 1024)) KB"
  else
    echo "$((bytes / 1048576)) MB"
  fi
}

# --- Build each target ---
for rust_target in "${TARGETS[@]}"; do
  # Map rust target to Android ABI
  abi=""
  for pair in "${ABI_MAP[@]}"; do
    IFS=':' read -r rt abi_name <<< "${pair}"
    if [[ "${rt}" == "${rust_target}" ]]; then
      abi="${abi_name}"
      break
    fi
  done

  if [[ -z "${abi}" ]]; then
    echo "WARNING: No ABI mapping for ${rust_target}, skipping."
    continue
  fi

  echo "----------------------------------------------"
  echo "  Building ${rust_target} (ABI: ${abi})"
  echo "----------------------------------------------"

  # Find linker
  LINKER=$(find_ndk_toolchain "${rust_target}")
  if [[ ! -f "${LINKER}" ]]; then
    echo "ERROR: Linker not found for ${rust_target}: ${LINKER}"
    echo "  Check ANDROID_NDK_HOME points to a valid NDK."
    exit 1
  fi

  TOOLCHAIN_DIR="$(dirname "$(dirname "${LINKER}")")"
  AR="${TOOLCHAIN_DIR}/bin/llvm-ar"
  if [[ -f "${TOOLCHAIN_DIR}/bin/llvm-ar.exe" ]]; then
    AR="${TOOLCHAIN_DIR}/bin/llvm-ar.exe"
  fi

  # Set up cargo environment for cross-compilation
  target_upper=$(echo "${rust_target}" | tr '[:lower:]' '[:upper:]' | tr '-' '_')
  export CARGO_TARGET_${target_upper}_LINKER="${LINKER}"
  export CARGO_TARGET_${target_upper}_AR="${AR}"
  export PATH="${TOOLCHAIN_DIR}/bin:${PATH}"

  # Build the host-ffi crate
  echo "  Building angnative-host-ffi for ${rust_target}..."
  cargo build \
    --release \
    --target "${rust_target}" \
    --package angnative-host-ffi \
    2>&1 | tail -20

  TARGET_DIR="${REPO_ROOT}/target/${rust_target}/release"
  STATIC_LIB="${TARGET_DIR}/libangnative_host_ffi.a"
  SHARED_LIB="${TARGET_DIR}/libangnative_host_ffi.so"

  if [[ ! -f "${STATIC_LIB}" && ! -f "${SHARED_LIB}" ]]; then
    echo "ERROR: Built libraries not found in: ${TARGET_DIR}"
    exit 1
  fi

  JNI_DEST="${REPO_ROOT}/platforms/android/app/src/main/jniLibs/${abi}"
  mkdir -p "${JNI_DEST}"

  if [[ -f "${STATIC_LIB}" ]]; then
    cp "${STATIC_LIB}" "${JNI_DEST}/libangnative_host_ffi.a"
    echo "  ✓ static: ${STATIC_LIB} -> ${JNI_DEST}/"
  fi
  if [[ -f "${SHARED_LIB}" ]]; then
    cp "${SHARED_LIB}" "${JNI_DEST}/libangnative_host_ffi.so"
    echo "  ✓ shared: ${SHARED_LIB} -> ${JNI_DEST}/"
  fi
done

echo ""
echo "=============================================="
echo "  Build Complete!"
echo "=============================================="
echo "  Libs copied to:"
echo "    platforms/android/app/src/main/jniLibs/<abi>/"
echo ""
