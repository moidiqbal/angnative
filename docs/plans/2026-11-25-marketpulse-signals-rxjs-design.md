# MarketPulse — Signals + RxJS stream design (2026-11-25)

## Scope

Add reactive state and a polled quote stream to the existing MarketPulse Android
demo without claiming real-time delivery or adding a browser WebSocket to
QuickJS. Delay is acceptable; honesty about the source is not optional.

## Decision

- **Angular Signals** hold quote, status and flash state. `computed()` derives
  symbol, last price, bar count and the `attr:bars` JSON string. This replaces
  the previous plain mutable fields while keeping the same native text nodes.
- **RxJS** is used as the in-process stream: `Subject<string>` receives normalized
  quote JSON from initial `HttpClient`, host refresh pushes and the `Demo bars`
  fixture. The subscription parses and validates before writing signals;
  operators are intentionally not imported because the workspace resolves RxJS 6
  from the parent `node_modules` while Angular uses RxJS 7 in pnpm's store.
- **Polling lives in Kotlin**, not in a QuickJS timer. The QuickJS timer shim in
  `crates/angnative-host-engine/src/quickjs_shims.rs:691-728` deliberately makes
  `setInterval` one-shot; an RxJS `interval` would create an unbounded microtask
  chain and starve the bounded host job loop. A Kotlin
  `ScheduledExecutorService` fetches every 30 s and pushes through the existing
  `__marketPulseApplyQuote` hook. No new JNI async method is introduced.

## Data flow

1. `MainActivity` prefetches the first CoinGecko BTC/USD OHLC payload and boots
   the AOT bundle with a synchronous fetch handler.
2. `MarketPulseFeed.startPolling(30_000)` schedules background re-fetches. Each
   successful payload is posted to the main looper and pushed into QuickJS with
   `__marketPulseApplyQuote(json)`.
3. Angular's `Subject` → `map` → `filter` → `signal.set()` updates the same
   native nodes; `cdr.detectChanges()` still flushes the shadow tree.
4. Manual `Refresh` keeps the existing native `action=refresh` path; `Demo bars`
   remains a clearly labelled deterministic fixture.

## Verification

- Ignored Rust acceptance test: assert RxJS stream event counter increments on
  two pushes, price node id stays stable, native node count does not grow.
- Emulator: observe a polled refresh log line, unchanged node count, and the
  existing green/red candle pixels.
- Full gates: `pnpm test`, `cargo test --workspace`, ignored MarketPulse test,
  Android instrumentation, FFI declaration check, `git diff --check`.

## Not claimed

WebSocket/SSE, true real-time ticks, CoinGecko volume, smooth easing, physical
device or iOS/Mac results.
