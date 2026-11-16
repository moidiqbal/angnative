# NativeAngular — Design Spec

**Status:** Design (v0.1) · **Date:** 2026-11-11 · **License:** MIT
**Product:** NativeAngular · **npm scope:** `@nativeangular/*` · **Positioning:** "the native runtime for Angular"

---

## 1. Executive summary

NativeAngular is an open-source framework that renders **Angular components as real native views** — no WebView — on phones, watches, TV and desktop, from **one Angular codebase**.

Its wedge is not the renderer (two exist already). Its wedge is a **capability-aware runtime**: every platform declares what it can do, every component declares what it needs, and the framework resolves the best implementation at runtime and degrades gracefully instead of forking the app per platform.

The core is backend-agnostic. A **backend SPI** (built on Angular's public `Renderer2` / `RendererFactory2` extension point) lets the same Angular app run on different native backends:

- **Fabric backend** (v0.1) — React Native's Fabric renderer via the Expo runtime → iOS + Android, full RN native-module ecosystem.
- **Rust core backend** (v0.2+) — own Rust UI tree, layout and layout engine → watchOS, tvOS, visionOS, macOS, and any platform where RN does not exist or where a WebView is forbidden.

The unified claim — "one component API, every platform" — is enforced mechanically by a **shared backend contract test suite** that every backend must pass.

---

## 2. Problem statement and prior art

### 2.1 The gap

| Project | Rendering | Platforms | State (verified 2026-11-11) | Gap |
|---|---|---|---|---|
| **Ionic + Capacitor** | WebView | iOS, Android, web | Mature, large ecosystem | Not real native views; DOM-dependent UI libraries only |
| **NativeScript + Angular** (`@nativescript/angular`) | Real native | iOS, Android (+visionOS) | Years in production, active (v18/v20 Angular lines) | XML/TS hybrid templates, zone-era DX, single host family |
| **ng-native** (`ng-native/ng-native`, [ng-native.com](https://ng-native.com/)) | Real native via **React Native Fabric** | iOS, Android | **Alpha**, 622★, 26 forks, 1,038 commits, v0.6.0 (2026-10-07), last push 2026-11-08, MIT | **Phones only.** No watch/TV/desktop path |
| **Angular-Native** (`Angular-Native/angular-native`) | Real native, **Rust core**, embedded JS engine, zoneless, AOT-only | 8 targets (iOS, Android, macOS, tvOS, visionOS, watchOS, Wear OS) | **2★**, last push 2026-09-08, MIT; Windows + HarmonyOS not started | Effectively a solo project; no Angular-first DX, no ecosystem, no plugin story |
| **nstudio/ng-native** | Fabric (experiment) | iOS, Android | 0★, last push 2026-10-16 | Experiment, not a product |
| **Angular Three** | Custom `Renderer2` → THREE.js | Web | v4 active | Proves the Angular custom-renderer SPI works in production — but 3D, not native UI |

**Nobody today delivers: one Angular codebase → real native views on phones *and* watch/TV/desktop with one component API.** That is NativeAngular's position.

### 2.2 What is officially supported by Angular (verified)

- `Renderer2` and `RendererFactory2` are **public, documented API**. The `Renderer2` docs state custom rendering exists to "render to something other than DOM" — this is a sanctioned extension point, not a hack. ([angular.dev/api/core/Renderer2](https://angular.dev/api/core/Renderer2), [angular.dev/api/core/RendererFactory2](https://angular.dev/api/core/RendererFactory2))
- `ViewEncapsulation` (`Emulated` / `Native` / `ShadowDom` / `None`) and `RendererType2` exist specifically so non-DOM renderers can scope styles.
- `@angular/platform-browser` **must not** be used in a native target — it pulls in `DomAdapter`, `DomRendererFactory2` and the HTML sanitizer, all of which assume a DOM. NativeAngular ships its own minimal platform provider instead.
- Angular is **zoneless by default** from v21 (Ionic 9 confirmed this), and Signal Forms exist — which makes a renderer-only integration dramatically more tractable than it was in the zone.js era. No timer/XHR patching is required inside the JS engine.
- Angular **AOT** is mandatory for native targets: `ngc` compiles templates at build time, the Angular Linker resolves partially-compiled libraries, and `@angular/compiler` never ships to the device.

### 2.3 Licensing reality

Both prior-art projects are **MIT**, so their code may be reused with attribution. Upstream contributions are the preferred route over forking. NativeAngular therefore:

- Reuses **React Native's Fabric** (Apache-2.0) through the Expo runtime — no RN code copied.
- Credits the Rust-core project's architecture (MIT) and may port code with attribution.
- Contributes its contract-test suite and capability model **back upstream** rather than forking.

---

## 3. Goals, non-goals, success criteria

### Goals

1. **G1 — Real native views.** Angular templates compile to platform-native view hierarchies (`UIView`, `android.view.View`, `WKInterfaceObject`, …). No WebView in the default path.
2. **G2 — One codebase, many platforms.** A single Angular app targets phones, watch and TV without per-platform forks.
3. **G3 — Capability-aware degradation.** Unsupported features resolve to a documented fallback, never a crash or a silent no-op.
4. **G4 — First-class DX.** `create` → run → hot reload → build → ship, with a testing library that runs components in Node without a simulator.
5. **G5 — OSS credibility.** MIT, RFC-driven, reproducible CI, documented limitations, and measurable adoption (stars, dependent apps, plugin count).
6. **G6 — Smallest installer.** A NativeAngular app must not carry a cross-platform native runtime. The only cross-platform payload is the JS bundle; the UI is the platform's own toolkit and the JS engine is the OS's own where one exists.
7. **G7 — Fastest development.** A component edit must be visible on the device in well under a second, and a component must be testable in Node without a simulator, device or native toolchain.
8. **G8 — Production-grade enterprise path.** Offline-first, security, biometrics, location, notifications and observability must be usable in a production app without hand-rolling them, and the core must stay MIT and uncrippled so a paid support offering can sit on top of it.

**Primary audience, in order.** (1) **Angular enterprise teams** that already ship Angular + .NET products and need Android/iOS delivery — they are the fastest adopters because their component and service code already exists. (2) **Angular OSS developers** — component authors, library and plugin maintainers, who supply the ecosystem others depend on. (3) **React Native developers** — only once reusable Rust services or genuine interop create value for them; making them the launch audience would be a strategic mistake, because they already have a mature framework and need a compelling reason to switch.

### Non-goals (v0.1)

- Not a web framework; not an SSR replacement.
- Not a replacement for Ionic — Ionic's webview model is a valid, popular choice and stays supported in the ecosystem.
- Not a full design system in v0.1 (a component library lands in v0.4).
- Not a JS engine project (we embed an existing engine; we do not write one).
- Not a full CSS implementation — a documented, deliberately small subset (see §7.4).

### Success criteria for v0.1

> The size and speed criteria are budgets, not aspirations — §11.1 records the
> measured baselines they are derived from, and CI enforces them.

- A sample app (login → list → detail → form) runs on iOS simulator and Android emulator from `npm create nativeangular-app`.
- 100% of the shared contract suite passes against the Fabric backend.
- A component test suite runs in Node in < 5 s with no simulator.
- Cold start to first frame < 1.5 s on a mid-range Android device.
- The style subset covers flexbox layout, text, color, borders, opacity, and transforms — enough for a production list/form app.
- **JS payload ≤ 60 KB gzip for a hello-world build**, ≤ 250 KB gzip for the sample app; `@angular/compiler` and `zone.js` are 0 bytes in every build (G6).
- **NativeAngular's own native code ≤ 500 KB per platform**; no cross-platform native runtime is shipped (G6).
- **Component edit → visible on device ≤ 500 ms**; a Node-only component test run ≤ 300 ms (G7).

---

## 4. Design principles

1. **The seam is public API.** Everything plugs into Angular's documented extension points. No private Angular internals (`ɵ`) unless unavoidable and isolated behind one adapter.
2. **Backend-agnostic core.** The core never imports a backend. Backends never import core internals — only the SPI types.
3. **Fail loudly at build time, degrade gracefully at runtime.** A typo'd property fails the Angular compiler, not the device. A missing capability degrades with a documented fallback and a dev-mode diagnostic.
4. **AOT-only, zoneless, no `platform-browser`.**
5. **Every platform-specific decision is data, not code.** Capabilities are declared data; the resolver decides. No `if (isWatch)` scattered through app code.
6. **The contract suite is the product guarantee.** "One component API" is not a slogan; it is a test suite every backend must pass.

---

## 5. Architecture

```
┌───────────────────────────────────────────────────────────────┐
│  Application                                                  │
│  Angular components · signals · DI · @angular/router · Signal Forms│
│  <an-view> <an-text> <an-list> <an-text-input> <an-pressable> …    │
└───────────────────────────┬───────────────────────────────────┘
                            │  Renderer2 / RendererFactory2 SPI
                            │  (@nativeangular/core)
┌───────────────────────────▼───────────────────────────────────┐
│  NativeAngular Core (backend-agnostic)                        │
│  ① ElementRegistry    — abstract element kinds → backend impls│
│  ② CapabilityRegistry — platform capabilities + resolver      │
│  ③ StyleContract      — CSS subset → backend style ops        │
│  ④ ModuleBridge       — typed native-module/plugin API        │
│  ⑤ PlatformProvider   — non-DOM Angular platform              │
└────────┬──────────────────────────────┬───────────────────────┘
         │ Fabric backend                │ Rust core backend (v0.2+)
         │ RN Fabric + Expo modules      │ Rust UI tree + layout
         │ iOS / Android                 │ watchOS / tvOS / visionOS / macOS
```

### 5.1 Layer contract

| Layer | Knows about | Must not know about |
|---|---|---|
| Application | `@nativeangular/components`, `@nativeangular/router` | Backends, native APIs |
| `@nativeangular/core` | Angular, the SPI | Any backend, any platform API |
| Backend | The SPI, Angular | Core internals beyond SPI types |
| Native layer | Platform APIs | Angular, TypeScript |

### 5.2 The Backend SPI

```ts
// @nativeangular/core
export type ElementKind =
  | 'view' | 'text' | 'image' | 'scrollView' | 'list' | 'textInput'
  | 'pressable' | 'switch' | 'activityIndicator' | 'slot';

export interface NativeElementHandle { readonly id: string; readonly kind: ElementKind; }

/** One element implementation for one backend. */
export interface NativeElementFactory {
  readonly kind: ElementKind;
  /** Native capability level this implementation satisfies. */
  readonly capability: CapabilityDescriptor;
  create(ctx: ElementContext): NativeElementHandle;
  setProp(el: NativeElementHandle, name: string, value: unknown): void;
  setStyle(el: NativeElementHandle, style: NativeStyle): void;
  append(el: NativeElementHandle, child: NativeElementHandle): void;
  remove(el: NativeElementHandle): void;
  insertBefore(el: NativeElementHandle, child: NativeElementHandle, ref: NativeElementHandle): void;
  dispose(el: NativeElementHandle): void;
  measure?(el: NativeElementHandle, constraint: MeasureConstraint): MeasureResult;
}

/** A backend = a factory set + a platform capability declaration. */
export interface NativeBackend {
  readonly id: string;                      // 'fabric' | 'rust'
  readonly platforms: readonly PlatformId[];
  readonly capabilities: PlatformCapabilities;
  readonly elements: readonly NativeElementFactory[];
  createRendererFactory(): RendererFactory2;
}
```

**Supporting types (normative for implementers).**

| Type | Meaning |
|---|---|
| `PlatformId` | `'ios' \| 'android' \| 'watchos' \| 'tvos' \| 'visionos' \| 'macos' \| 'wearos'` |
| `ElementContext` | Backend services available while creating an element (module bridge, injector, platform id) |
| `NativeStyle` | Backend-neutral style payload produced by the style compiler |
| `MeasureConstraint` | Available width/height handed to a measuring element |
| `MeasureResult` | Measured size plus its source (`intrinsic` / `constraint`) |
| `CapabilityDescriptor` | The feature + level an element implementation satisfies |

### 5.3 The measured Angular renderer contract

The M0 spike (2026-11-11) executed a real Angular 22.2.5 application in Node with
no DOM, against a custom `Renderer2`. The contract Angular actually relies on is
**not** the one the DOM suggests — these behaviours were measured, not assumed:

| # | What Angular does | What a native renderer must support |
|---|---|---|
| F4 | Calls `renderer.createRenderer(null, cmpDef)` | Tolerate a null host; the type argument may be a component definition (a null type must be tolerated too) |
| F5 | Calls `renderer.removeChild(null, child, isHostElement, requireSync)` | The parent argument is **null** — resolve the parent from the node itself |
| F6 | Clears a container with `element.textContent = ''` | Clearing must remove the children |
| F7 | Writes classes with `setAttribute('class', value)` | Keep the class list as an attribute; derive the list from it |
| F8 | Writes text with `setValue(textNode, value)` onto `nodeValue` | Implement the DOM `nodeValue` property on text nodes |
| F9 | Reads `parentNode` and `nextSibling`; calls `renderer.destroy()` | Track the parent, compute the next sibling, implement `destroy()` |

**The silent failure that shaped the design.** Signal inputs are **AOT-only**: in
JIT, `[label]="expr"` on a component whose input is declared with `input()` is not
recognised as a directive input — Angular falls through to
`renderer.setProperty(hostElement, 'label', value)`. On a web page that writes a
DOM property; on a native host it writes **nothing**, and the component renders an
empty value with no error anywhere. AOT emits `inputs: { label: [1, "label"] }`
(`InputFlags.SignalBased`), which is why the test harness compiles with `ngc`.

**Design rule adopted:** a native renderer must treat an unknown property write as
an **error**, not a no-op. The contract suite in M1 and the fake backend in M0
enforce this, so the failure mode can never reach a device.

**Platform recipe (verified).** Platform providers: `PLATFORM_ID`, `DOCUMENT`,
`RendererFactory2`, `CSP_NONCE` (`null`), `IMAGE_CONFIG` (warnings disabled).
Application providers: `ɵINJECTOR_SCOPE` (`'root'`), `ErrorHandler`, `APP_ID` —
the last two **must** be application providers; in the platform injector the
internal error handler resolves circularly. `ɵsetDocument(shim)` is required
because core resolves the root element through a module-level document reference,
not through the `DOCUMENT` injection token.

All Angular private (`ɵ`) API is confined to **one file**,
`packages/platform/src/angular-internals.ts`, which documents every import — the
same isolation NativeScript's Angular integration uses.

### 5.4 The Capability Registry — the differentiator

```ts
export type CapabilityLevel = 'full' | 'partial' | 'none';

export interface PlatformCapabilities {
  readonly platform: PlatformId;
  readonly features: Readonly<Record<string, CapabilityLevel>>;
}

export interface CapabilityRequest {
  readonly feature: string;                 // e.g. 'textInput.secure'
  readonly requires?: CapabilityLevel;      // default 'full'
  readonly fallback?: ElementKind;          // what to use if unsupported
}

export interface CapabilityResolution {
  readonly feature: string;
  readonly level: CapabilityLevel;
  readonly implementation: ElementKind | 'unavailable';
  readonly degraded: boolean;
  readonly reason?: string;
}
```

**Resolution algorithm**

1. Look up the active backend's `capabilities.features[feature]`.
2. If the level satisfies `requires` → use the primary implementation.
3. Else, if the component declares a `fallback` kind → use it, mark `degraded: true`.
4. Else → render nothing for that element, and emit a **runtime diagnostic** (dev-mode overlay + `console.warn` in prod) naming the feature, the platform, and the fix.

The registry is **data**, so a platform's capability set can change without touching the resolver. It is also the hook a plugin system uses: a plugin declares the features it requires and the platform it supports, and the registry rejects incompatible installs at build time.

**Phasing.** The `CapabilityRegistry` **interface** and the per-platform capability *declarations* ship in M1 (data only — resolution is the identity function). The **resolver**, fallback chain and degradation diagnostics ship in M2. M1 therefore stays honest about scope while guaranteeing the SPI is designed for capabilities from the first commit.

### 5.5 Render and commit model

- The renderer is **incremental**: Angular's change detection produces a diff; only changed props/styles/structure are sent to the native side.
- The native side keeps a **retained tree** of native views (mirroring Fabric's model) so a commit is a set of mutations, not a rebuild.
- **View flattening:** a component whose host has no style, prop or listener is not given a native view; its children attach to the grandparent. This keeps the native tree shallow and matches Fabric semantics.
- **Recycling:** `<an-list>` mounts a fixed number of slots and mutates their content while scrolling — it does not create/destroy views per row.

### 5.6 Style contract

A deliberately small CSS subset, compiled by the Angular build into a style payload the backend maps to native properties:

| Group | Supported in v0.1 |
|---|---|
| Layout | `display:flex`, `flex-direction`, `flex`, `flex-grow/shrink/basis`, `gap`, `justify-content`, `align-items`, `align-self`, `flex-wrap` |
| Sizing | `width`, `height`, `min/max-width/height`, `aspect-ratio` |
| Spacing | `padding*`, `margin*` (incl. logical `*-inline`/`*-block`) |
| Visual | `background-color`, `color`, `opacity`, `border-radius`, `border-width/color`, `box-shadow` (mapped per platform) |
| Text | `font-size`, `font-weight`, `font-family`, `line-height`, `text-align`, `letter-spacing`, `text-overflow` |
| Transform | `translate`, `scale`, `rotate` |

Everything else is a **build-time warning**, not a silent failure. Tailwind support is delivered as a build transformer that emits this subset (same approach upstream `ng-native` uses).

### 5.7 Native module & plugin bridge

```ts
export interface NativeModule<T> {
  readonly name: string;
  readonly requires?: readonly string[];        // capability features
  readonly platforms?: readonly PlatformId[];
  readonly create(context: ModuleContext): T;
}
```

- Modules are **promise-based**, typed, and injected as Angular services.
- In v0.1, Expo modules are bridged as Angular services (camera, location, notifications, secure storage, filesystem, SQLite, biometrics, maps, haptics) — this is how the RN ecosystem is inherited rather than reimplemented.
- Modules declare required capabilities; the registry refuses to install a module on a platform that cannot satisfy them.

**Inheriting the RN ecosystem is the sharpest differentiator, and the hardest promise.**
An Angular developer who can consume existing Expo/RN modules does not re-implement
camera, location, biometrics or storage. But "minimal adapters" understates the
problem: a React Native **TurboModule** is a JSI host object created by the RN
runtime, and a Fabric component is registered with the Fabric component registry —
neither is a plain API you can call. Consuming them from a different JS runtime is
a genuine integration project, not a wrapper.

So the v0.1 promise is bounded and measured rather than assumed:

- **Acceptance criterion:** the **top-20 Expo modules** are consumable from Angular —
  each either behind an adapter of ≤ 30 lines, or on a published compatibility matrix
  with a written reason it cannot be. "Not yet" is allowed; "silently absent" is not.
- **The bridge is a spike before it is a product.** M1 measures the real cost on three
  representative modules — one simple (`expo-secure-store`), one TurboModule-backed
  (`expo-camera`/`expo-location`), and one Fabric component (`react-native-svg`) —
  before the architecture commits to a bridge shape.
- **Fallback:** a first-party plugin ABI (`NativeModule<T>` above) that works with no
  RN runtime present. If the spike shows the RN bridge is impractical on a platform,
  the ABI is the path and the RN ecosystem is inherited module by module, not en masse.

### 5.8 Router integration

`@nativeangular/router` bridges `@angular/router`: routes map to native navigation (push/pop/present modally), hardware back button and Apple’s swipe-back are wired to router navigation, deep links resolve to routes, and guards/resolvers/lazy routes work unchanged.

### 5.9 Rust core backend (v0.2+)

| Concern | Choice | Rationale |
|---|---|---|
| Layout | [`taffy`](https://crates.io/crates/taffy) v0.15+ | Mature Rust implementation of CSS Block/Flexbox/Grid; used by Servo, Blitz, Zed; has a `parley` text-measure example |
| Text | `parley` + `fontdb` | Rust-native shaping/line-breaking; `taffy` documents the integration |
| JS engine | **QuickJS-ng via `rquickjs`**, Boa optional behind one trait | Interpreter-only (see below); QuickJS-ng is ~1 MB, runs on consoles (Minecraft Bedrock ships it); Boa is pure Rust with better test262 conformance but ~3–4× slower |
| Host crates | `nativeangular-host-{ios,android,watchos,tvos,visionos,macos}` | Thin platform bindings only |
| Bridge | `nativeangular-ffi` (C ABI) + `nativeangular-js` (engine-neutral trait) | The TS side never knows which engine is embedded |

**Rust's job is services, not just rendering.** A Rust core used *only* as a renderer
is a limited use of a heavy dependency. The strategic shape is a **shared native
services core** that is optional, platform-neutral, and — critically — **usable
without Angular**: the capability resolver, offline-sync engine, cryptography,
serialization, performance diagnostics and native plugin contracts are the same
libraries whether the client is an Angular app, a React Native app or a plain
native host. That is what turns Rust from an implementation detail into an
ecosystem asset, and it is why no Rust bridge is placed *under* the Angular API
where it would only serve Angular users.

| Rust responsibility | Serves | Needed by |
|---|---|---|
| Layout (taffy) + text (parley) | The render tree | M3 backends (watch/tvOS) |
| Capability resolver | Which features a platform can honour | M2, all backends |
| Offline-sync engine | Durable, conflict-aware local queue | Enterprise apps |
| Cryptography + secure storage | Key material that never reaches JS | Enterprise apps |
| Native plugin contracts | The wire format between JS and native modules | M1+ |
| Performance diagnostics | Frame, memory and battery telemetry | Devtools (v0.2) |

**Hard constraint:** iOS forbids JIT for third-party engines, and consoles forbid writable executable memory. Consequences:

- The embedded engine must be an **interpreter** (QuickJS-ng / Boa) — no V8, no `deno_core`.
- Angular code must be **AOT-compiled**; runtime JIT is unavailable and unnecessary.
- The app binary must not contain a JIT-capable engine, which is why the Rust backend targets watchOS/tvOS/visionOS where a WebView is also unavailable.

---

## 6. Repository and package layout

```
nativeangular/
├─ packages/                      # pnpm workspace
│  ├─ core/                       # @nativeangular/core        — SPI, registries, platform provider
│  ├─ components/                 # @nativeangular/components   — an-* elements, directives
│  ├─ fabric/                     # @nativeangular/fabric       — Fabric backend
│  ├─ router/                     # @nativeangular/router
│  ├─ native/                     # @nativeangular/native       — module/plugin API
│  ├─ platform/                   # @nativeangular/platform     — non-DOM Angular platform
│  ├─ testing/                    # @nativeangular/testing      — Testing Library + fake backend
│  ├─ cli/                        # @nativeangular/cli          — create/schematics/dev server
│  ├─ bundler-metro/              # @nativeangular/bundler-metro
│  ├─ bundler-embedded/           # @nativeangular/bundler-embedded   (v0.2+)
│  └─ rust/                       # @nativeangular/rust         — FFI to the Rust core (v0.2+)
├─ crates/                        # Cargo workspace (v0.2+)
│  ├─ nativeangular-core/         # UI tree, MountOp model, style resolution
│  ├─ nativeangular-layout/       # taffy integration + measure
│  ├─ nativeangular-text/         # parley + fontdb
│  ├─ nativeangular-js/           # engine-neutral JS bridge
│  ├─ nativeangular-ffi/          # C ABI
│  └─ nativeangular-host-*/       # ios, android, watchos, tvos, visionos, macos
├─ e2e/                           # Maestro flows
├─ examples/                      # sample apps
├─ docs/                          # docs site
└─ rfcs/                          # design proposals
```

**Workspace tooling:** pnpm workspaces (npm) + Cargo workspace (Rust), orchestrated by a small Node script. Nx is deliberately **not** required — the repo is cross-language, and Nx's value is mostly TS-graph tooling that pnpm workspaces + `changesets` + CI affordances already cover.

**Release model:** `changesets` for npm packages, `cargo-release` for crates, with a version-alignment check in CI (the Rust crates and the npm packages that bind them must ship the same version, as the Rust-core project does).

---

## 7. Data flow

### 7.1 Bootstrap

```
1. Native app launches → native host creates the JS runtime (Hermes via Expo, or the embedded engine)
2. Bundle loads (Metro bundle for v0.1; AOT output, no @angular/compiler)
3. Angular platform provider (not @angular/platform-browser) installs:
   - RendererFactory2  → the active backend's renderer
   - Zone-less change detection (signals)
   - Document/Element shims limited to what the renderer needs
4. Angular bootstraps the root component → first render
5. The backend commits the initial tree to native views
```

### 7.2 A property update

```
signal changes
  → Angular marks the view dirty (zoneless)
  → renderer.setProperty(el, 'value', v)
  → backend translates to a native op (Fabric mutation / MountOp)
  → native commit (next frame)
```

### 7.3 Capability resolution at runtime

```
component instantiated
  → registry.resolve({ feature: 'textInput.secure', requires: 'full', fallback: 'textInput' })
  → backend.capabilities → 'partial' on watchOS
  → fallback implementation chosen, degraded = true
  → diagnostic emitted to the dev overlay and the diagnostics channel
```

### 7.4 Cross-backend invariants (enforced by contract tests)

- Identical element kinds exist on every backend (or have a declared fallback).
- The same component renders on every backend with the same semantics.
- The same style subset maps to visually equivalent output (verified by screenshot golden tests per platform).

---

## 8. Element set (v0.1)

| Element | Purpose | Notes |
|---|---|---|
| `<an-view>` | Generic container | Flexbox container by default |
| `<an-text>` | Text | Supports nested spans via projected content |
| `<an-image>` | Image | Remote + local asset, `resize-mode` |
| `<an-scroll-view>` | Scrollable container | Vertical/horizontal |
| `<an-list>` | Virtualized list | Recycled slots; sections optional by capability |
| `<an-text-input>` | Text entry | `type`, `secure`, `multiline` (capability-gated) |
| `<an-pressable>` | Touch target | Press/opacity feedback, `onPress` |
| `<an-switch>` | Toggle | |
| `<an-activity-indicator>` | Spinner | |
| `<an-slot>` | Content projection | Named slots for component children |

All element names use the `an-` prefix so Angular does not treat them as HTML elements, and they remain self-closing. Properties are **typed inputs**, not arbitrary attributes — a typo is a compile error, not a silent device failure.

---

## 9. Error handling and diagnostics

| Code | Situation | Behaviour |
|---|---|---|
| `NATIVE_NO_BACKEND` | No backend registered for the current platform | Fail fast at startup with remediation text (install the right backend package) |
| `NATIVE_CAPABILITY_UNAVAILABLE` | Feature unsupported and no fallback declared | Element renders nothing; diagnostic emitted; build-time warning if statically detectable |
| `NATIVE_CAPABILITY_DEGRADED` | Fallback used | Non-fatal; dev overlay shows what was degraded and why |
| `NATIVE_MODULE_UNAVAILABLE` | Native module missing on this platform | Typed rejection with an install hint; injectable error boundary |
| `NATIVE_STYLE_UNSUPPORTED` | Property outside the CSS subset | Build-time warning with the property name; ignored at runtime |
| `NATIVE_COMMIT_FAILED` | Native commit rejected | Last-known-good tree retained; error surfaced through `ErrorHandler` |

Diagnostics flow through a single `DiagnosticsChannel` so tooling (dev overlay, CLI, CI reporter) consumes one stream instead of `console.log` sprinkling.

---

## 10. Testing strategy

1. **Backend contract suite** (`@nativeangular/testing/contract`) — a shared spec (Vitest) that every backend must pass: create/append/remove/prop/style semantics, view flattening, recycling, capability degradation, error cases. This suite is the mechanical guarantee behind G2/G3.
2. **Fake backend** — a full in-memory implementation of the SPI running in Node, powering `@nativeangular/testing` (Testing Library style: `render()`, `fireEvent()`, `getByText()`), no simulator, < 5 s suites.
3. **Rust unit tests** — layout engine tested against a `NaiveMeasurer` with no platform under it; text measurement cached by `(text, font, available width)`.
4. **Golden screenshot tests** — per platform, per element, style-subset equivalence across backends.
5. **CI matrix** — Linux (Rust tests, Node tests, Android build), macOS (iOS build + simulator run), Windows (Rust cross-compile check only).
6. **E2E** — Maestro flows (engine-agnostic, unlike Detox which targets RN) over a sample app on simulator + emulator.
7. **Mutation discipline** — every gate we ship is mutation-tested: inject the original defect, confirm the gate fails with the defect's exact numbers.

---

## 11. Tooling and DX (v0.1)

- `npm create nativeangular-app` → ready-to-run app on the Fabric backend.
- `nativeangular dev` → dev server with fast refresh (component edits keep state; framework/package edits restart).
- `nativeangular build ios|android` → native builds; EAS-compatible for distribution.
- `nativeangular doctor` → environment check (SDKs, Node, Rust, native toolchains).
- **Devtools (v0.2):** inspect the native view tree and the signal graph from the desktop (an approach the Rust-core project has already proven).
- **Tailwind support** via a build transformer emitting the supported style subset.
- **VS Code extension (v0.3):** element autocomplete, capability-aware IntelliSense (shows which capabilities the current platform supports), inline diagnostics.

### 11.1 Delivery budgets — installer size and development loop

Both budgets are **measured against this repository**, not estimated. The
baselines below were taken on 2026-11-12 from the M0 fixture
(`packages/testing/fixtures/out/app.js` — a two-component AOT app) with the
repository's own toolchain; the commands are listed in Appendix A.

**Measured baselines**

| Metric | Measured | Note |
|---|---|---|
| JS payload, minified ESM (dev mode) | 363,901 B raw / 120,884 B gzip | esbuild bundle of the AOT fixture |
| JS payload, minified ESM (production) | 273,878 B raw / 90,438 B gzip | `--define:ngDevMode=false` → dead-code elimination, **−25%** |
| `@angular/core` share of that payload | 349,758 B (**96.3%**) | esbuild metafile — the framework *is* the payload |
| `rxjs` share | 10,995 B (3.0%) | tree-shaken down to the interop surface |
| `@angular/compiler` (what we never ship) | 1,222,744 B raw / 227,530 B gzip | the JIT compiler — banned on device by the AOT-only rule |
| AOT compile (`ngc`) of a two-component app | **5.05 s** | `pnpm run compile:fixtures` |
| Bundling that same AOT output | **0.24 s** | esbuild — 21× faster than the compile |
| Node-only component suite (75 tests) | 5.22 s wall, **251 ms** of test time | the rest is the AOT compile above |

**Installer-size budget (G6)**

| Item | Budget | Why |
|---|---|---|
| JS payload, production, gzip | ≤ **60 KB** for a hello-world; ≤ 250 KB for a typical app | the esbuild-only pipeline measures 90 KB, so the production bundler must add terser-class DCE — tracked as an M1 task |
| NativeAngular's own native code, per platform | ≤ **500 KB** | the platform toolkit and the JS engine are not ours to shrink |
| JS engine | iOS/tvOS/watchOS: **0 KB** (JavaScriptCore is in the OS image); Android: **~2.5 MB** per ABI (Hermes) | the engine is the one unavoidable per-platform cost, and only on Android |
| `@angular/compiler`, `zone.js` | **0 B, always** | enforced by `scripts/constraints.spec.ts`, not by convention |
| Rust core (M3, if built) | ≤ **1 MB** per platform, measured | see D8 — it buys cross-platform layout parity at a byte cost |

**Why this is smaller than React Native.** React Native ships a cross-platform
native runtime *plus* an engine: a fresh RN app carries ~15–20 MB of native code
on Android (arm64) and ~25–35 MB on iOS after thinning, against a ~8–15 MB pure
native floor. NativeAngular ships neither: the UI is `UIKit`/`android.view.View`
and the engine is the OS's own. The measured JS payload is the entire
cross-platform cost — and 96% of that is `@angular/core`, which every Angular
app pays whether or not it is native.

**Development-loop budget (G7)**

| Step | Budget | Baseline today |
|---|---|---|
| Component edit → compiled instructions | ≤ **200 ms** per changed component | `ngc` re-compiles the whole program in 5.05 s |
| Compiled instructions → visible on device | ≤ **300 ms** | not implemented — the device tooling is M1 |
| **Inner loop, end to end** | ≤ **500 ms** | — |
| Node-only component test (fake backend) | ≤ **300 ms** | 251 ms measured ✓ |
| Full app rebuild | ≤ **2 s** | 0.24 s of bundling after a 5.05 s compile |

**The decision that makes both budgets reachable:** *the device never runs a
compiler — not even in development.* The dev server compiles templates in Node
and ships the resulting instructions, so the device bundle stays compiler-free in
dev exactly as it is in production. That turns the AOT-only rule from a tax
into the thing that makes the loop fast: the compiler is on your laptop, where
it costs 200 ms instead of the seconds a JIT compile would cost on a phone, and
the artefact the device runs is the same artefact CI ships.

The corollary is that the slow step must leave the critical path. `ngc` at 5 s
for two components does not scale to a real app (30–60 s), so the pipeline has
to become incremental: compile per changed component, bundle with esbuild, ship
the delta. The fake backend already gives the fastest loop the project has —
75 component-level assertions in 251 ms with no device, no simulator and no
native toolchain — which is why M1's contract suite is worth more than a device
emulator.

### 11.2 Performance acceptance gates

Rust is not a marketing headline until it has been measured. Every number below is
a **target, not a measurement** — no device measurement exists yet, and the first
device run must fill this table in rather than leave it aspirational.

| Metric | Gate | Reference point |
|---|---|---|
| Cold start → first frame | ≤ 1.5 s on a mid-range Android device | RN baseline on the same device and app |
| 1,000-item list | 60 FPS sustained, no dropped-frame spikes | RN `FlatList` on the same device |
| Input latency | no perceptible lag; touch → visual response ≤ 1 frame | RN baseline |
| Memory | no major regression against RN for the same screen | RN baseline, same device |
| App package size | see the §11.1 budget table | measured per platform |
| Background location | OS-compliant, reliable within platform limits | platform policy |
| Offline sync | **no lost acknowledged operation** | contract test, not a benchmark |
| Crash-free sessions | no critical renderer/runtime crash | production telemetry |

Two of these are not performance numbers at all and are treated as invariants:
**no lost acknowledged operation** (offline sync) and **no critical renderer crash**
— a crash-free rate is a product property, not a speed property.

The comparison method is fixed in advance: same device, same screen, same data
set, RN and NativeAngular builds from the same team, and the numbers published
whether or not they flatter us.

---

## 12. Roadmap

| Milestone | Scope | Exit criteria | Indicative effort* |
|---|---|---|---|
| **M0 — Spike & skeleton** | Repo, workspaces, CI, Angular platform provider, SPI, **fake backend** rendering real Angular components in Node | A component tree renders through the SPI in Node; `Renderer2` gap analysis written up | 3–5 weeks — **prototype already validated** (see §5.3); the plan is `docs/superpowers/plans/2026-11-11-nativeangular-m0-spike.md` |
| **M1 — Fabric backend MVP** | 10 elements, style subset, router bridge, Expo module bridge, CLI, template app, testing lib | Sample app runs on iOS + Android; contract suite green | 8–12 weeks |
| **M1.5 — RN interop spike** *(inside M1's budget)* | Measure the real cost of consuming three representative Expo/RN modules from a non-RN JS runtime; publish the compatibility matrix | A measured answer — bridge, adapter-per-module, or first-party ABI only — with the top-20 matrix started | 1–2 weeks |
| **M2 — Capability & plugins** | Capability registry + resolver, degradation diagnostics, plugin API + registry, dev overlay | A plugin declaring an unsupported feature is rejected at build time; degradation works end-to-end | 6–8 weeks |
| **M3 — Rust core backend** | Rust core: tree, layout (taffy), text (parley), JS bridge (QuickJS-ng), hosts for watchOS + tvOS (+ visionOS/macOS) | One Angular app runs on iPhone, Apple Watch and TV from one codebase | 16–24 weeks |
| **M4 — Ecosystem** | Component library + theming, file-based routing, plugin marketplace, VS Code extension, docs site | Third-party plugin published without core changes | 12+ weeks |

\* Assumes a small team (see §14 assumptions). Effort is indicative, not a commitment.

**Resource split while the Fabric backend is still unproven on a device:
60% Fabric + DX · 25% tests + docs · 15% Rust research.** Rust research is held at
the low end on purpose — until the Fabric backend runs on a real device, every
Rust hour is a bet on an unvalidated architecture. The split shifts towards Rust
only once the device run is green and the M1 contract suite has something to
measure.

**The next decisive milestone is not a feature — it is adoption.** M1 completed and
validated with external Angular developers tells us more than another month of
internal work. The readiness scorecard in `docs/architecture/readiness-scorecard.md`
is the instrument for that.

**M0 kill criteria (critical).** If Angular's `Renderer2` SPI cannot express the element contract without DOM APIs (i.e. the renderer reaches for `document`/`Node`), the fallback architecture is a **NativeScript-style DOM shim** (implement enough of the DOM for Angular's renderer to run, then map the shim tree to native views). This is more work but is proven to work — NativeScript has shipped it for a decade. The spike exists to answer this question before committing to the SPI approach.

---

## 13. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| `Renderer2` cannot express the contract without DOM | ~~High~~ **Resolved** | The M0 spike rendered an AOT Angular app through a custom renderer with no DOM. Contract measured in §5.3; DOM-shim fallback no longer needed |
| Upstream `ng-native` is alpha and may pivot | High | Our core is backend-agnostic; Fabric backend is our code; contribute upstream rather than fork |
| Rust core is a large surface with a small talent pool | High | Engine choice de-risked (taffy/parley/QuickJS-ng are proven crates); layout delegated to taffy |
| Capability system becomes over-engineered | Medium | Ship the registry in M2 only after the Fabric path works end-to-end; keep the resolver ~200 lines |
| Apple JIT ban and watchOS constraints | Medium | Interpreter-only engine + AOT-only Angular; no JIT dependency by construction |
| Trademark confusion with "Angular Native" / upstream | Medium | Distinct product name (NativeAngular), distinct npm scope, explicit non-affiliation notice, no `ng-native`/`@angular/*` identifiers |
| Scope creep across 8 platforms | High | v0.1 ships phones only; the 8-platform claim is a v0.2 milestone, not a launch promise |
| An unregistered input silently becomes a no-op on device | High | **Measured** (§5.3): unknown property writes must throw. Enforced by the fake backend from M0 and by the contract suite in M1 |
| Low adoption (the classic OSS graveyard) | High | Lead with the **migration story**: an existing Angular web team gets a native app without rewriting the app; publish an RN→Angular case study with real numbers |

---

## 14. Assumptions (to be confirmed; the spec is written against these)

1. **Team:** 2–4 engineers with Angular fluency; at least one with native iOS/Android experience; Rust work starts at M3 and may be contracted. If the team is one Angular engineer, M1 alone is the realistic ceiling and M3 is a multi-year proposition.
2. **Angular version:** target the current stable Angular (v22 at time of writing); support the two latest majors.
3. **Target platforms:** v0.1 = iOS + Android. watchOS/tvOS in v0.2. visionOS/macOS are stretch.
4. **Licensing:** MIT for code; the trademark policy keeps "Angular" nominative use only, with a clear non-affiliation statement.

---

## 15. Naming, branding, governance

**Name:** NativeAngular — npm `nativeangular`, scope `@nativeangular/*`, GitHub org `native-angular`, domain `nativeangular.dev`.

Availability verified 2026-11-11 via the npm registry (all `@nativeangular/*` names return 404), the GitHub users API (`native-angular` returns 404) and DNS (no A record for `nativeangular.dev`). **Re-verify at the registrar and on npm before publishing — DNS absence is not proof of registration.**

Rejected alternatives: `ng-native` and `@ng-native/*` (taken by the upstream project), `Angular Native` / `angular-native` (taken; also maximally confusing with two existing projects), `Angora` (GitHub user and `angora.dev` taken).

**Positioning line:** "One Angular codebase. Every native platform."

**Trademark notice (required in README and package metadata):** NativeAngular is an independent community project, not affiliated with or endorsed by Google LLC, the Angular team, Expo, Meta, or NativeScript. "Angular" is a trademark of Google LLC and is used here only to describe compatibility.

**Commercial model — the core stays intact.** The framework is MIT and deliberately
uncrippled: crippling core features to charge enterprise users would cost more
community adoption than it could win in revenue. Commercial value sits *on top* of
the OSS core, where it removes engineering risk rather than gating capability:
enterprise support SLAs, a managed build-and-release service, enterprise-grade
component suites, certified plugin integrations, security/compliance certification,
and migration consulting. Customers pay for reduced risk and faster delivery, not
for the framework's name.

**Governance:** MIT licence, RFC directory for design proposals, `changesets` + `cargo-release` for coordinated releases, CODEOWNERS, SECURITY.md, CODE_OF_CONDUCT modelled on the Contributor Covenant, and a public roadmap. Major design changes require an RFC; backwards-incompatible API changes require a migration codemod in the same release.

---

## 16. Deferred decisions (defaults are chosen so work is never blocked)

| # | Decision | Default for now | Revisit at |
|---|---|---|---|
| D1 | Vendor a subset of RN's Fabric C++ instead of depending on the Expo runtime | **Do not vendor.** Depend on the Expo runtime — EAS, updates and the module ecosystem are worth more than dependency purity | M2, if upstream `ng-native` stalls |
| D2 | First Rust-backend platform | **watchOS** — RN is impossible there and the value proposition is clearest | M3 kickoff |
| D3 | WebView escape hatch for embedding web widgets in a native screen | **Out of scope for v0.1.** The "no WebView" promise holds; an opt-in `@nativeangular/webview` package may be evaluated in M2 | M2 |
| D4 | Where the capability registry lives | **Inside `@nativeangular/core`** — it is small and central to the thesis; split into its own package only if it grows | M2 |
| D5 | Engine for the Rust backend | **QuickJS-ng via `rquickjs`** (interpreter, ~1 MB, console-proven); Boa available behind the same trait as a pure-Rust fallback | M3 |
| D6 | Layout engine for the Rust backend | **`taffy`** (flexbox + grid) with `parley`/`fontdb` for text measurement | M3 |
| D7 | Where templates are compiled | **In Node, never on the device** — the dev server compiles and ships instructions, so the device bundle is compiler-free in dev as well as in production. This is what makes the ≤ 500 ms inner loop reachable while keeping the AOT-only rule absolute | M1 |
| D8 | Whether to build the Rust core at all | **Decide on measured bytes, not on layout ambition.** The Rust core buys cross-platform layout parity but costs ~1 MB per platform; the platform's own layout engines cost 0 bytes and guarantee no parity. M3 must measure the static library (stripped, LTO) and compare it against the parity risk before it starts | M3 kickoff |
| D9 | Element-name prefix | **Recommend `na-`** (NativeAngular) over the current `an-`, and add `<na-button>` beside `<na-pressable>` — the prefix should match the brand, and `button` is the name developers reach for. The M0 fixture keeps `an-` as an internal test name; the public element set is not implemented yet, so renaming costs nothing now | M1, before the first element ships |
| D10 | Scope of the Rust core | **A shared native services core** — capability resolver, offline sync, crypto, serialization, diagnostics, plugin contracts — usable without Angular, with rendering as one consumer of it. Not a renderer with extra features bolted on | M3 kickoff |
| D11 | How the RN ecosystem is inherited | **Undecided until the M1.5 spike measures it.** The RN bridge is the highest-value promise and the highest-risk one; the first-party plugin ABI is the guaranteed fallback. The promise is bounded by the top-20 compatibility matrix either way | M1.5 |

---

## Appendix A — Verification log (2026-11-11)

| Claim | Source |
|---|---|
| `Renderer2`/`RendererFactory2` are public API for non-DOM rendering | [angular.dev/api/core/Renderer2](https://angular.dev/api/core/Renderer2), [RendererFactory2](https://angular.dev/api/core/RendererFactory2) |
| ng-native: 622★, 26 forks, 1,038 commits, v0.6.0 on 2026-10-07, pushed 2026-11-08, MIT | GitHub API `repos/ng-native/ng-native` |
| ng-native requires Angular 22, Expo SDK 57/58, RN 0.86/0.88, New Architecture, Node ≥ 22.18 | [github.com/ng-native/ng-native](https://github.com/ng-native/ng-native) |
| Angular-Native (Rust): 2★, pushed 2026-09-08, MIT, 8 targets, Windows/HarmonyOS not started | GitHub API `repos/Angular-Native/angular-native`, [github.com/Angular-Native](https://github.com/Angular-Native) |
| nstudio/ng-native: 0★, pushed 2026-10-16 | GitHub API `repos/nstudio/ng-native` |
| NativeScript Angular packages: renderer, router integration, dialogs, list-view templates, zone setup, bootstrap | [github.com/NativeScript/angular](https://github.com/NativeScript/angular) |
| Ionic 9 supports Angular 18–22 and Capacitor 7+; zoneless by default from Angular 21 | [ionicframework.com/docs/updating/9-0](https://ionicframework.com/docs/updating/9-0), [ionic.io/blog](https://ionic.io/blog/feed) |
| taffy v0.15.1 — Rust Block/Flexbox/Grid layout, has a `parley` measure example; used by Servo, Blitz, Zed | [crates.io/crates/taffy](https://crates.io/crates/taffy), [doc.servo.org/taffy](https://doc.servo.org/taffy/index.html) |
| iOS bans JIT for third-party engines; consoles forbid writable executable memory | [outl.app RFC 0025](https://outl.app/docs/rfcs/0025-plugin-system.html), [bevy-ts research](https://github.com/abyme-studio/bevy-ts/blob/main/docs/research/js-runtime.md) |
| QuickJS-ng via `rquickjs` (interpreter, ~1 MB, ships on consoles) vs Boa 0.22 (pure Rust, better conformance, slower) | [bevy-ts research](https://github.com/abyme-studio/bevy-ts/blob/main/docs/research/js-runtime.md), [southstar-browser rust-port](https://github.com/nordstjernen-web/southstar-browser/blob/main/docs/rust-port.md) |
| macOS JIT requires `com.apple.security.cs.allow-jit` entitlement; iOS apps cannot use it | [developer.apple.com entitlement docs](https://developer.apple.com/documentation/bundleresources/entitlements/com.apple.security.cs.allow-jit) |
| Angular Three v4 — custom `Renderer2` for THREE.js in production | [angularthree.org](https://angularthree.org/) |
| npm scope / GitHub / DNS availability for the name | npm registry API, GitHub users API, `getent hosts` (checked 2026-11-11) |
| `ɵinternalCreateApplication`, `ɵsetDocument`, `ɵINJECTOR_SCOPE` are exported by `@angular/core` 22.2.5 | `node_modules/@angular/core/fesm2022/core.mjs`, `_debug_node-chunk.mjs` (verified in the sandbox) |
| `removeChild(null, child, …)`, `textContent = ''`, `setAttribute('class', …)`, `nodeValue` writes, `parentNode`/`nextSibling` reads, `renderer.destroy()` | `@angular/core` 22.2.5 source, confirmed by a running renderer in the M0 spike |
| NativeScript's DOM-less platform recipe (`platformCore` + `createPlatformFactory` + `ɵinternalCreateApplication`, `DOCUMENT`/`Sanitizer`/`LocationStrategy` providers, `ɵINJECTOR_SCOPE: 'root'`) | `node_modules/@nativescript/angular@22.0.2/fesm2022/nativescript-angular.mjs` |
| Angular 22 requires TypeScript `>=6.0 <6.1` | `@angular/compiler-cli@22.2.5` peer dependency — an *optional* peer, so an incompatible TypeScript installs; the repository pins `6.0.3` and the toolchain check fails on a mismatch |
| Angular 22 engine floor `^22.22.3 || ^24.15.0 || >=26.0.0` | `@angular/core@22.2.5` `engines` field |
| JS payload of the AOT fixture: 363,901 B raw / 120,884 B gzip; 273,878 B / 90,438 B with `ngDevMode=false`; `@angular/core` = 96.3% of it | esbuild 0.28.2 bundle of `packages/testing/fixtures/out/app.js` + `--metafile` (measured 2026-11-12) |
| `@angular/compiler` = 1,222,744 B raw / 227,530 B gzip — the payload the AOT-only rule keeps off the device | gzip of `@angular/compiler@22.2.5/fesm2022/compiler.mjs` |
| AOT compile of a two-component app = 5.05 s; esbuild bundling of the same output = 0.24 s | `pnpm run compile:fixtures` vs `esbuild --bundle` (measured 2026-11-12) |
| 75 component-level tests in 251 ms of test time, without a device or simulator | `pnpm vitest run` (measured 2026-11-12) |
| React Native ships ~15–20 MB of native code on Android (arm64) and ~25–35 MB on iOS after thinning; a pure native app floors at ~8–15 MB; Hermes adds ~2.5 MB per ABI while JSC is already resident in the iOS system image | [72technologies — React Native app size audit](https://www.72technologies.com/blog/react-native-app-size-bloat-audit-2026), [techamit95ch — Hermes startup regression](https://dev.to/techamit95ch/startup-time-regression-after-enabling-hermes-in-a-react-native-app-2df0) |
