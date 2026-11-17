# R8 — Rust Core Size Budget (≤1 MB per platform)

## Status
**Measurement tooling implemented** — actual measurement requires `cargo`/`rustc`
(available where the toolchain exists, not in this file-level sandbox).

## What was built

| Deliverable | File | Status |
|-------------|------|--------|
| Release profile with LTO + strip | `Cargo.toml` `[profile.release]` | ✅ |
| Size measurement script | `scripts/check-rust-size.mjs` | ✅ |
| Package.json script | `package.json` `"check:rust-size"` | ✅ |
| Actual measured number | — | ⏳ Needs cargo run |

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
angnative-host-engine` per target platform and enforces the 1 MB budget.

**Usage:**
```bash
# Measure all default targets
pnpm run check:rust-size

# Custom budget (bytes)
pnpm run check:rust-size --budget-bytes 1048576

# Specific targets only
pnpm run check:rust-size --target aarch64-apple-ios --target aarch64-linux-android
```

**Default targets:**
`x86_64-unknown-linux-gnu`, `aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-apple-ios`, `aarch64-linux-android`, `armv7-linux-androideabi`,
`x86_64-pc-windows-msvc`

**Exit codes:** `0` = all within budget, `1` = any over budget or build failure.

## Verification pipeline (TaskList §5)

Add to the per-commit verification pipeline:

```
9. pnpm run check:rust-size → ≤ 1 MB stripped+LTO per platform (D8's number)
```

## Identity D-8

> The 1 MB per-platform stripped+LTO static library budget stands as a hard
> constraint. The measured number is produced by Phase 2, Task R8 — it is not
> estimated. If it blows past 1 MB, that is the finding — not a reason to move
> the goalpost.

## Next step
Run `pnpm run check:rust-size` on a machine with `cargo` + `rustc` installed
to produce the measured D-8 number. Record it here once available.
