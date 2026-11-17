# 📋 AngNative System Requirements & Specifications

**Status:** Canonical Requirements (v1.0)  
**License:** MIT

---

## 1. Product Positioning & Goals

AngNative is positioned as **"the native runtime for Angular"**. It provides enterprise and open-source developers with a first-class route to build cross-platform mobile and desktop applications using Angular 19+ without WebViews.

### 1.1 Core Goals
- **G1 — Real Native Views**: Render real platform widgets (`android.view.View`, `UIView`, etc.). No WebViews in the default execution path.
- **G2 — Single Codebase, Multi-Platform**: One Angular component model targeting Android, iOS, and Desktop.
- **G3 — Capability-Aware Degradation**: Features unsupported on a given platform resolve gracefully to a documented fallback, never crashing or failing silently.
- **G4 — High-Velocity Developer Experience (DX)**: Instant HMR (< 1 second), standalone component testing in Node without an emulator, and seamless CLI tooling.
- **G5 — Ultra-Lightweight Binary Footprint**: Strict size budgeting for both the native binary payload and the compiled JavaScript bundle.
- **G6 — 60–120 FPS Fluidity**: Decoupled asynchronous layout ensures the main UI thread never drops frames during JavaScript execution.

### 1.2 Non-Goals
- Not a replacement for Angular Web / SSR.
- Not a replacement for Ionic's WebView model (Ionic remains an option for web-first apps).
- Not a full web browser or full CSS engine (supports a documented, clean styling subset).

---

## 2. Functional Requirements

### 2.1 Angular Framework Support
| Requirement | Specification |
| :--- | :--- |
| **Angular Version** | Angular 19.0+ |
| **Reactivity** | Angular Signals (`signal`, `computed`, `effect`) & Signal Forms |
| **Change Detection** | Zoneless by default (no `zone.js` patching required) |
| **Compilation** | Strict Ahead-of-Time (AOT) via `ngc` & Angular Linker |
| **Renderer SPI** | Custom `Renderer2` / `RendererFactory2` implementation |

### 2.2 Core Native UI Primitives (`na-*`)
All components use the canonical `na-` prefix:
- `<na-view>`: Multi-purpose container and layout box (flex/grid).
- `<na-text>`: Text rendering node with typography, color, and wrapping.
- `<na-button>`: Interactive button widget with press feedback and states.
- `<na-text-input>`: Native single/multi-line keyboard input field.
- `<na-scroll-view>`: High-performance scrollable surface (vertical & horizontal).
- `<na-image>`: Hardware-accelerated image viewer (PNG, JPEG, WebP, SVG).
- `<na-pressable>`: Low-level touch event responder.

### 2.3 Event System
The framework provides unified cross-platform event dispatching:
- `press` / `tap`: Primary user interaction.
- `change` / `input`: Form field value updates.
- `scroll`: Dynamic scroll position offsets.
- `focus` / `blur`: Focus lifecycle management.
- `layout`: Element geometry resolution notifications.

---

## 3. Non-Functional Requirements & Performance SLAs

### 3.1 Performance Metrics
| Metric | Threshold Target | Verification Status |
| :--- | :--- | :--- |
| **Cold Startup Time** | < 800 ms to interactive | Monitored |
| **Frame Rate** | Constant 60 FPS (120 FPS on ProMotion/High-Refresh) | Enforced |
| **Mutation Diffing** | < 1.0 ms for 1,000 node updates | Verified |
| **Touch-to-Response Latency** | < 16 ms | Verified on Android 17 |
| **Hot Module Replacement (HMR)** | < 1,000 ms from save to screen update | Enforced |

### 3.2 Size Budgets
- **Rust Static Library (`libangnative_host_ffi.a`)**: < 1.0 MB stripped + LTO per architecture target.
- **Android APK Overhead**: Native engine adds < 3.5 MB uncompressed to the application bundle. Total demo APK < 15 MB.
- **Memory Footprint**: Baseline runtime memory overhead < 25 MB on mobile platforms.

---

## 4. Platform Support Matrix

| Platform | Tier | Target Architecture | Host Runtime |
| :--- | :--- | :--- | :--- |
| **Android** | Tier 1 (Active) | `arm64-v8a`, `armeabi-v7a`, `x86_64` | Android 8.0+ (API 26 to 35+) |
| **iOS** | Tier 1 (Active) | `arm64`, `arm64e` | iOS 15.0+ |
| **macOS** | Tier 2 (Planned) | `arm64`, `x86_64` | macOS 12.0+ |
| **Windows** | Tier 2 (Planned) | `x64`, `arm64` | Windows 10 / 11 |
| **Linux** | Tier 2 (Planned) | `x86_64`, `aarch64` | Modern Wayland / X11 |
