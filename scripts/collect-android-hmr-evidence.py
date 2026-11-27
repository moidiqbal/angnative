"""Collect provenance-bearing Android instrumentation evidence.

Usage:
    python scripts/collect-android-hmr-evidence.py <serial> <log-path>

The log records UTC timestamps, the exact adb command lines, device identity
and properties, both APK hashes and the raw stdout/stderr of every command.
It makes no claim beyond what adb actually prints.
"""

from __future__ import annotations

import hashlib
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
ADB = Path(r"C:\Android\Sdk\platform-tools\adb.exe")
APP_APK = REPO / "showcases/ecosystem-core/app/build/outputs/apk/debug/app-debug.apk"
TEST_APK = (
    REPO
    / "showcases/ecosystem-core/app/build/outputs/apk/androidTest/debug/app-debug-androidTest.apk"
)
RUNNER = "com.angnative.demo.ecosystem.core.test/androidx.test.runner.AndroidJUnitRunner"
TEST_CLASS = "com.angnative.app.HmrZeroCopyBinaryTest"
PROPS = (
    "ro.product.manufacturer",
    "ro.product.model",
    "ro.product.device",
    "ro.build.version.release",
    "ro.build.version.sdk",
    "ro.product.cpu.abi",
    "ro.build.date.utc",
)


def utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def main() -> int:
    if len(sys.argv) != 3:
        print(__doc__, file=sys.stderr)
        return 2
    serial, log_arg = sys.argv[1], Path(sys.argv[2])
    log_arg.parent.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []

    def emit(text: str = "") -> None:
        lines.append(text)

    def command(title: str, args: list[str]) -> int:
        emit()
        emit(f"# {title} — {utc()}")
        emit(f"$ {ADB} " + " ".join(str(part) for part in args))
        result = subprocess.run([str(ADB), *args], text=True, capture_output=True)
        if result.stdout:
            emit("--- stdout ---")
            emit(result.stdout.rstrip("\n"))
        if result.stderr:
            emit("--- stderr ---")
            emit(result.stderr.rstrip("\n"))
        emit(f"exit_code: {result.returncode}")
        print(f"{title}: exit {result.returncode}")
        if result.stdout.strip():
            print(result.stdout.strip()[:500])
        return result.returncode

    emit(f"captured_at_utc: {utc()}")
    emit("claim: Android instrumentation of ANB1, borrowed JNI buffer and HMR")
    emit(f"claim_boundary: execution on adb serial {serial} only; no other device is claimed")
    emit(f"workspace_head: {subprocess.run(['git','rev-parse','HEAD'],cwd=REPO,text=True,capture_output=True).stdout.strip()}")
    emit(f"adb_path: {ADB}")
    emit(f"adb_version: {subprocess.run([str(ADB),'version'],text=True,capture_output=True).stdout.splitlines()[0]}")
    emit(f"serial: {serial}")
    emit(f"app_apk: {APP_APK.relative_to(REPO)}")
    emit(f"app_apk_bytes: {APP_APK.stat().st_size}")
    emit(f"app_apk_sha256: {sha256(APP_APK)}")
    emit(f"test_apk: {TEST_APK.relative_to(REPO)}")
    emit(f"test_apk_bytes: {TEST_APK.stat().st_size}")
    emit(f"test_apk_sha256: {sha256(TEST_APK)}")
    emit(f"runner: {RUNNER}")
    emit(f"test_class: {TEST_CLASS}")

    command("devices", ["devices", "-l"])
    for prop in PROPS:
        emit()
        emit(f"# getprop {prop} — {utc()}")
        result = subprocess.run(
            [str(ADB), "-s", serial, "shell", "getprop", prop],
            text=True,
            capture_output=True,
        )
        emit(f"$ {ADB} -s {serial} shell getprop {prop}")
        emit(f"value: {result.stdout.strip()}")
        emit(f"exit_code: {result.returncode}")

    failures = 0
    failures += command("install app APK", ["-s", serial, "install", "-r", str(APP_APK)])
    failures += command("install test APK", ["-s", serial, "install", "-r", str(TEST_APK)])
    failures += command(
        "instrument test",
        ["-s", serial, "shell", "am", "instrument", "-w", "-e", "class", TEST_CLASS, RUNNER],
    )
    emit()
    emit(f"captured_at_utc_end: {utc()}")
    emit(f"command_failures: {failures}")

    log_arg.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
    print(f"evidence written: {log_arg}")
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
