# 🚀 AngNative

<p align="center">
  <img src="https://raw.githubusercontent.com/moidiqbal/angnative/master/docs/assets/banner.png" alt="AngNative Banner" width="100%" onerror="this.style.display='none'"/>
</p>

<p align="center">
  <b>The High-Performance Native Mobile & Desktop Framework for Angular.</b><br/>
  <i>True Native UI widgets. Zero WebViews. Powered by an ultra-fast Rust Host Engine & Taffy Layout.</i>
</p>

<p align="center">
  <a href="https://github.com/moidiqbal/angnative/actions"><img src="https://img.shields.io/badge/build-passing-brightgreen?style=for-the-badge&logo=github-actions" alt="Build"/></a>
  <a href="https://angular.dev"><img src="https://img.shields.io/badge/Angular-19+-DD0031?style=for-the-badge&logo=angular&logoColor=white" alt="Angular"/></a>
  <a href="https://www.rust-lang.org"><img src="https://img.shields.io/badge/Rust-1.80+-orange?style=for-the-badge&logo=rust&logoColor=white" alt="Rust"/></a>
  <a href="https://developer.android.com"><img src="https://img.shields.io/badge/Android-17%20Ready-34A853?style=for-the-badge&logo=android&logoColor=white" alt="Android"/></a>
  <a href="https://developer.apple.com/ios/"><img src="https://img.shields.io/badge/iOS-arm64-000000?style=for-the-badge&logo=apple&logoColor=white" alt="iOS"/></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue?style=for-the-badge" alt="License"/></a>
</p>

---

## ⚡ Why AngNative?

For over a decade, **Angular** has been the enterprise gold standard for web applications. Yet on mobile, developers were forced to compromise: either wrap apps in sluggish **WebViews** (Cordova, Capacitor, Ionic) or abandon TypeScript and Angular altogether for Flutter (Dart) or React Native.

**AngNative eliminates the compromise.**

AngNative executes your Angular components and reactive signals on an embedded JS runtime, synchronizes a high-throughput shadow tree inside an ultra-fast **Rust Host Engine**, computes Flexbox/Grid layouts via **Taffy**, and renders **100% native platform UI widgets** (`android.widget.*`, `UIKit.*`) via zero-copy C-FFI / JNI bridges.

```
+-------------------------------------------------------------+
|                      Angular 19+ App                        |
|        (Signals, Zoneless, Dependency Injection, DI)        |
+-------------------------------------------------------------+
                              |
                     @angnative/core
              (Custom Renderer2 & Shadow Tree)
                              |  Batch Mutation Buffer
                              v
+-------------------------------------------------------------+
|                      Rust Host Engine                       |
|   crates/angnative-host-core   |   crates/angnative-host-ffi|
|   crates/angnative-host-layout (Taffy 0.5.2 Flexbox/Grid)   |
|   crates/angnative-host-view   |   crates/angnative-services|
+-------------------------------------------------------------+
                              |
          +-------------------+-------------------+
          | Zero-Copy JNI                         | C-FFI / Swift
          v                                       v
+-----------------------+               +-----------------------+
|   Android 17 Shell    |               |       iOS Shell       |
|  (Native View Hierarchy,              |   (UIKit / Metal UI,  |
|   Material 3 Surface) |               |    Mach-O arm64)      |
+-----------------------+               +-----------------------+
```

---

## ✨ Features

- 🎯 **Pure Native Widgets (No WebViews)**: Buttons are real `android.widget.Button` / `UIButton`. Text is real `android.widget.TextView` / `UILabel`. 60–120 FPS buttery smooth scrolling.
- 🦀 **Rust Core Engine**: Ultra-low memory footprint, lightning-fast mutation batching, and strict memory safety.
- 📐 **Taffy 0.5.2 Layout**: High-performance CSS Flexbox & CSS Grid computation directly in Rust before touching the UI thread.
- ⚡ **Angular 19 Ready**: Full support for Angular Signals (`signal()`, `computed()`, `effect()`), modern control flow (`@if`, `@for`), and zoneless change detection.
- 🌉 **Zero-Copy FFI**: Fast mutation batch serialization using byte buffers over JNI on Android and direct C-ABI on iOS/Desktop.
- 📱 **Cross-Platform**: Android (Android 17 / API 35 ready), iOS (arm64 pipeline), Desktop (Win32/macOS/Linux), and Web fallback adapter.

---

## 🧩 Writing an AngNative Component

Developing with AngNative feels identical to standard modern Angular, using native semantic tags:

```typescript
import { Component, signal } from '@angular/core';
import { NaView, NaText, NaButton } from '@angnative/common';

@Component({
  selector: 'app-counter',
  standalone: true,
  imports: [NaView, NaText, NaButton],
  template: `
    <na-view style="flex-direction: column; padding: 24px; gap: 16px;">
      <na-text style="font-size: 28px; font-weight: bold; color: #1E293B;">
        AngNative Demo
      </na-text>
      
      <na-text style="font-size: 18px; color: #64748B;">
        Clicks: {{ count() }}
      </na-text>

      <na-button 
        (press)="increment()"
        style="background-color: #4F46E5; border-radius: 12px; padding: 14px 20px;">
        <na-text style="color: #FFFFFF; font-weight: 600;">
          Tap Me! 🚀
        </na-text>
      </na-button>
    </na-view>
  `
})
export class CounterComponent {
  count = signal(0);

  increment() {
    this.count.update(c => c + 1);
  }
}
```

---

## 📦 Workspace Architecture

```
angnative/
├── Cargo.toml                    # Rust workspace configuration
├── package.json                  # pnpm monorepo workspace
├── crates/                       # 🦀 Rust Core Crates
│   ├── angnative-host-core/      # Mutation protocol, shadow DOM & element registry
│   ├── angnative-host-engine/    # QuickJS runtime & embedded JS executor
│   ├── angnative-host-ffi/       # C-ABI and JNI bindings with exported C header
│   ├── angnative-host-layout/    # Taffy 0.5.2 Flexbox/Grid adapter & text measurer
│   ├── angnative-host-view/      # View factory, view tree, and op emitter
│   ├── angnative-services-core/  # Capabilities, crypto, diagnostics, and sync
│   └── angnative-desktop/        # Native desktop shell (Windows, macOS, Linux)
├── packages/                     # 🅰️ Angular & TypeScript Packages
│   ├── core/                     # Custom Angular Renderer2 & NativeElement runtime
│   ├── common/                   # Native primitives (<na-view>, <na-text>, <na-button>)
│   ├── contract/                 # Binary mutation protocol schema & event specs
│   ├── platform/                 # Native Angular platform bootstrap
│   ├── cli/                      # CLI tools (init, build, run, doctor, inspect)
│   └── testing/                  # Conformance test suites and mutation teeth
├── platforms/                    # 📱 Native Platform Shells
│   ├── android/                  # Android Studio project, JNI bridge, Kotlin UI runtime
│   └── ios/                      # iOS SwiftPM shell & Darwin SDK toolchain
└── apps/
    └── demo-app/                 # Standalone Angular Native application
```

---

## 🛠️ Quick Start

### Prerequisites
- [Node.js](https://nodejs.org/) v20+ and [pnpm](https://pnpm.io/) v9+
- [Rust](https://rustup.rs/) 1.80+ (`rustup target add aarch64-linux-android x86_64-linux-android`)
- Android Studio / SDK (API 34+) & NDK 26+

### 1. Clone & Install
```bash
git clone https://github.com/moidiqbal/angnative.git
cd angnative

# Install JS dependencies
pnpm install

# Build Rust crates
cargo build --workspace
```

### 2. Run Android Demo App
```bash
# Build Rust host FFI library for Android targets
./scripts/build-android-native.sh

# Open and run platforms/android in Android Studio, or via CLI:
cd platforms/android
./gradlew installDebug
```

---

## 🗺️ Roadmap & Platform Matrix

| Platform | Target Runtime | Layout Engine | Status |
| :--- | :--- | :--- | :--- |
| **Android** | Kotlin + Android View Hierarchy / Jetpack | Rust Taffy + JNI | ✅ Functional (Android 17 / API 35) |
| **iOS** | Swift + UIKit / Metal | Rust Taffy + C-FFI | 🟡 Toolchain & Mach-O arm64 Ready |
| **Desktop** | Win32 / AppKit / Wayland | Rust Taffy + Winit | 🟡 In Development |
| **Web** | Standard DOM / HTML5 Fallback | CSS Flexbox | 🟢 Planned |

---

## 🤝 Contributing

Contributions are warmly welcome! Whether you are an Angular expert, a Rustacean, or a mobile platform engineer:

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'feat: add some amazing feature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

---

## 📄 License

Distributed under the **MIT License**. See `LICENSE` for more information.

---

<p align="center">
  Built with ❤️ by <a href="https://github.com/moidiqbal">Moid Iqbal</a> and the open-source community.
</p>
