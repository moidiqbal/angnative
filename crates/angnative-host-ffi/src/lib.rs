/**
 * lib.rs — angnative-host-ffi: C ABI bridge for platform shells.
 *
 * This crate exposes a C ABI (`extern "C"`) that platform shells (Android
 * Kotlin/JNI, iOS Swift, Desktop) call into. No TypeScript lives here —
 * the bridge speaks JSON over FFI to the Rust host crates:
 *
 *   angnative-host-core  → command decode + mutation engine
 *   angnative-host-layout → Taffy layout
 *   angnative-host-view   → OpEmitter + ViewFactory SPI
 *
 * Platform shells register callback function pointers implementing the
 * ViewFactory trait, then drive the host via:
 *   - angnative_host_process_batch(JSON batch) → JSON ops
 *   - angnative_host_run_layout(nodes JSON, w, h) → JSON rects
 *
 * Modules:
 *   - bridge — Host struct, CallbackViewFactory, JSON request/response types
 *   - ffi    — extern "C" entry points (stable C ABI)
 */

pub mod bridge;
pub mod ffi;

pub use bridge::*;
