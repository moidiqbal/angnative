# Architecture Decision Record: JavaScript Runtime Selection

## Status: Pending (Awaiting Phase G Benchmarks)

## Context

AngNative requires an embedded JavaScript engine to execute Angular applications natively. The engine must:
1. Execute Angular 22+ AOT-compiled code
2. Support Angular's zoneless change detection
3. Interface with Rust host via FFI
4. Run on Android (ARM64), iOS (ARM64), and Desktop (x64)
5. Meet binary size and startup performance budgets

## Decision Drivers

1. **Angular Runtime Compatibility** - Engine must execute Angular 22+ without modification
2. **Zoneless Change Detection** - Angular 18+ zoneless must work without Zone.js polyfill
3. **Binary Size** - Target < 5MB for mobile
4. **Startup Performance** - Cold start < 100ms, warm start < 20ms
5. **Memory Footprint** - Runtime heap < 10MB idle
6. **Cross-Platform** - Android ARM64, iOS ARM64, Desktop x64
7. **iOS JIT Constraints** - No JIT compilation on iOS
8. **Rust Integration** - Safe, ergonomic FFI
9. **Debugging** - Source maps, breakpoints, inspection
10. **Long-term Maintenance** - Active upstream, stable API

## Options Evaluated

### Option 1: QuickJS-NG (Current: rquickjs)

**Pros**:
- Tiny binary (~1.5MB stripped)
- Fast cold start (~20ms)
- Low memory (2-4MB)
- MIT license
- Pure Rust bindings (rquickjs)
- ES2023 support
- Bytecode compilation
- No JIT required (works on iOS)

**Cons**:
- No native Zone.js / zoneless support (requires shims)
- No native timer/microtask queue (requires shims)
- No native DOM/EventTarget (requires shims)
- No native fetch/XHR
- Interpreter only (no JIT)
- GC pauses scale with heap
- Single-threaded, no Workers
- Manual shim maintenance burden
- Limited debugging (no DevTools)

### Option 2: Hermes (Facebook)

**Pros**:
- Bytecode precompilation (hbc)
- Small binary (~3-4MB)
- JIT on Android, AOT on iOS
- Generational GC
- Used in production (React Native, Expo)
- ES2022+ support
- Snapshots

**Cons**:
- Larger binary (~3-4MB)
- Unofficial Rust bindings (hermes-rs incomplete)
- No Zone.js / zoneless (requires shims)
- iOS AOT compilation complexity
- Less Angular-specific testing
- Meta-controlled roadmap

### Option 3: V8 via deno_core

**Pros**:
- **Angular's native target** (V8/Chrome)
- **Zoneless works natively** (no Zone.js needed)
- V8 Snapshots (sub-10ms startup)
- Full ES2024 + WASM
- Incremental GC (low pause)
- DevTools debugging
- deno_core: excellent Rust embedding API
- TypeScript transpilation built-in
- Module loader trait
- Active Google + Deno maintenance

**Cons**:
- Larger binary (~8-15MB)
- No JIT on iOS (interpreter mode via rusty_v8)
- Larger memory footprint (~15MB baseline)
- Complex cross-platform build
- deno_core event loop integration (tokio)

### Option 4: JavaScriptCore (iOS/macOS only)

**Pros**:
- Native iOS/macOS integration
- JIT on macOS, interpreter on iOS
- Full ES2023

**Cons**:
- Apple platforms only
- Not cross-platform
- Low-level Rust bindings only

## Comparison Matrix

| Criterion | QuickJS-NG | Hermes | V8 (deno_core) |
|-----------|------------|--------|----------------|
| **Angular Native Target** | ❌ | ❌ | ✅ |
| **Zoneless Without Shims** | ❌ | ❌ | ✅ |
| **Binary Size** | ~1.5MB | ~3-4MB | ~8-15MB |
| **Cold Startup** | ~20ms | ~15ms | ~5ms (snapshot) |
| **Warm Startup** | ~5ms | ~8ms | ~1ms |
| **Memory (idle)** | 2-4MB | 4-8MB | 8-15MB |
| **GC Pause** | ~5ms | ~2ms | ~1ms (incremental) |
| **iOS Support** | ✅ | ✅ AOT | ⚠️ JIT-less |
| **Android JIT** | ❌ | ✅ | ⚠️ JIT-less |
| **Rust Integration** | Good (rquickjs) | Partial (hermes-rs) | **Excellent (deno_core)** |
| **Snapshots** | Bytecode | ✅ Full | ✅ **V8 Snapshots** |
| **Debugging** | Basic | Good | **DevTools** |
| **Long-term Maintenance** | Active (QuickJS-NG) | Active (Meta) | **Active (Google/Deno)** |

## Decision

**Pending Phase G benchmarks**. The ADR will be finalized after:

1. Phase C: Angular execution proof on QuickJS-NG
2. Phase G: Benchmarks on physical Android device
3. V8/deno_core proof-of-concept integration
4. Binary size / startup / memory measurements

## Provisional Recommendation

**Primary**: V8 via deno_core (if benchmarks confirm)
**Fallback**: QuickJS-NG (current, with shims)

## Consequences

### If V8 Selected:
- Binary size increases ~8MB
- Build complexity increases (V8 toolchain)
- iOS requires JIT-less V8 build
- Angular runs "natively" with minimal shims
- Best debugging experience
- Best long-term alignment with Angular

### If QuickJS-NG Retained:
- Binary stays ~2MB
- Continuous shim maintenance required
- Angular boot time longer
- Debugging more difficult
- Zone.js/zoneless shims permanent

## Next Steps

1. Complete Phase C (Angular on QuickJS-NG)
2. Implement V8/deno_core proof-of-concept
3. Phase G benchmarks on physical device
4. Finalize ADR with evidence

## References

- Angular v22 source: https://github.com/angular/angular
- NativeScript Angular: https://github.com/NativeScript/angular
- QuickJS-NG: https://github.com/quickjs-ng/quickjs
- rquickjs: https://github.com/DelSkayn/rquickjs
- Hermes: https://github.com/facebook/hermes
- deno_core: https://github.com/denoland/deno_core
- rusty_v8: https://github.com/denoland/rusty_v8
- Angular Zoneless: https://angular.dev/guide/zoneless