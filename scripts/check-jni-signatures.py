#!/usr/bin/env python3
"""
check-jni-signatures.py — Static consistency check for the Android JNI bridge.

No Android SDK/NDK required. Verifies that every `external fun nativeX(...)`
declared in JniBridge.kt has a matching `Java_com_angnative_app_JniBridge_nativeX`
implementation in jni_bridge.cpp with the SAME argument count. JNIEnv* and
jobject are implicit and excluded from the count.

This is a STATIC check only — it does not replace an Android/Gradle build.

Usage: python scripts/check-jni-signatures.py
"""
import pathlib
import re
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
KT = ROOT / "showcases/ecosystem-core/app/src/main/java/com/angnative/app/JniBridge.kt"
CPP = ROOT / "showcases/ecosystem-core/app/src/main/cpp/jni_bridge.cpp"

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


def cpp_methods(text: str) -> dict[str, int]:
    result = {}
    pattern = r"Java_com_angnative_app_JniBridge_(\w+)\s*\(([^)]*)\)"
    for m in re.finditer(pattern, text, re.S):
        name, params = m.group(1), m.group(2)
        args = [
            re.sub(r"/\*.*?\*/", "", a).strip()
            for a in params.split(",")
            if a.strip()
        ]
        # First two JNI parameters are JNIEnv* and jobject.
        result[name] = max(0, len(args) - 2)
    return result


def main() -> int:
    kt = kotlin_methods(KT.read_text(encoding="utf-8"))
    cpp = cpp_methods(CPP.read_text(encoding="utf-8"))
    errors = []

    for name, count in sorted(kt.items()):
        if name not in cpp:
            errors.append(f"JNI implementation missing in C++: {name}")
        elif cpp[name] != count:
            errors.append(
                f"argument count mismatch for {name}: "
                f"Kotlin={count}, C++={cpp[name]}"
            )

    for name in sorted(cpp):
        if name not in kt and name not in LEGACY_CPP_ONLY:
            errors.append(f"C++ JNI export has no Kotlin declaration: {name}")

    if errors:
        print("JNI signature check FAILED:")
        for error in errors:
            print("  -", error)
        return 1

    print(f"JNI signature check OK: {len(kt)} Kotlin declarations matched")
    return 0


if __name__ == "__main__":
    sys.exit(main())
