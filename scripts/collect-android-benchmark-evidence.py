"""Collect provenance-bearing device JNI benchmark evidence.

Usage:
    python scripts/collect-android-benchmark-evidence.py <serial> <log-path> <json-path>

Runs Anb1VsJsonBenchmarkTest on the given adb serial, retains the raw stdout in
the log and the parsed result (with APK hashes and device identity) in JSON.
The JSON assertion line emitted by the test is never rewritten or summarized
before retention.
"""

from __future__ import annotations

import hashlib
import json
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
TEST_CLASS = "com.angnative.app.Anb1VsJsonBenchmarkTest"
PROPS = (
    "ro.product.manufacturer",
    "ro.product.model",
    "ro.product.device",
    "ro.build.version.release",
    "ro.build.version.sdk",
    "ro.product.cpu.abi",
)


def utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__, file=sys.stderr)
        return 2
    serial, log_arg, json_arg = sys.argv[1], Path(sys.argv[2]), Path(sys.argv[3])
    for target in (log_arg, json_arg):
        target.parent.mkdir(parents=True, exist_ok=True)

    lines: list[str] = []
    props: dict[str, str] = {}
    outputs: dict[str, str] = {}

    def emit(text: str = "") -> None:
        lines.append(text)

    def command(title: str, args: list[str]) -> tuple[int, str]:
        emit()
        emit(f"# {title} — {utc()}")
        emit(f"$ {ADB} " + " ".join(str(part) for part in args))
        result = subprocess.run([str(ADB), *args], text=True, capture_output=True)
        merged = (result.stdout or "") + (result.stderr or "")
        emit("--- stdout+stderr ---")
        emit(merged.rstrip("\n"))
        emit(f"exit_code: {result.returncode}")
        outputs[title] = merged
        print(f"{title}: exit {result.returncode}")
        return result.returncode, merged

    emit(f"captured_at_utc: {utc()}")
    emit("claim: device JNI ANB1 vs JSON batch measurement, reported separately from host")
    emit(f"claim_boundary: execution on adb serial {serial} only")
    emit(
        "workspace_head: "
        + subprocess.run(
            ["git", "rev-parse", "HEAD"], cwd=REPO, text=True, capture_output=True
        ).stdout.strip()
    )
    emit(f"adb_path: {ADB}")
    emit(f"serial: {serial}")
    emit(f"app_apk_bytes: {APP_APK.stat().st_size}")
    emit(f"app_apk_sha256: {sha256(APP_APK)}")
    emit(f"test_apk_bytes: {TEST_APK.stat().st_size}")
    emit(f"test_apk_sha256: {sha256(TEST_APK)}")

    failures = 0
    code, _ = command("devices", ["devices", "-l"])
    failures += code
    # `am instrument -w` does not echo println output; clear logcat so the
    # single result line emitted by the test can be retained verbatim.
    failures += command("clear logcat", ["-s", serial, "logcat", "-c"])[0]
    for prop in PROPS:
        result = subprocess.run(
            [str(ADB), "-s", serial, "shell", "getprop", prop],
            text=True,
            capture_output=True,
        )
        props[prop] = result.stdout.strip()
        emit()
        emit(f"# getprop {prop} — {utc()}")
        emit(f"$ {ADB} -s {serial} shell getprop {prop}")
        emit(f"value: {props[prop]}")
        emit(f"exit_code: {result.returncode}")
        failures += result.returncode
    failures += command("install app APK", ["-s", serial, "install", "-r", str(APP_APK)])[0]
    failures += command("install test APK", ["-s", serial, "install", "-r", str(TEST_APK)])[0]
    code, instrument = command(
        "instrument benchmark",
        ["-s", serial, "shell", "am", "instrument", "-w", "-e", "class", TEST_CLASS, RUNNER],
    )
    failures += code
    _, logcat = command("read logcat", ["-s", serial, "logcat", "-d", "-s", "System.out:I"])

    result_payloads = [
        line.split("ANB1_VS_JSON_RESULT:", 1)[1]
        for line in logcat.splitlines()
        if "ANB1_VS_JSON_RESULT:" in line
    ]
    if len(result_payloads) != 1:
        emit()
        emit(f"error: expected one benchmark result line, found {len(result_payloads)}")
        log_arg.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")
        print(f"evidence written (incomplete): {log_arg}", file=sys.stderr)
        return 1
    measured = json.loads(result_payloads[0])

    emit()
    emit(f"captured_at_utc_end: {utc()}")
    emit(f"command_failures: {failures}")
    log_arg.write_text("\n".join(lines) + "\n", encoding="utf-8", newline="\n")

    evidence = {
        "captured_at_utc": utc(),
        "collector": {
            "script": "scripts/collect-android-benchmark-evidence.py",
            "command": (
                f"python scripts/collect-android-benchmark-evidence.py {serial} "
                f"{log_arg.as_posix()} {json_arg.as_posix()}"
            ),
            "workspace_head": next(
                line.split(":", 1)[1].strip()
                for line in lines
                if line.startswith("workspace_head:")
            ),
            "raw_log": log_arg.as_posix(),
            "adb_path": str(ADB),
            "serial": serial,
            "device_properties": props,
            "app_apk_bytes": APP_APK.stat().st_size,
            "app_apk_sha256": sha256(APP_APK),
            "test_apk_bytes": TEST_APK.stat().st_size,
            "test_apk_sha256": sha256(TEST_APK),
            "runner": RUNNER,
            "test_class": TEST_CLASS,
            "instrument_exit_code": code,
            "command_failures": failures,
            "raw_instrumentation_output": instrument,
        },
        "measurement": measured,
        "claim_boundary": {
            "device_jni_arm_only": True,
            "host_rust_arm": "reported separately at docs/benchmarks/anb1-vs-json/",
            "superiority_claim": "none",
        },
    }
    json_arg.write_text(
        json.dumps(evidence, indent=2, ensure_ascii=False) + "\n",
        encoding="utf-8",
        newline="\n",
    )
    print(f"evidence written: {log_arg} and {json_arg}")
    return 0 if failures == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
