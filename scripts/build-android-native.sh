#!/usr/bin/env bash
# build-android-native.sh — Build the unified AngNative Rust library for Android.
#
# The runtime-bridge staticlib bundles host-ffi + host-engine + QuickJS + the
# runtime bridge into one archive, so platform shells link a single library.
# rquickjs-sys 0.5.1 ships no Android bindings, so bindgen is enabled with the
# NDK sysroot and a libclang installation (LIBCLANG_PATH).
#
# Prerequisites: Rust Android targets, Android NDK, libclang (for bindgen).
# Usage: ./scripts/build-android-native.sh [arm64-v8a|armeabi-v7a|x86_64|all]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

ANDROID_API_LEVEL=24
DEFAULT_TARGETS="aarch64-linux-android armv7-linux-androideabi x86_64-linux-android"
BUILD_TARGET="${1:-all}"

# --- Validate environment -------------------------------------------------
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  for candidate in "${ANDROID_HOME:-}/ndk"/* "$HOME/AppData/Local/Android/Sdk/ndk"/*; do
    if [[ -d "$candidate" ]]; then
      export ANDROID_NDK_HOME="${candidate%/}"
      break
    fi
  done
fi
if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "ERROR: ANDROID_NDK_HOME is not set."
  exit 1
fi
if [[ -z "${LIBCLANG_PATH:-}" ]]; then
  echo "ERROR: LIBCLANG_PATH must point to a directory containing libclang."
  exit 1
fi

HOST_TAG="linux-x86_64"
[[ "$(uname -s)" == "Darwin" ]] && HOST_TAG="darwin-x86_64"
uname -s | grep -qi mingw && HOST_TAG="windows-x86_64"

TOOLCHAIN="${ANDROID_NDK_HOME}/toolchains/llvm/prebuilt/${HOST_TAG}"
BIN="${TOOLCHAIN}/bin"
SYSROOT="${TOOLCHAIN}/sysroot"
AR="${BIN}/llvm-ar"
[[ -f "${AR}.exe" ]] && AR="${AR}.exe"

echo "=============================================="
echo "  AngNative Android Native Library Build"
echo "=============================================="
echo "  NDK: ${ANDROID_NDK_HOME}"
echo "  Bindgen libclang: ${LIBCLANG_PATH}"
echo ""

case "${BUILD_TARGET}" in
  all) TARGETS=(${DEFAULT_TARGETS}) ;;
  arm64-v8a) TARGETS=("aarch64-linux-android") ;;
  armeabi-v7a) TARGETS=("armv7-linux-androideabi") ;;
  x86_64) TARGETS=("x86_64-linux-android") ;;
  *) echo "ERROR: unknown target '${BUILD_TARGET}'"; exit 1 ;;
esac

DESTINATIONS=(
  "${REPO_ROOT}/platforms/android/app/src/main/jniLibs"
  "${REPO_ROOT}/showcases/ecosystem-core/app/src/main/jniLibs"
)

for rust_target in "${TARGETS[@]}"; do
  case "${rust_target}" in
    aarch64-linux-android)
      abi="arm64-v8a"; clang="aarch64-linux-android${ANDROID_API_LEVEL}-clang"; incdir="aarch64-linux-android"
      prefix="AARCH64_LINUX_ANDROID"; ccvar="CC_aarch64_linux_android" ;;
    armv7-linux-androideabi)
      abi="armeabi-v7a"; clang="armv7a-linux-androideabi${ANDROID_API_LEVEL}-clang"; incdir="arm-linux-androideabi"
      prefix="ARMV7_LINUX_ANDROIDEABI"; ccvar="CC_armv7_linux_androideabi" ;;
    x86_64-linux-android)
      abi="x86_64"; clang="x86_64-linux-android${ANDROID_API_LEVEL}-clang"; incdir="x86_64-linux-android"
      prefix="X86_64_LINUX_ANDROID"; ccvar="CC_x86_64_linux_android" ;;
  esac

  clang_path="${BIN}/${clang}"
  [[ -f "${clang_path}.cmd" ]] && clang_path="${clang_path}.cmd"
  if [[ ! -f "${clang_path}" ]]; then
    echo "ERROR: linker not found: ${clang_path}"
    exit 1
  fi

  export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=${SYSROOT} -I${SYSROOT}/usr/include -I${SYSROOT}/usr/include/${incdir} -D__ANDROID_API__=${ANDROID_API_LEVEL}"
  export "${ccvar}=${clang_path}"
  export "CARGO_TARGET_${prefix}_LINKER=${clang_path}"
  export "CARGO_TARGET_${prefix}_AR=${AR}"
  export PATH="${BIN}:${PATH}"

  echo "=== Building ${rust_target} (${abi}) ==="
  cargo build --release --target "${rust_target}" --package angnative-runtime-bridge

  src="${REPO_ROOT}/target/${rust_target}/release/libangnative_runtime_bridge.a"
  for dest in "${DESTINATIONS[@]}"; do
    abi_dir="${dest}/${abi}"
    mkdir -p "${abi_dir}"
    cp "${src}" "${abi_dir}/libangnative_runtime_bridge.a"
    # Keep the legacy filename in sync for configurations that cached it.
    cp "${src}" "${abi_dir}/libangnative_host_ffi.a"
    echo "  copied -> ${abi_dir}"
  done
done

echo ""
echo "Android Rust libraries rebuilt (unified runtime bridge)."
