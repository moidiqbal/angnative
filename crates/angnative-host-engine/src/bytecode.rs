//! Versioned container for rquickjs module-bytecode payloads.
//!
//! Layout (v2):
//!   `ANGNQJSB2` (9 bytes) | u32 LE payload length | u32 LE fingerprint length |
//!   UTF-8 fingerprint | raw module bytecode
//!
//! The fingerprint binds the artifact to the producer's rquickjs/QuickJS
//! version and target triple, so a `.qjsbc` from another build is rejected
//! before the unsafe `Module::load` call with an actionable error. This is a
//! compatibility check, not a security boundary: bytecode remains trusted
//! build output.

use crate::engine::EngineError;
use rquickjs::{qjs, Ctx};
use std::ffi::{CStr, CString};

/// Magic bytes identifying an AngNative v2 bytecode artifact.
pub const MAGIC: &[u8; 9] = b"ANGNQJSB2";
/// Magic bytes of the superseded v1 container (no fingerprint field).
pub const LEGACY_MAGIC: &[u8; 9] = b"ANGNQJSB1";
/// Size of each little-endian `u32` length field.
pub const LENGTH_FIELD_LEN: usize = 4;
/// Fixed header size: magic (9) + payload length (4) + fingerprint length (4).
pub const HEADER_LEN: usize = 9 + LENGTH_FIELD_LEN + LENGTH_FIELD_LEN;
/// Maximum accepted bytecode payload. AOT bundles are far below this bound.
pub const MAX_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
/// Maximum accepted UTF-8 fingerprint length (guards absurd declared sizes).
pub const MAX_FINGERPRINT_BYTES: usize = 256;
/// Stable virtual module name used when dumping self-contained AOT IIFEs.
pub const VIRTUAL_MODULE_NAME: &str = "angnative-bytecode-module.js";
/// Fingerprint format tag; an artifact with another prefix is not recognized.
pub const FINGERPRINT_FORMAT: &str = "qjsbc-v2";
/// rquickjs crate version this container format is pinned to.
///
/// rquickjs does not expose its own crate version through a stable API, so this
/// is a documented compile-time constant. Bump it together with the workspace
/// `Cargo.toml` / `Cargo.lock` dependency and regenerate all `.qjsbc` files.
pub const RQUICKJS_VERSION: &str = "0.14.0";

fn bytecode_error(message: impl Into<String>) -> EngineError {
    EngineError::Bytecode(message.into())
}

/// Deterministic producer fingerprint for the running engine and target.
///
/// Sources: `qjs::JS_GetVersion()` (QuickJS-ng version string compiled into
/// the linked C library), the pinned rquickjs crate version above, and Rust's
/// stable target constants. Both generator and loader call this function, so
/// they agree without a hand-maintained artifact header.
pub fn runtime_fingerprint() -> String {
    // SAFETY: vendored QuickJS's `JS_GetVersion()` returns a pointer to a
    // process-lifetime NUL-terminated literal and needs no runtime or context.
    let quickjs_version = unsafe { CStr::from_ptr(qjs::JS_GetVersion()) }.to_string_lossy();
    format!(
        "{FINGERPRINT_FORMAT}/rquickjs-{RQUICKJS_VERSION}/quickjs-ng-{quickjs_version}/{arch}-{os}-{endian}-{pointer_bits}",
        arch = std::env::consts::ARCH,
        os = std::env::consts::OS,
        endian = if cfg!(target_endian = "little") { "le" } else { "be" },
        pointer_bits = usize::BITS,
    )
}

/// Wrap a validated bytecode payload in the v2 container with this runtime's fingerprint.
pub fn encode(payload: &[u8]) -> Result<Vec<u8>, EngineError> {
    encode_with_fingerprint(payload, &runtime_fingerprint())
}

fn encode_with_fingerprint(payload: &[u8], fingerprint: &str) -> Result<Vec<u8>, EngineError> {
    if payload.is_empty() {
        return Err(bytecode_error("payload is empty"));
    }
    if payload.len() > MAX_PAYLOAD_BYTES {
        return Err(bytecode_error(format!(
            "payload exceeds {MAX_PAYLOAD_BYTES} byte limit"
        )));
    }
    if fingerprint.is_empty() {
        return Err(bytecode_error("fingerprint is missing"));
    }
    let length = u32::try_from(payload.len())
        .map_err(|_| bytecode_error("payload length exceeds u32"))?;
    let fingerprint_length = u32::try_from(fingerprint.len())
        .map_err(|_| bytecode_error("fingerprint length exceeds u32"))?;
    let mut artifact = Vec::with_capacity(HEADER_LEN + fingerprint.len() + payload.len());
    artifact.extend_from_slice(MAGIC);
    artifact.extend_from_slice(&length.to_le_bytes());
    artifact.extend_from_slice(&fingerprint_length.to_le_bytes());
    artifact.extend_from_slice(fingerprint.as_bytes());
    artifact.extend_from_slice(payload);
    Ok(artifact)
}

/// Validate the container, including the producer fingerprint, and return the raw payload.
pub fn decode(bytes: &[u8]) -> Result<&[u8], EngineError> {
    if bytes.len() < HEADER_LEN {
        return Err(bytecode_error(format!(
            "artifact is shorter than {HEADER_LEN} byte header"
        )));
    }
    if &bytes[..9] != MAGIC {
        if &bytes[..9] == LEGACY_MAGIC {
            return Err(bytecode_error(
                "magic mismatch: legacy ANGNQJSB1 container has no fingerprint; regenerate it with `pnpm run bundle:bytecode`",
            ));
        }
        return Err(bytecode_error("magic mismatch"));
    }
    let declared = u32::from_le_bytes(bytes[9..13].try_into().unwrap()) as usize;
    let fingerprint_declared = u32::from_le_bytes(bytes[13..17].try_into().unwrap()) as usize;
    if declared == 0 {
        return Err(bytecode_error("payload is empty"));
    }
    if declared > MAX_PAYLOAD_BYTES {
        return Err(bytecode_error(format!(
            "declared length exceeds {MAX_PAYLOAD_BYTES} byte limit"
        )));
    }
    if fingerprint_declared == 0 {
        return Err(bytecode_error("fingerprint is missing"));
    }
    if fingerprint_declared > MAX_FINGERPRINT_BYTES {
        return Err(bytecode_error(format!(
            "declared fingerprint length exceeds {MAX_FINGERPRINT_BYTES} byte limit"
        )));
    }
    let Some(total) = HEADER_LEN
        .checked_add(fingerprint_declared)
        .and_then(|n| n.checked_add(declared))
    else {
        return Err(bytecode_error("declared lengths overflow"));
    };
    if bytes.len() != total {
        return Err(bytecode_error(format!(
            "length mismatch: header declares {fingerprint_declared} fingerprint bytes and {declared} payload bytes, artifact has {} bytes after magic",
            bytes.len() - HEADER_LEN
        )));
    }
    let stored = std::str::from_utf8(&bytes[HEADER_LEN..HEADER_LEN + fingerprint_declared])
        .map_err(|_| bytecode_error("fingerprint is not valid UTF-8"))?;
    if !stored.starts_with(FINGERPRINT_FORMAT) {
        return Err(bytecode_error(format!(
            "unknown fingerprint format: {stored}"
        )));
    }
    let expected = runtime_fingerprint();
    if stored != expected {
        return Err(bytecode_error(format!(
            "fingerprint mismatch: artifact was produced by \"{stored}\", but this runtime expects \"{expected}\""
        )));
    }
    Ok(&bytes[HEADER_LEN + fingerprint_declared..])
}

fn exception_detail(ctx: &Ctx<'_>) -> Option<String> {
    if ctx.has_exception() {
        Some(format!("{:?}", ctx.catch()))
    } else {
        None
    }
}

/// Compile source into module bytecode **without executing it**.
///
/// This uses QuickJS's low-level compile-only eval because rquickjs only
/// exposes `Module::write` for already-evaluated modules. AOT bundles must not
/// be executed during artifact generation: they expect the host-side `na_*`
/// globals installed by `AngNativeRuntime`, not a bare generator context.
/// The generated bytecode is little-endian and strips source/debug metadata.
pub fn compile_module_bytecode(
    ctx: &Ctx<'_>,
    name: &str,
    source: &str,
) -> Result<Vec<u8>, EngineError> {
    let filename = CString::new(name)
        .map_err(|_| bytecode_error("module name contains an interior NUL byte"))?;
    // Mirror rquickjs `eval_raw`: QuickJS expects a NUL-terminated input
    // buffer even when an explicit length is supplied.
    let source_c = CString::new(source.as_bytes())
        .map_err(|_| bytecode_error("source contains an interior NUL byte"))?;
    let source_len = source.len() as qjs::size_t;
    let eval_flags = (qjs::JS_EVAL_TYPE_MODULE
        | qjs::JS_EVAL_FLAG_STRICT
        | qjs::JS_EVAL_FLAG_COMPILE_ONLY) as i32;

    // SAFETY: `JS_Eval` receives a NUL-terminated source buffer and filename;
    // COMPILE_ONLY ensures no source top-level code is executed.
    let module_value = unsafe {
        qjs::JS_Eval(
            ctx.as_raw().as_ptr(),
            source_c.as_ptr(),
            source_len,
            filename.as_ptr(),
            eval_flags,
        )
    };
    let tag = unsafe { qjs::JS_VALUE_GET_TAG(module_value) };

    let result = (|| {
        if tag == qjs::JS_TAG_EXCEPTION as i32 {
            return Err(bytecode_error(format!(
                "QuickJS module compilation failed: {}",
                exception_detail(ctx).unwrap_or_else(|| "unknown exception".to_string())
            )));
        }
        if tag != qjs::JS_TAG_MODULE as i32 {
            return Err(bytecode_error(format!(
                "QuickJS did not produce a module value (tag {tag})"
            )));
        }

        let module_ptr = unsafe { qjs::JS_VALUE_GET_PTR(module_value) };
        if module_ptr.is_null() {
            return Err(bytecode_error("QuickJS returned a null module pointer"));
        }

        let mut write_flags = qjs::JS_WRITE_OBJ_BYTECODE
            | qjs::JS_WRITE_OBJ_STRIP_SOURCE
            | qjs::JS_WRITE_OBJ_STRIP_DEBUG;
        if cfg!(target_endian = "big") {
            write_flags |= qjs::JS_WRITE_OBJ_BSWAP;
        }

        let mut length: qjs::size_t = 0;
        // SAFETY: `module_value` is a live module value from this context, and
        // `length` is a valid out-parameter. The returned buffer is owned by
        // QuickJS and freed after copying.
        let buffer = unsafe {
            qjs::JS_WriteObject(
                ctx.as_raw().as_ptr(),
                &mut length,
                qjs::JS_MKPTR(qjs::JS_TAG_MODULE as i32, module_ptr),
                write_flags as i32,
            )
        };
        if buffer.is_null() {
            return Err(bytecode_error(format!(
                "QuickJS bytecode write failed: {}",
                exception_detail(ctx).unwrap_or_else(|| "unknown exception".to_string())
            )));
        }
        let bytes = unsafe { std::slice::from_raw_parts(buffer, length as usize) }.to_vec();
        unsafe { qjs::js_free(ctx.as_raw().as_ptr(), buffer.cast()) };
        Ok(bytes)
    })();

    // `JS_EXCEPTION` carries no reference count; module values do.
    if tag != qjs::JS_TAG_EXCEPTION as i32 {
        unsafe { qjs::JS_FreeValue(ctx.as_raw().as_ptr(), module_value) };
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use rquickjs::{Context, Runtime};

    /// Build a container byte-for-byte, independent of `encode`, so decoder
    /// rejection tests exercise malformed headers rather than encoder behavior.
    fn artifact(fingerprint: &[u8], payload: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&(fingerprint.len() as u32).to_le_bytes());
        bytes.extend_from_slice(fingerprint);
        bytes.extend_from_slice(payload);
        bytes
    }

    fn valid_fingerprint() -> Vec<u8> {
        runtime_fingerprint().into_bytes()
    }

    #[test]
    fn encode_decode_round_trip() {
        let payload = b"raw-bytecode";
        let artifact = encode(payload).unwrap();
        assert_eq!(&artifact[..9], MAGIC);
        assert_eq!(&artifact[9..13], &(payload.len() as u32).to_le_bytes());
        assert_eq!(
            &artifact[13..17],
            &(valid_fingerprint().len() as u32).to_le_bytes()
        );
        assert_eq!(
            &artifact[HEADER_LEN..HEADER_LEN + valid_fingerprint().len()],
            valid_fingerprint()
        );
        assert_eq!(artifact.len(), HEADER_LEN + valid_fingerprint().len() + payload.len());
        assert_eq!(decode(&artifact).unwrap(), payload);
    }

    #[test]
    fn runtime_fingerprint_is_deterministic_and_descriptive() {
        let fingerprint = runtime_fingerprint();
        assert_eq!(fingerprint, runtime_fingerprint());
        assert!(fingerprint.starts_with(FINGERPRINT_FORMAT), "{fingerprint}");
        assert!(fingerprint.contains(&format!("rquickjs-{RQUICKJS_VERSION}")), "{fingerprint}");
        assert!(fingerprint.contains("quickjs-ng-"), "{fingerprint}");
        assert!(fingerprint.contains(std::env::consts::ARCH), "{fingerprint}");
        assert!(fingerprint.contains(std::env::consts::OS), "{fingerprint}");
        assert!(fingerprint.len() <= MAX_FINGERPRINT_BYTES);
    }

    #[test]
    fn decode_rejects_truncated_header() {
        for len in 0..HEADER_LEN {
            let bytes = vec![0u8; len];
            let err = decode(&bytes).unwrap_err();
            assert!(matches!(err, EngineError::Bytecode(_)));
            assert!(err.to_string().contains("short"), "{len}: {err}");
        }
        // A full header with a short fingerprint payload is also truncated.
        let mut bytes = artifact(&valid_fingerprint(), b"payload");
        bytes.truncate(HEADER_LEN + 2);
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("length mismatch"), "{err}");
    }

    #[test]
    fn decode_rejects_wrong_magic() {
        let mut bytes = artifact(&valid_fingerprint(), b"payload");
        bytes[0] = b'X';
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("magic"), "{err}");
    }

    #[test]
    fn decode_rejects_legacy_magic_with_migration_message() {
        let mut bytes = artifact(&valid_fingerprint(), b"payload");
        bytes[8] = b'1';
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("magic mismatch"), "{err}");
        assert!(err.to_string().contains("legacy ANGNQJSB1"), "{err}");
        assert!(err.to_string().contains("bundle:bytecode"), "{err}");
    }

    #[test]
    fn decode_rejects_length_mismatch() {
        let mut bytes = artifact(&valid_fingerprint(), b"payload");
        bytes[12] += 1;
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("length mismatch"), "{err}");
        let mut bytes = artifact(&valid_fingerprint(), b"payload");
        bytes[13] += 1;
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("length mismatch"), "{err}");
    }

    #[test]
    fn decode_rejects_empty_payload() {
        let bytes = artifact(&valid_fingerprint(), b"");
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("empty"), "{err}");
    }

    #[test]
    fn decode_rejects_oversized_declared_length() {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend_from_slice(&((MAX_PAYLOAD_BYTES + 1) as u32).to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("limit"), "{err}");
    }

    #[test]
    fn decode_rejects_oversized_declared_fingerprint_length() {
        let mut bytes = Vec::from(MAGIC);
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&((MAX_FINGERPRINT_BYTES + 1) as u32).to_le_bytes());
        let err = decode(&bytes).unwrap_err();
        assert!(err.to_string().contains("fingerprint"), "{err}");
        assert!(err.to_string().contains("limit"), "{err}");
    }

    #[test]
    fn decode_rejects_wrong_fingerprint() {
        let wrong = runtime_fingerprint().replace(RQUICKJS_VERSION, "0.0.0");
        assert_ne!(wrong, runtime_fingerprint());
        let err = decode(&artifact(wrong.as_bytes(), b"payload")).unwrap_err();
        assert!(err.to_string().contains("fingerprint mismatch"), "{err}");
        assert!(err.to_string().contains("0.0.0"), "{err}");
    }

    #[test]
    fn decode_rejects_missing_fingerprint() {
        let err = decode(&artifact(b"", b"payload")).unwrap_err();
        assert!(err.to_string().contains("fingerprint is missing"), "{err}");
        assert_eq!(
            encode_with_fingerprint(b"payload", "").unwrap_err().to_string(),
            err.to_string()
        );
    }

    #[test]
    fn decode_rejects_unknown_fingerprint_format() {
        let err = decode(&artifact(b"some-other-format/1.0", b"payload")).unwrap_err();
        assert!(err.to_string().contains("unknown fingerprint format"), "{err}");
    }

    #[test]
    fn decode_rejects_non_utf8_fingerprint() {
        let err = decode(&artifact(&[0xff, 0xfe], b"payload")).unwrap_err();
        assert!(err.to_string().contains("not valid UTF-8"), "{err}");
    }

    #[test]
    fn encode_rejects_empty_payload() {
        let err = encode(&[]).unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[test]
    fn compile_module_bytecode_is_compile_only() {
        let runtime = Runtime::new().unwrap();
        let context = Context::full(&runtime).unwrap();
        let bytes = context
            .with(|ctx| {
                compile_module_bytecode(
                    &ctx,
                    VIRTUAL_MODULE_NAME,
                    "globalThis.__ranAtDump = true;",
                )
            })
            .unwrap();
        assert!(!bytes.is_empty());
        let ran = context
            .with(|ctx| ctx.globals().get::<_, bool>("__ranAtDump").unwrap_or(false));
        assert!(!ran, "generator must not execute AOT source");
    }
}
