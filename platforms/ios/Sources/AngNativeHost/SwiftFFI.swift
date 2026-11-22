//
// SwiftFFI.swift — Swift wrapper around the angnative-host-ffi C ABI.
//
// All node ids are UInt64, matching `uint64_t` in
// crates/angnative-host-ffi/include/angnative_host_ffi.h.
// Runtime bridge handles are validated UInt64 values; 0 is invalid.
// Every Rust-allocated string is freed with the matching free function.
//
// This is static FFI wiring only. No iOS/macOS runtime claim is made here.
//

import Foundation

public typealias AngNativeHost = OpaquePointer

@_silgen_name("angnative_host_create")
public func angnative_host_create() -> AngNativeHost?

@_silgen_name("angnative_host_destroy")
public func angnative_host_destroy(_ handle: AngNativeHost?)

@_silgen_name("angnative_host_free_string")
public func angnative_host_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

@_silgen_name("angnative_host_last_error")
public func angnative_host_last_error(_ handle: AngNativeHost?) -> UnsafeMutablePointer<CChar>?

public typealias ANCreateViewCallback = @convention(c) (UInt64, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void
public typealias ANSetFrameCallback = @convention(c) (UInt64, Float, Float, Float, Float, UnsafeMutableRawPointer?) -> Void
public typealias ANSetPropsCallback = @convention(c) (UInt64, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void
public typealias ANSetTextCallback = @convention(c) (UInt64, UnsafePointer<CChar>?, UnsafeMutableRawPointer?) -> Void
public typealias ANAppendChildCallback = @convention(c) (UInt64, UInt64, UnsafeMutableRawPointer?) -> Void
public typealias ANInsertChildCallback = @convention(c) (UInt64, UInt64, UInt32, UnsafeMutableRawPointer?) -> Void
public typealias ANRemoveChildCallback = @convention(c) (UInt64, UInt64, UnsafeMutableRawPointer?) -> Void
public typealias ANDestroyViewCallback = @convention(c) (UInt64, UnsafeMutableRawPointer?) -> Void

@_silgen_name("angnative_host_register_view_factory")
public func angnative_host_register_view_factory(
    _ handle: AngNativeHost?,
    _ create_view: ANCreateViewCallback?,
    _ set_frame: ANSetFrameCallback?,
    _ set_props: ANSetPropsCallback?,
    _ set_text: ANSetTextCallback?,
    _ append_child: ANAppendChildCallback?,
    _ insert_child: ANInsertChildCallback?,
    _ remove_child: ANRemoveChildCallback?,
    _ destroy_view: ANDestroyViewCallback?,
    _ user_data: UnsafeMutableRawPointer?
) -> Int32

@_silgen_name("angnative_host_process_batch")
public func angnative_host_process_batch(
    _ handle: AngNativeHost?,
    _ batch_json: UnsafePointer<CChar>?,
    _ out_len: UnsafeMutablePointer<UInt32>?
) -> UnsafeMutablePointer<CChar>?

@_silgen_name("angnative_host_run_layout")
public func angnative_host_run_layout(
    _ handle: AngNativeHost?,
    _ request_json: UnsafePointer<CChar>?,
    _ out_len: UnsafeMutablePointer<UInt32>?
) -> UnsafeMutablePointer<CChar>?

@_silgen_name("angnative_host_node_count")
public func angnative_host_node_count(_ handle: AngNativeHost?) -> UInt32

@_silgen_name("angnative_host_snapshot")
public func angnative_host_snapshot(_ handle: AngNativeHost?) -> UnsafeMutablePointer<CChar>?

/// Swift-friendly wrapper around the Rust host.
public final class NativeHost {
    public let handle: AngNativeHost
    private var isDestroyed = false

    public init() throws {
        guard let h = angnative_host_create() else { throw HostError.createFailed }
        self.handle = h
    }

    deinit { destroy() }

    public func destroy() {
        guard !isDestroyed else { return }
        angnative_host_destroy(handle)
        isDestroyed = true
    }

    public var lastError: String? {
        guard let ptr = angnative_host_last_error(handle) else { return nil }
        defer { angnative_host_free_string(ptr) }
        return String(cString: ptr)
    }

    public func processBatch(_ json: String) throws -> String {
        guard let result = json.withCString({ angnative_host_process_batch(handle, $0, nil) }) else {
            throw HostError.operationFailed(lastError ?? "unknown error")
        }
        defer { angnative_host_free_string(result) }
        return String(cString: result)
    }

    public func runLayout(_ json: String) throws -> String {
        guard let result = json.withCString({ angnative_host_run_layout(handle, $0, nil) }) else {
            throw HostError.operationFailed(lastError ?? "unknown error")
        }
        defer { angnative_host_free_string(result) }
        return String(cString: result)
    }

    public var nodeCount: UInt32 { angnative_host_node_count(handle) }

    public func snapshot() throws -> String {
        guard let result = angnative_host_snapshot(handle) else {
            throw HostError.operationFailed(lastError ?? "unknown error")
        }
        defer { angnative_host_free_string(result) }
        return String(cString: result)
    }
}

public enum HostError: Error, LocalizedError {
    case createFailed
    case operationFailed(String)

    public var errorDescription: String? {
        switch self {
        case .createFailed: return "Failed to create AngNative host"
        case .operationFailed(let msg): return "Host operation failed: \(msg)"
        }
    }
}

// MARK: - Unified runtime bridge (Host + JS engine + Angular runtime)
//
// These symbols are implemented by `angnative-runtime-bridge`, not by
// `angnative-host-ffi`. Its staticlib already bundles the host FFI symbols, so
// iOS links only `libangnative_runtime_bridge.a` (see Package.swift and
// scripts/build-ios-native.sh).
//
// The bridge is thread-affine: create it, call it, and destroy it on the same
// (normally main/UI) thread. `destroy()` from another thread is a no-op in the
// C ABI rather than a panic; call it on the owner thread before releasing the
// Swift wrapper.

public typealias AngNativeRuntimeBridgeHandle = UInt64

@_silgen_name("angnative_runtime_bridge_create")
public func angnative_runtime_bridge_create() -> AngNativeRuntimeBridgeHandle

@_silgen_name("angnative_runtime_bridge_destroy")
public func angnative_runtime_bridge_destroy(_ handle: AngNativeRuntimeBridgeHandle)

@_silgen_name("angnative_runtime_bridge_dispatch_event")
public func angnative_runtime_bridge_dispatch_event(
    _ handle: AngNativeRuntimeBridgeHandle,
    _ nodeId: UInt64,
    _ eventName: UnsafePointer<CChar>?,
    _ payloadJson: UnsafePointer<CChar>?
) -> UInt32

@_silgen_name("angnative_runtime_bridge_load_script")
public func angnative_runtime_bridge_load_script(
    _ handle: AngNativeRuntimeBridgeHandle,
    _ source: UnsafePointer<CChar>?
) -> Int32

@_silgen_name("angnative_runtime_bridge_get_host")
public func angnative_runtime_bridge_get_host(
    _ handle: AngNativeRuntimeBridgeHandle
) -> AngNativeHost?

@_silgen_name("angnative_runtime_bridge_lifecycle_state")
public func angnative_runtime_bridge_lifecycle_state(
    _ handle: AngNativeRuntimeBridgeHandle
) -> Int32

@_silgen_name("angnative_runtime_bridge_suspend")
public func angnative_runtime_bridge_suspend(
    _ handle: AngNativeRuntimeBridgeHandle
) -> Int32

@_silgen_name("angnative_runtime_bridge_resume")
public func angnative_runtime_bridge_resume(
    _ handle: AngNativeRuntimeBridgeHandle
) -> Int32

@_silgen_name("angnative_runtime_bridge_stop")
public func angnative_runtime_bridge_stop(
    _ handle: AngNativeRuntimeBridgeHandle
) -> Int32

@_silgen_name("angnative_runtime_bridge_metrics_json")
public func angnative_runtime_bridge_metrics_json(
    _ handle: AngNativeRuntimeBridgeHandle
) -> UnsafeMutablePointer<CChar>?

@_silgen_name("angnative_runtime_bridge_last_error_json")
public func angnative_runtime_bridge_last_error_json(
    _ handle: AngNativeRuntimeBridgeHandle
) -> UnsafeMutablePointer<CChar>?

@_silgen_name("angnative_runtime_bridge_free_string")
public func angnative_runtime_bridge_free_string(_ ptr: UnsafeMutablePointer<CChar>?)

public typealias AngNativeErrorCallback = @convention(c) (
    UnsafePointer<CChar>?,
    UnsafePointer<CChar>?,
    UnsafeMutableRawPointer?
) -> Void

@_silgen_name("angnative_runtime_bridge_set_error_callback")
public func angnative_runtime_bridge_set_error_callback(
    _ handle: AngNativeRuntimeBridgeHandle,
    _ callback: AngNativeErrorCallback?,
    _ userData: UnsafeMutableRawPointer?
)

public enum RuntimeLifecycleState: Int32 {
    case invalid = -1
    case created = 0
    case initializing = 1
    case ready = 2
    case running = 3
    case suspended = 4
    case stopping = 5
    case destroyed = 6
    case failed = 7
}

/// Result of dispatching a native event using one delivery per event
/// (canonical JS thread-local path + legacy FFI fallback).
public struct RuntimeBridgeDispatchResult {
    public let listenersInvoked: UInt32

    /// `DISPATCH_PANIC` (`UInt32.max`) means the C ABI rejected a wrong-thread
    /// call before JS ran; it is never a real listener count.
    public var rejectedForWrongThread: Bool { listenersInvoked == UInt32.max }
}

public final class NativeRuntimeBridge {
    public let handle: AngNativeRuntimeBridgeHandle
    private var isDestroyed = false

    /// Create the bridge on the thread that will use it (normally the main
    /// thread). Do not create a second bridge in the same process.
    public init() throws {
        let h = angnative_runtime_bridge_create()
        guard h != 0 else { throw RuntimeBridgeError.createFailed }
        self.handle = h
    }

    deinit { destroy() }

    /// Borrow the Host owned by the bridge for view-factory registration and
    /// node inspection. Never wrap this in `NativeHost` or call
    /// `angnative_host_destroy` on it — the bridge owns the Host.
    public func hostHandle() throws -> AngNativeHost {
        guard let h = angnative_runtime_bridge_get_host(handle) else {
            throw RuntimeBridgeError.hostUnavailable
        }
        return h
    }

    /// Load a self-contained JS bundle (e.g. a generated Angular AOT bundle).
    /// Returns 0 on success, 1 on script error, 2 when called off the owner
    /// thread.
    @discardableResult
    public func loadScript(_ source: String) throws -> Int32 {
        let rc = source.withCString { angnative_runtime_bridge_load_script(handle, $0) }
        guard rc == 0 else {
            throw rc == 2 ? RuntimeBridgeError.wrongThread : RuntimeBridgeError.scriptFailed(rc)
        }
        return rc
    }

    /// Dispatch a native event using one delivery per event (canonical JS
    /// thread-local path + legacy FFI fallback).
    @discardableResult
    public func dispatchEvent(
        nodeId: UInt64,
        eventName: String,
        payloadJson: String? = nil
    ) -> RuntimeBridgeDispatchResult {
        eventName.withCString { name in
            if let payloadJson = payloadJson {
                return RuntimeBridgeDispatchResult(
                    listenersInvoked: payloadJson.withCString {
                        angnative_runtime_bridge_dispatch_event(handle, nodeId, name, $0)
                    }
                )
            }
            return RuntimeBridgeDispatchResult(
                listenersInvoked: angnative_runtime_bridge_dispatch_event(handle, nodeId, name, nil)
            )
        }
    }

    public var lifecycleState: RuntimeLifecycleState {
        RuntimeLifecycleState(rawValue: angnative_runtime_bridge_lifecycle_state(handle)) ?? .invalid
    }

    public func suspend() -> Int32 { angnative_runtime_bridge_suspend(handle) }
    public func resume() -> Int32 { angnative_runtime_bridge_resume(handle) }
    public func stop() -> Int32 { angnative_runtime_bridge_stop(handle) }

    public func metricsJson() -> String? {
        guard let ptr = angnative_runtime_bridge_metrics_json(handle) else { return nil }
        defer { angnative_runtime_bridge_free_string(ptr) }
        return String(cString: ptr)
    }

    public func lastErrorJson() -> String? {
        guard let ptr = angnative_runtime_bridge_last_error_json(handle) else { return nil }
        defer { angnative_runtime_bridge_free_string(ptr) }
        return String(cString: ptr)
    }

    public func setErrorCallback(
        _ callback: AngNativeErrorCallback?,
        userData: UnsafeMutableRawPointer? = nil
    ) {
        angnative_runtime_bridge_set_error_callback(handle, callback, userData)
    }

    /// Must be called on the creating thread. Calling from another thread is a
    /// safe no-op in the C ABI but leaks the bridge, so do not rely on it.
    public func destroy() {
        guard !isDestroyed else { return }
        angnative_runtime_bridge_destroy(handle)
        isDestroyed = true
    }
}

public enum RuntimeBridgeError: Error, LocalizedError {
    case createFailed
    case hostUnavailable
    case wrongThread
    case scriptFailed(Int32)

    public var errorDescription: String? {
        switch self {
        case .createFailed: return "Failed to create AngNative runtime bridge"
        case .hostUnavailable: return "Runtime bridge returned a NULL host handle"
        case .wrongThread: return "Runtime bridge called from a non-owner thread"
        case .scriptFailed(let rc): return "Loading the JS bundle failed (rc=\(rc))"
        }
    }
}
