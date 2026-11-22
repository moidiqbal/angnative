# R8 — Rust Core Size Budget (≤1 MB per platform)

## Status
**Measurement tooling implemented and wired into CI** (2026-11-23). The budget
is read from `rustCoreMaxBytesPerPlatform` in `packages/cli/src/budgets.json`
(currently 1,048,576 bytes); no hardcoded 1 MB default remains in the gate
script. No local or CI size measurement result is claimed by this document —
actual measurement requires `cargo`/`rustc` and a populated runner. See
`docs/evidence/RUST-SIZE-GATE.md`.

## What was built

| Deliverable | File | Status |
|-------------|------|--------|
| Release profile with LTO + strip | `Cargo.toml` `[profile.release]` | ✅ |
| Size measurement script | `scripts/check-rust-size.mjs` | ✅ |
| Package.json script | `package.json` `"check:rust-size"` | ✅ |
| Budget sourced from `packages/cli/src/budgets.json` | `scripts/check-rust-size.mjs` | ✅ |
| CI `size` job wiring | `.github/workflows/ci.yml` | ✅ |
| Actual measured number | — | Not claimed by this document |

## Release profile (`Cargo.toml`)

```toml
[profile.release]
opt-level = "z"           # Optimize for binary size
lto = "fat"               # Link-time optimization across crate graph
codegen-units = 1         # Better optimization, slower compile
panic = "abort"           # Smaller binaries (no unwind tables)
strip = true              # Strip symbols from release binaries
```

## Size measurement script (`scripts/check-rust-size.mjs`)

Measures the stripped + LTO static library size of
`angnative-host-core + angnative-host-layout + angnative-host-view +
angnative-host-engine` per target platform and enforces the budget from
`packages/cli/src/budgets.json` (`rustCoreMaxBytesPerPlatform`).

**Usage:**
```bash
# Measure all default targets
pnpm run check:rust-size

# Override budget (bytes); default comes from budgets.json
pnpm run check:rust-size --budget-bytes 1048576

# Host target — the form used by the CI `size` job
node scripts/check-rust-size.mjs --host

# Explicit installed targets only
node scripts/check-rust-size.mjs --target aarch64-linux-android
```

**Default targets:**
`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-apple-ios`, `aarch64-linux-android`, `armv7-linux-androideabi`,
`x86_64-pc-windows-msvc`

**Explicit skips:** the CI `size` job runs the host target only where the
pinned Rust toolchain is installed. Apple (`aarch64-apple-darwin`,
`x86_64-apple-darwin`, `aarch64-apple-ios`) and Windows
(`x86_64-pc-windows-msvc`) targets are skipped on the Linux runner because
those targets/toolchains are unavailable there. Android targets require an
explicitly installed target; no cross-target CI measurement is claimed.

**Exit codes:** `0` = all within budget, `1` = any over budget or build failure.

## Verification pipeline (TaskList §5)

The per-commit verification pipeline now includes the Rust size gate in the
`.github/workflows/ci.yml` `size` job, using the pinned Rust toolchain and
`node scripts/check-rust-size.mjs --host` (or an explicit installed target):

```
9. node scripts/check-rust-size.mjs --host → ≤ budget from budgets.json (D8's number)
```

## Identity D-8

> The 1 MB per-platform stripped+LTO static library budget stands as a hard
> constraint. The measured number is produced by Phase 2, Task R8 — it is not
> estimated. If it blows past 1 MB, that is the finding — not a reason to move
> the goalpost.

## Next step
Run `pnpm run check:rust-size` (or `node scripts/check-rust-size.mjs --host`)
on a machine with `cargo` + `rustc` installed to produce the measured D-8
number. Record it here once available; this document does not claim a measured
local or CI result.
