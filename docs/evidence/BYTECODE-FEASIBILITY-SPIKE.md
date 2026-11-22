# QuickJS bytecode feasibility spike

**Date:** 2026-11-23
**Outcome:** **Deferred — not implemented, not verified.**

## Finding

`rquickjs` 0.5.1 exposes bytecode dump and module-bytecode traits behind
feature flags. The presence of those traits is a library capability only: no
runtime bytecode loader exists in this repository, and no bytecode artifact is
produced or consumed by the supported path.

## Supported path today

AOT output is compiled to self-contained IIFE bundles and loaded as text into
the embedded QuickJS runtime. There is no `RuntimeController.loadBytecode()`
and no test that loads `qjsbc`-style bytecode. This spike is a feasibility
note, not an implementation or a performance result.

## Scope caveats

- No bytecode loading is claimed to work; startup-speed claims from bytecode
  are not made.
- ES module loading also remains deferred (P1-08); IIFE bundles are the
  supported input form.
- No new runtime, engine adapter or platform path was introduced by this spike.

**Related register row:** P2-02 in `docs/architecture/issue-register.md`.
