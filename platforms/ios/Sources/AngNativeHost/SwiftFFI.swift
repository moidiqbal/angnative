//
// SwiftFFI.swift — Swift wrapper around the angnative-host-ffi C ABI.
//
// All node ids are UInt64, matching `uint64_t` in
// crates/angnative-host-ffi/include/angnative_host_ffi.h.
// Every Rust-allocated string is freed with angnative_host_free_string.
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
