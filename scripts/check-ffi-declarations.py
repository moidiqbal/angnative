#!/usr/bin/env python3
"""
check-ffi-declarations.py — Static consistency checks for all platform FFI bridges.

No SDK/Xcode/device required. Checks:
  1. Kotlin `external fun nativeX(...)` <-> C++ `Java_..._nativeX` (name + arity)
  2. C++ `angnative_*(...)` calls <-> declarations in angnative_host_ffi.h
  3. Swift `@_silgen_name("angnative_*")` <-> declarations in angnative_host_ffi.h

This is STATIC verification only; it does not replace Android/iOS builds.

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

# C exports intentionally kept after Kotlin switched to the runtime bridge.
LEGACY_CPP_ONLY = {"nativeHostCreate", "nativeHostDestroy"}


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
    header = header_declarations(HEADER.read_text(encoding="utf-8"))
    swift = swift_symbols(SWIFT.read_text(encoding="utf-8"))
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

    if errors:
        print("FFI declaration check FAILED:")
        for error in errors:
            print("  -", error)
        return 1

    print(
        f"FFI declaration check OK: {len(kt)} JNI methods, "
        f"{len(swift)} Swift bindings, {len(header)} header declarations"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
