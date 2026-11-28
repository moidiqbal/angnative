"""Retain real-browser save -> SSE -> DOM evidence for the web HMR client.

This harness uses the REAL DevServer (no compileHook) and the REAL
startWebHmr client (no subscribe/loadModule/fetchImpl overrides). It writes a
generated Angular AOT fixture, waits for the real SSE event and blob-module
replacement, and records browser-side timestamps. The claim it supports is
"web HMR client under a harness", not shipping dev-app auto-reload.

Run from anywhere:
    python scripts/verify-hmr-browser.py
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import shutil
import subprocess
import sys
import threading
import time
from datetime import datetime, timezone
from pathlib import Path
from urllib.request import urlopen

REPO = Path(__file__).resolve().parents[1]
EVIDENCE_DIR = REPO / "dist" / "hmr-browser-evidence"
SRC = EVIDENCE_DIR / "src" / "counter.component.ts"
TSCONFIG = EVIDENCE_DIR / "tsconfig.json"
RUNNER = EVIDENCE_DIR / "dev-server.mjs"
HARNESS = EVIDENCE_DIR / "harness.js"
PORT_FILE = EVIDENCE_DIR / "port.txt"

# Each run writes a NEW dated artifact so the honest earlier failed record is
# never overwritten by a later pass (or vice versa).
PARSER = argparse.ArgumentParser(description=__doc__)
PARSER.add_argument(
    "--stamp",
    default=datetime.now().strftime("%Y%m%d-%H%M%S"),
    help="timestamp used in retained evidence filenames",
)
STAMP = PARSER.parse_args().stamp
DOC_JSON = REPO / "docs" / "evidence" / f"hmr-browser-save-sse-dom-{STAMP}.json"
DOC_LOG = REPO / "docs" / "assets" / f"hmr-browser-dev-server-{STAMP}.log"
DOC_SSE = REPO / "docs" / "assets" / f"hmr-browser-sse-{STAMP}.txt"
DOC_BEFORE = REPO / "docs" / "assets" / f"hmr-browser-before-{STAMP}.png"
DOC_AFTER = REPO / "docs" / "assets" / f"hmr-browser-after-{STAMP}.png"

MEASURED_RUNS = 10
BUDGET_MS = 1000

FIXTURE_TEMPLATE = """/**
 * Generated HMR evidence fixture — variant __VARIANT__.
 * Produced by scripts/verify-hmr-browser.py; never committed.
 */
import { CUSTOM_ELEMENTS_SCHEMA, Component, signal } from '@angular/core';

@Component({
  selector: 'na-hmr-counter',
  template: '<na-view><na-text data-variant="__VARIANT__">{{ count() }}</na-text></na-view>',
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class HmrCounterComponent {
  readonly count = signal(0);

  variant(): string { return '__VARIANT__'; }
  setCount(value: number): void { this.count.set(value); }
  snapshot(): { count: number } { return { count: this.count() }; }
  restore(state: unknown): void {
    const count = (state as { count?: number } | undefined)?.count;
    if (typeof count === 'number') this.count.set(count);
  }
}

export function bootstrap(): HmrCounterComponent { return new HmrCounterComponent(); }
"""

TSCONFIG_TEMPLATE = {
    "extends": "../../tsconfig.json",
    "compilerOptions": {
        "outDir": "out",
        "rootDir": "src",
        "declaration": False,
        "sourceMap": False,
        "inlineSources": False,
    },
    "angularCompilerOptions": {
        "enableIvy": True,
        "enableResourceInlining": True,
        "skipTemplateCodegen": False,
        "strictMetadataEmit": False,
        "fullTemplateTypeCheck": False,
    },
    "files": ["src/counter.component.ts"],
    "include": ["src/**/*.ts"],
    "exclude": [],
}

RUNNER_TEMPLATE = """/**
 * Generated DevServer runner — no compileHook, ephemeral port, real ngc.
 */
import { DevServer } from '../../packages/cli/dist/dev.js';
import { writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const dir = fileURLToPath(new URL('.', import.meta.url));
const server = new DevServer({
  entry: join(dir, 'out', 'counter.component.js'),
  outfile: join(dir, 'out', 'dev-bundle.js'),
  nativeOutfile: join(dir, 'out', 'dev-bundle.native.js'),
  port: 0,
  aotTsconfig: join(dir, 'tsconfig.json'),
  watchPaths: [join(dir, 'src', 'counter.component.ts')],
  hmrModuleId: 'root',
});
await server.start();
writeFileSync(join(dir, 'port.txt'), String(server.getPort()));
setInterval(() => {}, 1 << 30);
"""

HTML = """<!doctype html>
<html lang="en"><head><meta charset="utf-8"><title>HMR evidence</title></head>
<body><div id="state">booting</div><script type="module" src="/harness.js"></script></body></html>
"""


def utc() -> str:
    return datetime.now(timezone.utc).isoformat()


def sha256(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest().upper()


def run(cmd: list[str], **kwargs) -> subprocess.CompletedProcess:
    if cmd and cmd[0] == "pnpm":
        # pnpm is a .CMD shim on Windows; CreateProcess cannot launch it.
        resolved = shutil.which("pnpm") or "pnpm"
        shell_cmd = " ".join(f'"{part}"' for part in [resolved, *cmd[1:]])
        return subprocess.run(shell_cmd, cwd=REPO, text=True, capture_output=True, shell=True, **kwargs)
    return subprocess.run(cmd, cwd=REPO, text=True, capture_output=True, **kwargs)


def esbuild_entry() -> Path:
    store = REPO / "node_modules" / ".pnpm"
    matches = sorted(store.glob("esbuild@*/node_modules/esbuild/bin/esbuild"))
    if not matches:
        raise FileNotFoundError("esbuild JS entry not found in the pnpm store")
    return matches[-1]


def write_variant(variant: str) -> None:
    # Atomic save: truncate-in-place produces two ngc watch notifications (one
    # for the truncation, one for the content), which would add a whole extra
    # compile cycle to the measured interval.
    # Keep the temp file outside `src/` so ngc's directory watcher does not
    # observe its creation as a second source change.
    temp = EVIDENCE_DIR / "counter.component.ts.tmp"
    with temp.open("w", encoding="utf-8", newline="\n") as handle:
        handle.write(FIXTURE_TEMPLATE.replace("__VARIANT__", variant))
    os.replace(temp, SRC)


def sse_reader(port: int, stop: threading.Event, path: Path, holder: dict) -> list[str]:
    """Read the raw SSE stream. The main thread closes `holder['response']`
    to unblock the buffered readline when the measurement is finished."""
    lines: list[str] = []
    try:
        response = urlopen(f"http://127.0.0.1:{port}/hmr")
        holder["response"] = response
        with response, path.open("w", encoding="utf-8", newline="\n") as out:
            while not stop.is_set():
                raw = response.readline()
                if not raw:
                    break
                line = raw.decode("utf-8", "replace").rstrip("\r\n")
                out.write(f"{utc()} {line}".rstrip() + "\n")
                out.flush()
                lines.append(line)
    except Exception as exc:  # closed by the main thread during cleanup
        if not stop.is_set():
            raise
        lines.append(f"reader stopped: {exc}")
    return lines


def main() -> int:
    # Windows ngc/Chromium descendants occasionally keep a shell invocation
    # alive after Python finishes; this guarantees a bounded command.
    def hard_exit_watchdog() -> None:
        time.sleep(300)
        print("watchdog: hard exit after 300 s", flush=True)
        os._exit(99)

    threading.Thread(target=hard_exit_watchdog, daemon=True).start()

    EVIDENCE_DIR.mkdir(parents=True, exist_ok=True)
    DOC_JSON.parent.mkdir(parents=True, exist_ok=True)
    DOC_LOG.parent.mkdir(parents=True, exist_ok=True)

    SRC.parent.mkdir(parents=True, exist_ok=True)
    write_variant("v1")
    TSCONFIG.write_text(json.dumps(TSCONFIG_TEMPLATE, indent=2) + "\n", encoding="utf-8", newline="\n")
    RUNNER.write_text(RUNNER_TEMPLATE, encoding="utf-8", newline="\n")

    print("phase: esbuild start", flush=True)
    esbuild = run([
        "node", str(esbuild_entry()), "scripts/hmr-browser/harness-entry.ts",
        "--bundle", f"--outfile={HARNESS.relative_to(REPO)}", "--format=esm",
        "--target=es2022", "--platform=browser", "--define:ngDevMode=true",
        "--log-level=warning",
    ])
    if esbuild.returncode != 0:
        print(esbuild.stdout[-4000:], esbuild.stderr[-4000:], file=sys.stderr)
        return 1

    bundle_hash = sha256(HARNESS)
    fixture_hash_before = sha256(SRC)
    log_handle = DOC_LOG.open("w", encoding="utf-8", newline="\n")
    log_handle.write(f"{utc()} harness esbuild command: {' '.join(esbuild.args)}\n")
    log_handle.write(f"{utc()} harness bundle sha256: {bundle_hash}\n")
    log_handle.flush()

    PORT_FILE.unlink(missing_ok=True)  # never reuse a previous run's ephemeral port
    server = subprocess.Popen(
        ["node", str(RUNNER.relative_to(REPO))],
        cwd=REPO,
        stdout=log_handle,
        stderr=subprocess.STDOUT,
    )
    stop_sse = threading.Event()
    sse_lines: list[str] = []
    sse_holder: dict = {}
    sse_thread: threading.Thread | None = None
    page_errors: list[str] = []
    console: list[str] = []
    requested: list[str] = []

    try:
        deadline = time.time() + 90
        while not PORT_FILE.exists() and time.time() < deadline:
            if server.poll() is not None:
                raise RuntimeError("dev server exited before writing its port")
            time.sleep(0.2)
        if not PORT_FILE.exists():
            raise TimeoutError("dev server did not write port.txt within 90 s")
        port = int(PORT_FILE.read_text(encoding="utf-8").strip())
        print(f"phase: server port {port}", flush=True)
        log_handle.write(f"{utc()} dev server port: {port}\n")
        log_handle.flush()

        sse_thread = threading.Thread(
            target=lambda: sse_lines.extend(sse_reader(port, stop_sse, DOC_SSE, sse_holder)),
            daemon=True,
        )
        sse_thread.start()

        from playwright.sync_api import sync_playwright

        with sync_playwright() as playwright:
            browser = playwright.chromium.launch(headless=True)
            context = browser.new_context(viewport={"width": 1000, "height": 700})
            page = context.new_page()
            page.on("pageerror", lambda error: page_errors.append(str(error)))
            page.on("console", lambda message: console.append(f"{message.type}: {message.text}"))

            def route_handler(route) -> None:
                url = route.request.url
                requested.append(url)
                path = url.split("?", 1)[0]
                if path.endswith(f":{port}/") or path.endswith(f":{port}"):
                    route.fulfill(status=200, content_type="text/html", body=HTML)
                elif path.endswith("/harness.js"):
                    route.fulfill(
                        status=200,
                        content_type="application/javascript",
                        body=HARNESS.read_text(encoding="utf-8"),
                    )
                else:
                    route.continue_()

            page.route("**/*", route_handler)
            print("phase: page goto", flush=True)
            page.goto(f"http://127.0.0.1:{port}/", wait_until="domcontentloaded")
            print("phase: page loaded", flush=True)

            def wait_ready() -> None:
                deadline = time.time() + 60
                ready = False
                while time.time() < deadline:
                    ready = page.evaluate(
                        "() => !!(window.__hmrState && (window.__hmrState.ready || window.__hmrState.error))"
                    )
                    if ready:
                        break
                    time.sleep(0.1)
                if not ready:
                    raise TimeoutError("harness did not boot within 60 s")
                error = page.evaluate("() => window.__hmrState.error")
                if error:
                    print("BOOT DIAGNOSTICS console:", console[-30:], file=sys.stderr, flush=True)
                    print("BOOT DIAGNOSTICS pageerror:", page_errors, file=sys.stderr, flush=True)
                    print("BOOT DIAGNOSTICS requested:", requested[-30:], file=sys.stderr, flush=True)
                    raise RuntimeError(f"harness boot failed: {error}")

            print("phase: waiting ready", flush=True)
            wait_ready()
            print("phase: ready", flush=True)
            initial = page.evaluate("() => JSON.parse(JSON.stringify(window.__hmrState))")
            realm_token = initial["realmToken"]
            navigation_count = initial["navigationCount"]

            def latest_version(key: str) -> int:
                entries = page.evaluate(f"() => window.__hmrState.{key}")
                return max((entry["version"] for entry in entries), default=0)

            def poll_new(key: str, baseline: int) -> dict:
                # Explicit Python polling: Playwright's wait_for_function can
                # stall on a page with an open EventSource, and a hard deadline
                # keeps the evidence run bounded.
                deadline = time.time() + 30
                while time.time() < deadline:
                    entries = page.evaluate(f"() => window.__hmrState.{key}")
                    newer = [entry for entry in entries if entry["version"] > baseline]
                    if newer:
                        return newer[0]
                    time.sleep(0.05)
                raise TimeoutError(f"no new {key} entry after version {baseline} within 30 s")

            def wait_new_applied(baseline: int) -> dict:
                return poll_new("applied", baseline)

            def wait_new_sse(baseline: int) -> dict:
                return poll_new("sse", baseline)

            def wait_applied_version(version: int) -> dict:
                # Match by version, not by array position: queued or stale
                # entries must never be paired with a different SSE event.
                deadline = time.time() + 30
                while time.time() < deadline:
                    matches = [
                        entry
                        for entry in page.evaluate("() => window.__hmrState.applied")
                        if entry["version"] == version
                    ]
                    if matches:
                        return matches[0]
                    time.sleep(0.01)
                raise TimeoutError(f"no applied entry for version {version} within 30 s")

            def settle(quiet_ms: float = 300, timeout_s: float = 5) -> None:
                """Confirm one write produced no extra queued reload cycles."""
                deadline = time.time() + timeout_s
                last_count = None
                last_change = time.time()
                while time.time() < deadline:
                    counts = page.evaluate(
                        """() => [
                             window.__hmrState.applied.length,
                             window.__hmrState.sse.length,
                           ]"""
                    )
                    if counts != last_count:
                        last_count = counts
                        last_change = time.time()
                    elif (time.time() - last_change) * 1000 >= quiet_ms:
                        return
                    time.sleep(0.05)
                raise TimeoutError(f"HMR streams did not settle within {timeout_s} s")

            # Warm-up update is retained but excluded from timing statistics.
            print("phase: warm-up write", flush=True)
            write_variant("v2")
            warm = wait_new_applied(0)
            settle()
            print(f"phase: warm-up applied {warm['version']} {warm['text']!r}", flush=True)
            if warm["text"] != "v2:0":
                raise AssertionError(f"warm-up DOM text unexpected: {warm['text']!r}")

            records: list[dict] = []
            variant = "v1"
            page.screenshot(path=str(DOC_BEFORE), full_page=True)
            for index in range(1, MEASURED_RUNS + 1):
                page.evaluate("() => window.__hmrSetCount(7)")
                text_before = page.evaluate("() => document.getElementById('state').textContent")
                if text_before != f"{'v2' if index % 2 == 1 else 'v1'}:7":
                    raise AssertionError(f"pre-write DOM text unexpected: {text_before!r}")

                sse_baseline = latest_version("sse")
                expected_text = f"{variant}:7"
                print(f"phase: run {index} write {variant}, sse baseline {sse_baseline}", flush=True)
                write_variant(variant)
                t0 = time.time() * 1000  # recorded after the write returns
                sse = wait_new_sse(sse_baseline)
                version = sse["version"]
                applied = wait_applied_version(version)
                dom_at = applied["at"]
                settle()  # post-measurement duplicate check; not part of t0→DOM

                state_after = page.evaluate("() => JSON.parse(JSON.stringify(window.__hmrState))")
                requested_urls = list(requested)
                sse_for_version = [entry for entry in state_after["sse"] if entry["version"] == version]
                applied_for_version = [entry for entry in state_after["applied"] if entry["version"] == version]
                single_event = len(sse_for_version) == 1 and len(applied_for_version) == 1

                if applied["text"] != expected_text:
                    raise AssertionError(
                        f"run {index}: state preservation mismatch: "
                        f"expected {expected_text!r}, got {applied['text']!r}; "
                        f"all applied={json.dumps(state_after['applied'])}; "
                        f"probe={json.dumps(page.evaluate('() => window.__hmrProbe()'))}"
                    )
                if not single_event:
                    raise AssertionError(
                        f"run {index}: expected one SSE + one apply for v{version}, got "
                        f"{len(sse_for_version)}/{len(applied_for_version)}"
                    )
                if not sse["url"].endswith(f"?v={version}"):
                    raise AssertionError(f"run {index}: SSE url {sse['url']!r}")
                if not any(f"/dev-bundle.js?v={version}" in url for url in requested_urls):
                    raise AssertionError(f"run {index}: versioned bundle was not requested")
                if state_after["realmToken"] != realm_token:
                    raise AssertionError(f"run {index}: realm token changed (full reload)")
                if state_after["navigationCount"] != navigation_count:
                    raise AssertionError(f"run {index}: navigation count changed (full reload)")
                if page_errors:
                    raise AssertionError(f"run {index}: page errors: {page_errors}")

                records.append({
                    "run": index,
                    "variant": variant,
                    "version": version,
                    "sse_version": sse["version"],
                    "applied_version": applied["version"],
                    "t0_epoch_ms": round(t0, 1),
                    "sse_epoch_ms": sse["at"],
                    "dom_epoch_ms": dom_at,
                    "sse_minus_t0_ms": round(sse["at"] - t0, 1),
                    "dom_minus_t0_ms": round(dom_at - t0, 1),
                    "dom_minus_sse_ms": round(dom_at - sse["at"], 1),
                    "state_preserved": applied["text"] == expected_text,
                    "dom_text_before": text_before,
                    "dom_text_after": applied["text"],
                    "bundle_url": sse["url"],
                    "realm_token_unchanged": state_after["realmToken"] == realm_token,
                    "navigation_count": state_after["navigationCount"],
                    "single_event_per_write": single_event,
                    "sse_entries_for_version": len(sse_for_version),
                    "applied_entries_for_version": len(applied_for_version),
                })
                variant = "v2" if variant == "v1" else "v1"
                print(
                    f"run {index:2d}: save-to-dom = {records[-1]['dom_minus_t0_ms']:7.1f} ms, "
                    f"sse-to-dom = {records[-1]['dom_minus_sse_ms']:6.1f} ms, "
                    f"single-event = {single_event}",
                    flush=True,
                )

            page.screenshot(path=str(DOC_AFTER), full_page=True)
            browser.close()

        timings = [record["dom_minus_t0_ms"] for record in records]
        ordered = sorted(timings)
        p50 = ordered[len(ordered) // 2]
        p95 = ordered[min(len(ordered) - 1, int(len(ordered) * 0.95))]

        git_head = run(["git", "rev-parse", "HEAD"]).stdout.strip()
        git_status = run(["git", "status", "--short"]).stdout
        git_diff_hash = hashlib.sha256(
            subprocess.run(["git", "diff"], cwd=REPO, capture_output=True).stdout
        ).hexdigest().upper()
        node_version = run(["node", "--version"]).stdout.strip()
        pnpm_version = run(["pnpm", "--version"]).stdout.strip()
        python_version = platform.python_version()

        evidence = {
            "captured_at_utc": utc(),
            "claim": "web HMR client under a harness",
            "claim_boundary": (
                "Real DevServer, resident real ngc watch completion as the only real-path trigger, "
                "real EventSource from the same client, real fetch and blob-module import, real "
                "DOM mutation, immutable per-version bundles. Product wiring for auto-reload is "
                "NOT promoted; ANB1 is not used on the web SSE path."
            ),
            "pass_criterion": f"all {MEASURED_RUNS} measured dom_minus_t0_ms < {BUDGET_MS} ms",
            "passed": all(value < BUDGET_MS for value in timings),
            "summary": {
                "runs": len(records),
                "save_to_dom_min_ms": ordered[0],
                "save_to_dom_p50_ms": p50,
                "save_to_dom_p95_ms": p95,
                "save_to_dom_max_ms": ordered[-1],
                "sse_to_dom_min_ms": min(r["dom_minus_sse_ms"] for r in records),
                "sse_to_dom_max_ms": max(r["dom_minus_sse_ms"] for r in records),
                "state_preserved_runs": sum(1 for r in records if r["state_preserved"]),
                "single_event_runs": sum(1 for r in records if r["single_event_per_write"]),
                "realm_token_unchanged_runs": sum(
                    1 for r in records if r["realm_token_unchanged"]
                ),
            },
            "claims": {
                "save_to_sse_to_dom_under_1s": {
                    "verified": all(value < BUDGET_MS for value in timings),
                    "note": (
                        "Verified: full save -> real ngc watch completion -> bundle -> SSE -> "
                        "DOM measurement stayed under the stated budget."
                        if all(value < BUDGET_MS for value in timings)
                        else "Not verified: at least one measured save-to-DOM interval exceeded the budget."
                    ),
                },
                "named_sse_event_delivered_to_real_client": True,
                "local_state_survives_all_measured_replacements": all(
                    r["state_preserved"] for r in records
                ),
                "no_full_page_reload": all(r["realm_token_unchanged"] for r in records),
                "product_dev_app_auto_reload": "not verified (library-only wiring)",
                "anb1_on_web_sse_path": "not applicable (ANB1 is Host/JNI only)",
            },
            "warmup": {
                "version": warm["version"],
                "dom_text": warm["text"],
                "dom_epoch_ms": warm["at"],
                "excluded_from_statistics": True,
            },
            "runs": records,
            "environment": {
                "git_head": git_head,
                "git_status_short": git_status,
                "git_diff_sha256": git_diff_hash,
                "node": node_version,
                "pnpm": pnpm_version,
                "python": python_version,
                "playwright": __import__("importlib.metadata", fromlist=["version"]).version("playwright"),
                "chromium_executable": None,
                "chromium_version": None,
                "os": f"{platform.system()} {platform.release()} {platform.machine()}",
                "fixture_sha256_before": fixture_hash_before,
                "fixture_sha256_after": sha256(SRC),
                "harness_bundle_sha256": bundle_hash,
                "harness_entry_sha256": sha256(REPO / "scripts" / "hmr-browser" / "harness-entry.ts"),
                "dev_server_command": f"node {RUNNER.relative_to(REPO)}",
                "esbuild_command": " ".join(esbuild.args),
                "no_compile_hook": True,
                "no_injected_subscribe_loadmodule_fetchimpl": True,
                "real_ngc_watch_completion_only_trigger": True,
                "duplicate_watch_events_allowed": False,
                "immutable_versioned_bundles": True,
                "sse_timing_source": "same-client EventSource wrapper (no second observer)",
                "raw_sse_transcript_is_observer_only": True,
            },
            "console": console[-200:],
            "page_errors": page_errors,
            "requested_urls_tail": requested[-200:],
            "sse_transcript_lines": len(sse_lines),
            "artifacts": {
                "dev_server_log": str(DOC_LOG.relative_to(REPO)),
                "sse_transcript": str(DOC_SSE.relative_to(REPO)),
                "screenshot_before": str(DOC_BEFORE.relative_to(REPO)),
                "screenshot_after": str(DOC_AFTER.relative_to(REPO)),
            },
        }

        # Chromium identity is captured after the browser run from the same
        # cached executable Playwright used.
        try:
            from playwright.sync_api import sync_playwright as _sp

            with _sp() as pw:
                browser = pw.chromium.launch(headless=True)
                evidence["environment"]["chromium_executable"] = pw.chromium.executable_path
                evidence["environment"]["chromium_version"] = browser.version
                browser.close()
        except Exception as exc:  # evidence must stay honest if this fails
            evidence["environment"]["chromium_identity_error"] = str(exc)

        DOC_JSON.write_text(
            json.dumps(evidence, indent=2, ensure_ascii=False) + "\n", encoding="utf-8", newline="\n"
        )
        print(f"evidence written: {DOC_JSON.relative_to(REPO)}")
        print(f"p50={p50} ms p95={p95} ms max={ordered[-1]} ms passed={evidence['passed']}")
        return 0 if evidence["passed"] else 1

    finally:
        stop_sse.set()
        response = sse_holder.get("response")
        if response is not None:
            try:
                response.close()
            except Exception:
                pass
        if sse_thread is not None:
            sse_thread.join(timeout=5)
        if platform.system() == "Windows":
            # ngc child processes inherit the server's handles; terminate the
            # whole tree or the shell call can hang after Python exits.
            subprocess.run(
                ["taskkill", "/PID", str(server.pid), "/T", "/F"],
                capture_output=True,
            )
        server.terminate()
        try:
            server.wait(timeout=10)
        except subprocess.TimeoutExpired:
            server.kill()
            server.wait(timeout=10)
        log_handle.write(f"{utc()} dev server terminated\n")
        log_handle.close()


if __name__ == "__main__":
    # Playwright's driver can keep a non-daemon thread alive on Windows after
    # main() has already cleaned up. Flush and exit explicitly so the harness
    # command terminates promptly and retains a trustworthy exit code.
    code = main()
    sys.stdout.flush()
    sys.stderr.flush()
    os._exit(code)
