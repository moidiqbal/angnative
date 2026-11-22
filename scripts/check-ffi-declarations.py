#!/usr/bin/env python3
"""
check-ffi-declarations.py — Static consistency checks for all platform FFI bridges.

No SDK/Xcode/device required. Checks:
  1. Kotlin `external fun nativeX(...)` <-> C++ `Java_..._nativeX` (name + arity)
  2. C++ `angnative_*(...)` calls <-> declarations in angnative_host_ffi.h
  3. Swift `@_silgen_name("angnative_*")` <-> declarations in angnative_host_ffi.h
  4. Swift runtime-bridge bindings: required symbols, key signature types, and
     iOS build wiring (unified staticlib, not the legacy host-only archive)

This is STATIC verification only; it does not replace Android/iOS builds or
prove that `swift build` / Simulator tests pass on macOS. The Kotlin/JNI side
covers the flagship `ecosystem-core` shell only; other shells call Rust host
APIs directly and are outside this checker's scope. Signature checks compare
names/arity for Kotlin and exact patterns for the Swift runtime bridge.

Usage: python scripts/check-ffi-declarations.py
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
KT = ROOT / "showcases/ecosystem-core/app/src/main/java/com/angnative/app/JniBridge.kt"
CPP = ROOT / "showcases/ecosystem-core/app/src/main/cpp/jni_bridge.cpp"
SWIFT = ROOT / "platforms/ios/Sources/AngNativeHost/SwiftFFI.swift"
HEADER = ROOT / "crates/angnative-host-ffi/include/angnative_host_ffi.h"
IOS_PACKAGE = ROOT / "platforms/ios/Package.swift"
IOS_BUILD_SCRIPT = ROOT / "scripts/build-ios-native.sh"

# The unified runtime bridge is implemented in angnative-runtime-bridge, but
# its C ABI is declared in the shared host-ffi header. Every iOS binding below
# must exist in both places with the exact ABI types used by the Swift wrapper.
REQUIRED_RUNTIME_BRIDGE_SYMBOLS = {
    "angnative_runtime_bridge_create",
    "angnative_runtime_bridge_destroy",
    "angnative_runtime_bridge_dispatch_event",
    "angnative_runtime_bridge_load_script",
    "angnative_runtime_bridge_get_host",
    "angnative_runtime_bridge_lifecycle_state",
    "angnative_runtime_bridge_suspend",
    "angnative_runtime_bridge_resume",
    "angnative_runtime_bridge_stop",
    "angnative_runtime_bridge_metrics_json",
    "angnative_runtime_bridge_last_error_json",
    "angnative_runtime_bridge_free_string",
    "angnative_runtime_bridge_set_error_callback",
}

SWIFT_SIGNATURE_PATTERNS = {
    "angnative_runtime_bridge_create": (
        r"func\s+angnative_runtime_bridge_create\s*\(\s*\)\s*->\s*"
        r"AngNativeRuntimeBridgeHandle"
    ),
    "angnative_runtime_bridge_destroy": (
        r"func\s+angnative_runtime_bridge_destroy\s*\(\s*_\s+handle:\s*"
        r"AngNativeRuntimeBridgeHandle\s*\)"
    ),
    "angnative_runtime_bridge_dispatch_event": (
        r"func\s+angnative_runtime_bridge_dispatch_event\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*,\s*"
        r"_\s+nodeId:\s*UInt64\s*,\s*"
        r"_\s+eventName:\s*UnsafePointer<CChar>\?\s*,\s*"
        r"_\s+payloadJson:\s*UnsafePointer<CChar>\?\s*\)\s*->\s*UInt32"
    ),
    "angnative_runtime_bridge_load_script": (
        r"func\s+angnative_runtime_bridge_load_script\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*,\s*"
        r"_\s+source:\s*UnsafePointer<CChar>\?\s*\)\s*->\s*Int32"
    ),
    "angnative_runtime_bridge_get_host": (
        r"func\s+angnative_runtime_bridge_get_host\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)\s*->\s*AngNativeHost\?"
    ),
    "angnative_runtime_bridge_lifecycle_state": (
        r"func\s+angnative_runtime_bridge_lifecycle_state\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)\s*->\s*Int32"
    ),
    "angnative_runtime_bridge_suspend": (
        r"func\s+angnative_runtime_bridge_suspend\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)\s*->\s*Int32"
    ),
    "angnative_runtime_bridge_resume": (
        r"func\s+angnative_runtime_bridge_resume\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)\s*->\s*Int32"
    ),
    "angnative_runtime_bridge_stop": (
        r"func\s+angnative_runtime_bridge_stop\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)\s*->\s*Int32"
    ),
    "angnative_runtime_bridge_metrics_json": (
        r"func\s+angnative_runtime_bridge_metrics_json\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)"
    ),
    "angnative_runtime_bridge_last_error_json": (
        r"func\s+angnative_runtime_bridge_last_error_json\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*\)"
    ),
    "angnative_runtime_bridge_free_string": (
        r"func\s+angnative_runtime_bridge_free_string\s*\(\s*"
        r"_\s+ptr:\s*UnsafeMutablePointer<CChar>\?\s*\)"
    ),
    "angnative_runtime_bridge_set_error_callback": (
        r"func\s+angnative_runtime_bridge_set_error_callback\s*\(\s*"
        r"_\s+handle:\s*AngNativeRuntimeBridgeHandle\s*,\s*"
        r"_\s+callback:\s*AngNativeErrorCallback\?\s*,\s*"
        r"_\s+userData:\s*UnsafeMutableRawPointer\?\s*\)"
    ),
}

# C exports intentionally kept after Kotlin switched to the runtime bridge.
LEGACY_CPP_ONLY = {
    "nativeHostCreate",
    "nativeHostDestroy",
    # C++ implementation kept for legacy shells; no longer declared by the
    # 10 unified showcase bridges (they use nativeRuntimeBridgeDispatchEvent).
    "nativeHostDispatchEvent",
}


def kotlin_methods(text: str) -> dict[str, int]:
    result = {}
    for m in re.finditer(r"external\s+fun\s+(\w+)\s*\(([^)]*)\)", text):
        name, params = m.group(1), m.group(2).strip()
        result[name] = 0 if not params else len(
            [p for p in params.split(",") if p.strip()]
        )
    return result


def cpp_jni_methods(text: str) -> dict[str, int]:
    result = {}
    pattern = r"Java_com_angnative_app_JniBridge_(\w+)\s*\(([^)]*)\)"
    for m in re.finditer(pattern, text, re.S):
        args = [
            re.sub(r"/\*.*?\*/", "", a).strip()
            for a in m.group(2).split(",")
            if a.strip()
        ]
        # JNIEnv* and jobject are implicit.
        result[m.group(1)] = max(0, len(args) - 2)
    return result


def angnative_calls(text: str) -> set[str]:
    return {m.rstrip(" (") for m in re.findall(r"\bangnative_\w+\s*\(", text)}


def header_declarations(text: str) -> set[str]:
    return {m.rstrip(" (") for m in re.findall(r"\bangnative_\w+\s*\(", text)}


def swift_symbols(text: str) -> set[str]:
    return set(re.findall(r'@_silgen_name\("(angnative_\w+)"\)', text))


def main() -> int:
    kt = kotlin_methods(KT.read_text(encoding="utf-8"))
    jni = cpp_jni_methods(CPP.read_text(encoding="utf-8"))
    cpp_text = CPP.read_text(encoding="utf-8")
    header_text = HEADER.read_text(encoding="utf-8")
    header = header_declarations(header_text)
    swift_text = SWIFT.read_text(encoding="utf-8")
    swift = swift_symbols(swift_text)
    package_text = IOS_PACKAGE.read_text(encoding="utf-8")
    ios_build_text = IOS_BUILD_SCRIPT.read_text(encoding="utf-8")
    errors = []

    # 1. Kotlin <-> C++ JNI
    for name, count in sorted(kt.items()):
        if name not in jni:
            errors.append(f"JNI implementation missing in C++: {name}")
        elif jni[name] != count:
            errors.append(
                f"JNI arity mismatch for {name}: Kotlin={count}, C++={jni[name]}"
            )
    for name in sorted(jni):
        if name not in kt and name not in LEGACY_CPP_ONLY:
            errors.append(f"C++ JNI export has no Kotlin declaration: {name}")

    # 2. C++ calls must exist in the C header
    for name in sorted(angnative_calls(cpp_text) - header):
        errors.append(f"C++ calls undeclared C ABI symbol: {name}")

    # 3. Swift @_silgen_name symbols must exist in the C header
    for name in sorted(swift - header):
        errors.append(f"Swift binds undeclared C ABI symbol: {name}")

    # 4. iOS must bind and link the unified runtime bridge, not just the
    #    legacy host. These are static checks only (no macOS/Xcode required).
    for name in sorted(REQUIRED_RUNTIME_BRIDGE_SYMBOLS - swift):
        errors.append(f"Swift runtime bridge binding missing: {name}")
    for name in sorted(REQUIRED_RUNTIME_BRIDGE_SYMBOLS - header):
        errors.append(f"C header runtime bridge declaration missing: {name}")
    if "typedef uint64_t AngNativeRuntimeBridgeHandle;" not in header_text:
        errors.append("C header runtime bridge handle must be a uint64_t handle")
    if "typedef struct AngNativeRuntimeBridge AngNativeRuntimeBridge;" in header_text:
        errors.append("C header still declares the legacy opaque bridge pointer")
    for name, pattern in sorted(SWIFT_SIGNATURE_PATTERNS.items()):
        if not re.search(pattern, swift_text, re.S):
            errors.append(f"Swift runtime bridge signature mismatch: {name}")
    if '.linkedLibrary("angnative_runtime_bridge")' not in package_text:
        errors.append("Package.swift must link angnative_runtime_bridge staticlib")
    if '.linkedLibrary("angnative_host_ffi")' in package_text:
        errors.append("Package.swift still links the legacy host-only staticlib")
    if "--package angnative-runtime-bridge" not in ios_build_text:
        errors.append("build-ios-native.sh must build angnative-runtime-bridge")
    if "libangnative_runtime_bridge.a" not in ios_build_text:
        errors.append("build-ios-native.sh must stage libangnative_runtime_bridge.a")

    if errors:
        print("FFI declaration check FAILED:")
        for error in errors:
            print("  -", error)
        return 1

    print(
        f"FFI declaration check OK: {len(kt)} JNI methods, "
        f"{len(swift)} Swift bindings "
        f"({len(REQUIRED_RUNTIME_BRIDGE_SYMBOLS)} runtime bridge), "
        f"{len(header)} header declarations; iOS unified staticlib wiring OK"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
