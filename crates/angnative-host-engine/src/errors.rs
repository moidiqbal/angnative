/**
 * errors.rs — workspace-level error and diagnostics boundary (P1-11 / P3-04).
 *
 * Domain crates keep their own typed errors. This enum is the single stable
 * boundary used by the FFI/runtime bridge; `ffi_code()` values are part of the
 * C ABI and must not be renumbered.
 */
use crate::engine::EngineError;
use angnative_host_core::mutation::MutationError;
use serde::Serialize;
use std::time::{Duration, Instant};

/// Stable numeric code returned through FFI error accessors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ErrorCode {
    Ok = 0,
    InvalidArgument = 1,
    ScriptError = 2,
    Lifecycle = 3,
    InvalidHandle = 4,
    Serialization = 5,
    Panic = 6,
    Engine = 7,
    Mutation = 8,
    ViewFactory = 9,
    OpEmitter = 10,
    Crypto = 11,
    Internal = 12,
}

impl ErrorCode {
    pub fn as_i32(self) -> i32 {
        self as i32
    }
}

/// Single workspace-level error type at the runtime/FFI boundary.
#[derive(Debug, thiserror::Error)]
pub enum AngNativeError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("script error: {0}")]
    ScriptError(String),
    #[error("lifecycle error: {0}")]
    Lifecycle(String),
    #[error("invalid handle: {0}")]
    InvalidHandle(String),
    #[error("serialization error: {0}")]
    Serialization(String),
    #[error("panic caught at FFI boundary: {0}")]
    Panic(String),
    #[error("engine error: {0}")]
    Engine(String),
    #[error("mutation error: {0}")]
    Mutation(String),
    #[error("view factory error: {0}")]
    ViewFactory(String),
    #[error("op emitter error: {0}")]
    OpEmitter(String),
    #[error("crypto error: {0}")]
    Crypto(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl AngNativeError {
    pub fn ffi_code(&self) -> ErrorCode {
        match self {
            Self::InvalidArgument(_) => ErrorCode::InvalidArgument,
            Self::ScriptError(_) => ErrorCode::ScriptError,
            Self::Lifecycle(_) => ErrorCode::Lifecycle,
            Self::InvalidHandle(_) => ErrorCode::InvalidHandle,
            Self::Serialization(_) => ErrorCode::Serialization,
            Self::Panic(_) => ErrorCode::Panic,
            Self::Engine(_) => ErrorCode::Engine,
            Self::Mutation(_) => ErrorCode::Mutation,
            Self::ViewFactory(_) => ErrorCode::ViewFactory,
            Self::OpEmitter(_) => ErrorCode::OpEmitter,
            Self::Crypto(_) => ErrorCode::Crypto,
            Self::Internal(_) => ErrorCode::Internal,
        }
    }

    pub fn category(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) => "invalid_argument",
            Self::ScriptError(_) => "script_error",
            Self::Lifecycle(_) => "lifecycle",
            Self::InvalidHandle(_) => "invalid_handle",
            Self::Serialization(_) => "serialization",
            Self::Panic(_) => "panic",
            Self::Engine(_) => "engine",
            Self::Mutation(_) => "mutation",
            Self::ViewFactory(_) => "view_factory",
            Self::OpEmitter(_) => "op_emitter",
            Self::Crypto(_) => "crypto",
            Self::Internal(_) => "internal",
        }
    }

    pub fn from_domain(category: &'static str, error: impl std::fmt::Display) -> Self {
        match category {
            "view_factory" => Self::ViewFactory(error.to_string()),
            "op_emitter" => Self::OpEmitter(error.to_string()),
            "crypto" => Self::Crypto(error.to_string()),
            "serialization" => Self::Serialization(error.to_string()),
            "mutation" => Self::Mutation(error.to_string()),
            "lifecycle" => Self::Lifecycle(error.to_string()),
            _ => Self::Internal(error.to_string()),
        }
    }
}

impl From<EngineError> for AngNativeError {
    fn from(error: EngineError) -> Self {
        match error {
            EngineError::ScriptError(message) => Self::ScriptError(message),
            EngineError::NotInitialized
            | EngineError::CallError(_)
            | EngineError::ConversionError(_)
            | EngineError::Internal(_)
            | EngineError::Unsupported(_)
            | EngineError::Bytecode(_) => Self::Engine(error.to_string()),
        }
    }
}

impl From<MutationError> for AngNativeError {
    fn from(error: MutationError) -> Self {
        Self::Mutation(error.to_string())
    }
}

impl From<anyhow::Error> for AngNativeError {
    fn from(error: anyhow::Error) -> Self {
        Self::Internal(error.to_string())
    }
}

/// Structured error surfaced to Angular/native shells.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimeErrorInfo {
    pub code: i32,
    pub category: &'static str,
    pub phase: String,
    pub message: String,
    pub stack: Option<String>,
}

impl RuntimeErrorInfo {
    pub fn new(error: &AngNativeError, phase: &str, stack: Option<String>) -> Self {
        Self {
            code: error.ffi_code().as_i32(),
            category: error.category(),
            phase: phase.to_string(),
            message: error.to_string(),
            stack,
        }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_else(|_| {
            r#"{"code":12,"category":"internal","phase":"serialize","message":"serialization failed","stack":null}"#.to_string()
        })
    }
}

/// Runtime counters collected by the authoritative bridge (P1-11).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RuntimeMetrics {
    pub engine_name: &'static str,
    pub heap_bytes: Option<usize>,
    pub event_count: u64,
    pub last_event_latency_ns: u64,
    pub total_event_latency_ns: u64,
    pub script_load_count: u64,
    pub script_load_ms: u64,
    pub reported_error_count: u64,
}

impl RuntimeMetrics {
    pub fn new(engine_name: &'static str, heap_bytes: Option<usize>) -> Self {
        Self {
            engine_name,
            heap_bytes,
            event_count: 0,
            last_event_latency_ns: 0,
            total_event_latency_ns: 0,
            script_load_count: 0,
            script_load_ms: 0,
            reported_error_count: 0,
        }
    }

    pub fn refresh_heap(&mut self, heap_bytes: Option<usize>) {
        self.heap_bytes = heap_bytes;
    }

    pub fn record_event(&mut self, started: Instant) {
        let elapsed = started.elapsed();
        self.event_count += 1;
        self.last_event_latency_ns = elapsed.as_nanos() as u64;
        self.total_event_latency_ns += self.last_event_latency_ns;
    }

    pub fn record_script_load(&mut self, started: Instant) {
        self.script_load_count += 1;
        self.script_load_ms += started.elapsed().as_millis() as u64;
    }

    pub fn record_error(&mut self) {
        self.reported_error_count += 1;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self)
            .unwrap_or_else(|_| r#"{"engine_name":"unknown"}"#.to_string())
    }
}

/// Small helper used by callers that need a duration snapshot.
pub fn elapsed_ms(started: Instant) -> u64 {
    started.elapsed().as_millis() as u64
}

/// Kept for call sites that want an explicit zero-duration sentinel.
pub fn zero_duration() -> Duration {
    Duration::ZERO
}
