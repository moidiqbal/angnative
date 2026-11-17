/**
 * build.rs — Build script for angnative-host-ffi.
 *
 * Copies the C ABI header to the include directory so platform shells
 * (Android JNI, iOS Swift, Desktop) can find it.
 */

use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let include_dir = out_dir.join("include");
    fs::create_dir_all(&include_dir).expect("Failed to create include dir");

    // Copy the public header
    let header_src = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
        .join("include")
        .join("angnative_host_ffi.h");
    let header_dst = include_dir.join("angnative_host_ffi.h");

    if header_src.exists() {
        fs::copy(&header_src, &header_dst)
            .expect("Failed to copy angnative_host_ffi.h");
        println!("cargo:include={}", include_dir.display());
    }

    // Tell cargo to rerun if the header changes
    println!("cargo:rerun-if-changed=include/angnative_host_ffi.h");
}
