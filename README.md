# AngNative

> **AngNative** is a high-performance cross-platform application framework bringing the power of **Angular** to Native Android, Desktop (Windows, macOS, Linux), and Web — powered by an ultra-fast **Rust** core engine, embedded TypeScript compiler, and Taffy layout.

---

## 🚀 Key Highlights

- **Angular Component Model**: Build native apps with Angular 19+ signals, dependency injection, routing, and declarative templates.
- **Pure Native UI (No WebViews)**: Renders native platform widgets and GPU hardware-accelerated surfaces instead of wrapping HTML inside WebViews.
- **Rust Core & Layout**: View tree mutations, event loops, and Flexbox/Grid layout computations are executed in Rust via [Taffy](https://github.com/DioxusLabs/taffy).
- **TypeScript Compiler in Rust**: High-throughput TS/JS compilation and AST analysis implemented in Rust (using [OXC](https://github.com/oxc-project/oxc) / [SWC](https://swc.rs)).
- **Unified Multiplatform Targets**:
  - 📱 **Android**: High-speed JNI bridge to Android native Views or GPU Surface.
  - 🖥️ **Desktop**: Windows (Win32), macOS (AppKit), and Linux (Wayland/X11) via native Rust shell.
  - 🌐 **Web**: Zero-friction DOM adapter mapping native tags to HTML5/CSS flexbox.

---

## 📂 Repository Structure

```
angnative/
├── Cargo.toml                    # Rust workspace root
├── package.json                  # Node workspace root
├── crates/                       # Rust Crates
│   ├── angnative-core/           # View tree, diffing, mutation batching, event dispatch
│   ├── angnative-layout/         # Flexbox / Grid layout engine powered by Taffy
│   ├── angnative-compiler/       # Rust-based TypeScript compiler & AST transformer
│   ├── angnative-runtime/        # Embedded JS/TS runtime engine (QuickJS / Deno)
│   ├── angnative-desktop/        # Desktop shell for Windows, macOS, Linux (winit/wgpu)
│   └── angnative-android/        # Android NDK & JNI bridge
├── packages/                     # Angular / TypeScript Packages
│   ├── core/                     # Custom Angular Native Renderer & Element model
│   ├── common/                   # Built-in native components (<native-view>, <native-text>)
│   ├── web/                      # Web DOM fallback adapter
│   └── cli/                      # AngNative CLI (init, build, run)
├── platforms/                    # Native Platform Harnesses
│   ├── android/                  # Android Gradle build & JNI bindings
│   └── desktop/                  # Native desktop entry points
├── apps/
│   └── demo-app/                 # Example Angular Native Application
└── docs/
    ├── ARCHITECTURE.md           # In-depth architectural blueprint
    └── ROADMAP.md                # Multi-phase development roadmap
```

---

## 🛠️ Quick Start

```bash
# Clone and enter repo
cd angnative

# Build Rust native crates
cargo build --workspace

# Install JavaScript/Angular dependencies
pnpm install

# Run the demo app on Web
pnpm --filter @angnative/demo-app run start:web

# Run the demo app on Desktop
cargo run -p angnative-desktop
```

---

## 📖 Documentation
- [System Architecture](docs/ARCHITECTURE.md)
- [Development Roadmap](docs/ROADMAP.md)
