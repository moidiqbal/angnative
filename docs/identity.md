# AngNative — Canonical Identity (P0.1)

**Status:** DECIDED 2026-11-14 · recorded by Kilo Code (executing TaskList.txt Phase 0)

## Decision

| Item | Value | Source |
|---|---|---|
| Product name | **AngNative** | Repo name `angnative`; TaskList.txt D-2 |
| npm scope | `@angnative/*` | TaskList.txt D-2; never `@angular/*`, never `ng-native` (taken upstream) |
| Element prefix | `na-` (e.g. `na-view`, `na-text`) | TaskList.txt D-2 / spec D9 |
| Licence | **MIT** | TaskList.txt D-3; spec §14/§15 |
| Backend | Rust host is the only native backend (no RN/Fabric/Expo) | TaskList.txt D-1 |
| Layout owner | Rust (taffy) lays out; platform draws | TaskList.txt D-4 |
| JS engine | Engine-neutral trait; QuickJS-ng default; JSC on Apple | TaskList.txt D-5 |
| Web | Uses Angular's own DOM renderer (not the native renderer) | TaskList.txt D-6 |
| Scope order | Android → iOS → macOS → watchOS/tvOS → Windows/Linux | TaskList.txt D-7 |

## Reconciliation notes

- **Code vs. decision (prefix):** `crates/angnative-core/src/node.rs` currently parses
  `native-view` / `native-text` / `view` / `div` tags (`from_tag`). The canonical prefix
  is `na-`. Migration of the tag vocabulary to `na-*` is tracked under Phase 1, Task T4
  (fixture migration) and Task T1 (contract). Until then, `from_tag` accepts both forms
  so nothing breaks; new code must use `na-*`.
- **Code vs. decision (licence):** `Cargo.toml` previously declared `MIT OR Apache-2.0`;
  corrected to `MIT` to match `package.json` and the spec. `docs/` and `README.md` already
  say MIT.
- **Code vs. decision (scope):** `package.json` already uses `@angnative/*` and MIT.
  Rust crate names use `angnative-*` (no `@`), which is correct for Cargo.

## D-8 (Rust-first) — decided

Rust-first is the decided direction (TaskList.txt D-1, Q3 default). The 1 MB per-platform
stripped+LTO static library budget (spec D-8) stands as a hard constraint. The measured
number is produced by Phase 2, Task R8 — it is not estimated.

## D-9 (element prefix) — decided

`na-` is the canonical element prefix (spec D9). See reconciliation note above re: the
current `native-*` parsing in `angnative-core`.

## Open questions (block tasks, see TaskList.txt §7)

- Q1 canonical identity recorded here as AngNative (default applied).
- Q2 M1 tree recoverability — still unverified; P0.2 tracks it.
- Q4 Windows/Linux desktop in scope — deferred to P3.5.
- Q5 engine choice — JSC on Apple, QuickJS-ng elsewhere (default applied).
- Q6 device availability — assume emulator-only until stated otherwise.
