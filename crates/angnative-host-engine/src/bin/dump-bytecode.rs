//! Generate an AngNative versioned QuickJS module-bytecode artifact.
//!
//! Usage:
//!   cargo run -p angnative-host-engine --bin dump-bytecode -- <input.js> <output.qjsbc>
//!
//! The source is compiled as a module with QuickJS's COMPILE_ONLY flag, so
//! self-contained AOT IIFEs are not executed during generation. The resulting
//! artifact uses the same rquickjs version as the runtime loader.

use angnative_host_engine::bytecode::{compile_module_bytecode, encode, VIRTUAL_MODULE_NAME};
use rquickjs::{Context, Runtime};
use std::path::PathBuf;
use std::process;

fn fail(message: impl std::fmt::Display) -> ! {
    eprintln!("[angnative bytecode] error: {message}");
    process::exit(1);
}

fn main() {
    let mut args = std::env::args_os().skip(1);
    let input = PathBuf::from(
        args.next()
            .unwrap_or_else(|| fail("usage: dump-bytecode <input.js> <output.qjsbc>")),
    );
    let output = PathBuf::from(
        args.next()
            .unwrap_or_else(|| fail("usage: dump-bytecode <input.js> <output.qjsbc>")),
    );
    if args.next().is_some() {
        fail("unexpected extra argument");
    }

    let source = std::fs::read_to_string(&input)
        .unwrap_or_else(|e| fail(format!("cannot read {}: {e}", input.display())));
    let runtime = Runtime::new().unwrap_or_else(|e| fail(format!("QuickJS runtime: {e}")));
    let context = Context::full(&runtime)
        .unwrap_or_else(|e| fail(format!("QuickJS context: {e}")));
    let raw = context
        .with(|ctx| compile_module_bytecode(&ctx, VIRTUAL_MODULE_NAME, &source))
        .unwrap_or_else(|e| fail(format!("bytecode compilation failed: {e}")));
    let artifact = encode(&raw).unwrap_or_else(|e| fail(format!("bytecode framing failed: {e}")));

    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|e| fail(format!("cannot create {}: {e}", parent.display())));
    }
    std::fs::write(&output, &artifact)
        .unwrap_or_else(|e| fail(format!("cannot write {}: {e}", output.display())));
    println!(
        "[angnative bytecode] {} bytes from {} -> {}",
        artifact.len(),
        input.display(),
        output.display()
    );
}
