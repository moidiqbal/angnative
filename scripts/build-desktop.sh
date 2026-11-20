#!/usr/bin/env bash
# build-desktop.sh — Build AngNative desktop app for Windows/macOS/Linux
#
# Prerequisites:
#   1. Rust toolchain installed (rustup)
#   2. For Windows: cargo + rustc (MSVC target)
#   3. For Linux: cargo + rustc + libgtk-3-dev (winit dependency)
#   4. For macOS: cargo + rustc (Apple Silicon or Intel)
#
# Usage:
#   ./scripts/build-desktop.sh [windows|macos|linux|all]
#
# Output:
#   target/<target>/release/angnative-desktop

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# --- Configuration ---
DEFAULT_TARGETS="x86_64-pc-windows-msvc aarch64-apple-darwin x86_64-apple-darwin x86_64-unknown-linux-gnu"

BUILD_TARGET="${1:-all}"

# --- Validate environment ---
if ! command -v cargo &> /dev/null; then
  echo "ERROR: cargo not found. Install Rust: https://rustup.rs/"
  exit 1
fi

if ! command -v rustc &> /dev/null; then
  echo "ERROR: rustc not found."
  exit 1
fi

echo "=============================================="
echo "  AngNative Desktop Build"
echo "=============================================="
echo "  Rust: $(rustc --version)"
echo "  Cargo: $(cargo --version)"
echo ""

# --- Determine targets to build ---
if [[ "${BUILD_TARGET}" == "all" ]]; then
  TARGETS=($DEFAULT_TARGETS)
else
  TARGETS=("${BUILD_TARGET}")
fi

echo "  Targets: ${TARGETS[*]}"
echo ""

# --- Add targets if needed ---
for target in "${TARGETS[@]}"; do
  if ! rustup target list --installed 2>/dev/null | grep -q "^${target}$"; then
    echo "Installing Rust target: ${target}"
    rustup target add "${target}"
  fi
done

# --- Build each target ---
for target in "${TARGETS[@]}"; do
  echo "----------------------------------------------"
  echo "  Building angnative-desktop for ${target}"
  echo "----------------------------------------------"

  cargo build \
    --release \
    --target "${target}" \
    --package angnative-desktop \
    2>&1

  if [[ $? -ne 0 ]]; then
    echo "ERROR: Build failed for ${target}"
    exit 1
  fi

  BINARY_PATH="${REPO_ROOT}/target/${target}/release/angnative-desktop"
  if [[ "${target}" == *"windows"* ]]; then
    BINARY_PATH="${BINARY_PATH}.exe"
  fi

  if [[ -f "${BINARY_PATH}" ]]; then
    SIZE=$(stat -c%s "${BINARY_PATH}" 2>/dev/null || stat -f%z "${BINARY_PATH}" 2>/dev/null || echo 0)
    echo "  ✓ ${target}: $(numfmt --to=iec ${SIZE} 2>/dev/null || echo "${SIZE} bytes") -> ${BINARY_PATH}"
  else
    echo "  ✗ ${target}: binary not found at ${BINARY_PATH}"
    exit 1
  fi
done

echo ""
echo "=============================================="
echo "  Desktop Build Complete!"
echo "=============================================="
echo ""
echo "Run the desktop app:"
echo "  ./target/<target>/release/angnative-desktop"
echo ""
